//! Le serveur MCP lui-même : transport HTTP sur la boucle locale.
//!
//! Pourquoi l'application héberge plutôt que d'être hébergée : un binaire MCP
//! lancé par le client IA est **un autre process**. Il ne voit pas les enfants
//! de Lynk Dev, et devrait donc gérer ses propres process — soit un second
//! superviseur, soit deux vérités concurrentes sur qui tourne. En exposant
//! depuis l'application, le superviseur reste unique.
//!
//! Trois garde-fous, tous nécessaires :
//!
//! 1. **Écoute strictement sur `127.0.0.1`.** Jamais `0.0.0.0` : ce serveur
//!    démarre et arrête des process, il n'a rien à faire sur le réseau.
//! 2. **Jeton obligatoire** sur chaque requête (`Authorization: Bearer`).
//! 3. **Contrôle de l'en-tête `Origin`.** Sans lui, n'importe quelle page web
//!    ouverte dans le navigateur peut poster vers `http://127.0.0.1:<port>` —
//!    c'est le détournement par réattribution DNS, et la spécification MCP
//!    l'exige explicitement pour un serveur local.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use tokio::sync::{oneshot, Mutex};

use super::journal::{CallRecord, Journal};
use super::protocol::{self, Request};
use super::tools::{self, ToolContext};

/// Chemin unique du transport « Streamable HTTP ».
pub const PATH: &str = "/mcp";

struct Shared {
    ctx: ToolContext,
    journal: Arc<Journal>,
    token: String,
    version: String,
}

struct Running {
    port: u16,
    shutdown: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

/// Le serveur, et son état de marche.
pub struct McpServer {
    ctx: ToolContext,
    journal: Arc<Journal>,
    version: String,
    running: Mutex<Option<Running>>,
}

impl McpServer {
    pub fn new(ctx: ToolContext, journal: Arc<Journal>, version: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            ctx,
            journal,
            version: version.into(),
            running: Mutex::new(None),
        })
    }

    pub fn journal(&self) -> &Arc<Journal> {
        &self.journal
    }

    /// Port d'écoute, si le serveur tourne.
    pub async fn port(&self) -> Option<u16> {
        self.running.lock().await.as_ref().map(|run| run.port)
    }

    /// Démarre l'écoute. Un serveur déjà en marche est d'abord arrêté : c'est
    /// ce qui rend un changement de port ou de jeton simplement idempotent.
    pub async fn start(&self, port: u16, token: String) -> anyhow::Result<u16> {
        self.stop().await;

        let shared = Arc::new(Shared {
            ctx: self.ctx.clone(),
            journal: Arc::clone(&self.journal),
            token,
            version: self.version.clone(),
        });

        let app = Router::new()
            .route(PATH, post(handle).get(no_server_stream))
            .with_state(shared);

        // `127.0.0.1` et rien d'autre.
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(|err| anyhow::anyhow!("écoute sur 127.0.0.1:{port} impossible — {err}"))?;
        let bound = listener.local_addr()?.port();

        let (shutdown, wait) = oneshot::channel();
        let task = tokio::spawn(async move {
            let served = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = wait.await;
                })
                .await;
            if let Err(err) = served {
                tracing::error!("serveur MCP interrompu : {err}");
            }
        });

        tracing::info!("serveur MCP en écoute sur http://127.0.0.1:{bound}{PATH}");
        *self.running.lock().await = Some(Running {
            port: bound,
            shutdown,
            task,
        });
        Ok(bound)
    }

    /// Arrête l'écoute et **attend** que la tâche soit sortie : sans cette
    /// attente, un redémarrage immédiat sur le même port échouerait en
    /// « adresse déjà utilisée ».
    pub async fn stop(&self) {
        let Some(run) = self.running.lock().await.take() else {
            return;
        };
        let _ = run.shutdown.send(());
        let _ = run.task.await;
        tracing::info!("serveur MCP arrêté (port {})", run.port);
    }
}

// ── Garde-fous ───────────────────────────────────────────────────────────

/// Comparaison à durée constante : une comparaison qui s'arrête au premier
/// octet différent laisse deviner le jeton, un caractère à la fois.
fn token_matches(expected: &str, given: &str) -> bool {
    let (expected, given) = (expected.as_bytes(), given.as_bytes());
    if expected.len() != given.len() {
        return false;
    }
    expected
        .iter()
        .zip(given)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

/// Une origine **absente** est acceptée : c'est le cas d'un client natif, qui
/// n'en envoie pas. Une origine **présente** doit être locale — un navigateur
/// en envoie toujours une, et c'est de lui qu'on se protège.
fn origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return true;
    };
    ["http://127.0.0.1", "http://localhost", "http://[::1]"]
        .iter()
        .any(|prefix| origin == *prefix || origin.starts_with(&format!("{prefix}:")))
}

// ── Transport ────────────────────────────────────────────────────────────

/// Le flux serveur → client (`GET`) n'est pas nécessaire ici : aucun outil
/// n'émet spontanément. La spécification autorise à le refuser.
async fn no_server_stream() -> Response {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [(header::ALLOW, "POST")],
        "ce serveur ne diffuse pas d'événements — utilisez POST",
    )
        .into_response()
}

async fn handle(State(shared): State<Arc<Shared>>, headers: HeaderMap, body: String) -> Response {
    if !origin_allowed(&headers) {
        return (StatusCode::FORBIDDEN, "origine refusée").into_response();
    }
    if !bearer(&headers).is_some_and(|given| token_matches(&shared.token, given)) {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
            "jeton absent ou invalide",
        )
            .into_response();
    }

    let payload: Value = match serde_json::from_str(&body) {
        Ok(value) => value,
        Err(err) => {
            return Json(protocol::failure(
                None,
                protocol::PARSE_ERROR,
                format!("JSON illisible : {err}"),
            ))
            .into_response()
        }
    };

    // Le lot de messages a été retiré du protocole en 2025-06-18. Le dire vaut
    // mieux que de traiter le premier élément en silence.
    if payload.is_array() {
        return Json(protocol::failure(
            None,
            protocol::INVALID_REQUEST,
            "les lots de messages ne sont pas acceptés — un message par requête",
        ))
        .into_response();
    }

    let request: Request = match serde_json::from_value(payload) {
        Ok(request) => request,
        Err(err) => {
            return Json(protocol::failure(
                None,
                protocol::INVALID_REQUEST,
                format!("message JSON-RPC invalide : {err}"),
            ))
            .into_response()
        }
    };

    let is_notification = request.is_notification();
    let response = dispatch(&shared, request).await;

    match (is_notification, response) {
        // Une notification n'attend rien : répondre serait une faute de
        // protocole, et certains clients s'en offusquent.
        (true, _) => StatusCode::ACCEPTED.into_response(),
        (false, Some(value)) => Json(value).into_response(),
        (false, None) => StatusCode::ACCEPTED.into_response(),
    }
}

async fn dispatch(shared: &Shared, request: Request) -> Option<Value> {
    let id = request.id.clone();

    match request.method.as_str() {
        "initialize" => Some(protocol::success(
            id,
            protocol::initialize_result(
                request
                    .params
                    .get("protocolVersion")
                    .and_then(Value::as_str),
                &shared.version,
            ),
        )),
        "ping" => Some(protocol::success(id, json!({}))),
        "tools/list" => Some(protocol::success(id, tools::catalogue())),
        "tools/call" => Some(call_tool(shared, id, &request.params).await),
        // Les notifications du client (`notifications/initialized`, annulations)
        // n'appellent aucune réponse.
        method if method.starts_with("notifications/") => None,
        other => Some(protocol::failure(
            id,
            protocol::METHOD_NOT_FOUND,
            format!("méthode « {other} » non gérée"),
        )),
    }
}

async fn call_tool(shared: &Shared, id: Option<Value>, params: &Value) -> Value {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return protocol::failure(
            id,
            protocol::INVALID_PARAMS,
            "nom d'outil manquant dans l'appel",
        );
    };
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let started = Instant::now();
    let outcome = tools::call(&shared.ctx, name, &arguments).await;
    let elapsed = started.elapsed().as_millis() as u64;

    shared.journal.record(CallRecord::new(
        name,
        outcome.target.clone(),
        !outcome.is_error,
        &outcome.text,
        elapsed,
    ));

    protocol::success(id, protocol::tool_result(outcome.text, outcome.is_error))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                header::HeaderName::from_bytes(name.as_bytes()).expect("nom"),
                value.parse().expect("valeur"),
            );
        }
        map
    }

    #[test]
    fn the_token_comparison_rejects_a_different_length() {
        assert!(token_matches("abcdef", "abcdef"));
        assert!(!token_matches("abcdef", "abcde"));
        assert!(!token_matches("abcdef", "abcdeg"));
        assert!(!token_matches("", "x"));
    }

    #[test]
    fn the_bearer_prefix_is_required() {
        assert_eq!(
            bearer(&headers(&[("authorization", "Bearer secret")])),
            Some("secret")
        );
        // Un jeton nu, sans schéma, n'est pas accepté.
        assert_eq!(bearer(&headers(&[("authorization", "secret")])), None);
        assert_eq!(bearer(&HeaderMap::new()), None);
    }

    /// Un client natif n'envoie pas d'origine : le refuser fermerait la porte
    /// au cas d'usage principal.
    #[test]
    fn a_missing_origin_is_allowed() {
        assert!(origin_allowed(&HeaderMap::new()));
    }

    /// Le cas qui motive le contrôle : une page web quelconque qui poste vers
    /// la boucle locale.
    #[test]
    fn a_remote_origin_is_refused() {
        assert!(!origin_allowed(&headers(&[(
            "origin",
            "https://exemple.test"
        )])));
        assert!(!origin_allowed(&headers(&[(
            "origin",
            "http://127.0.0.1.exemple.test"
        )])));
        assert!(!origin_allowed(&headers(&[(
            "origin",
            "http://localhost.exemple.test"
        )])));
    }

    #[test]
    fn a_local_origin_is_allowed() {
        for origin in [
            "http://127.0.0.1:52780",
            "http://localhost:3000",
            "http://[::1]:52780",
            "http://localhost",
        ] {
            assert!(origin_allowed(&headers(&[("origin", origin)])), "{origin}");
        }
    }
}
