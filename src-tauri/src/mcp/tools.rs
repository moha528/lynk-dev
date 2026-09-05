//! Le catalogue d'outils MCP et leur exécution.
//!
//! ⚠️ **Aucune logique métier ici.** Chaque outil lit le profil actif, appelle
//! le superviseur, met en forme la réponse. C'est la même règle que pour le
//! pont IPC (`commands/dev.rs`) : deux façades, un seul superviseur. Une règle
//! glissée à ce niveau ne vaudrait que pour l'IA, et divergerait de l'écran.
//!
//! ⚠️ **Il n'y a pas — et il ne doit pas y avoir — d'outil d'exécution de
//! commande arbitraire.** Un `run_command` transformerait ce serveur en shell
//! distant : le jeton ne protégerait plus une supervision, il ouvrirait la
//! machine. Les seules commandes lancées sont celles que l'utilisateur a
//! lui-même écrites dans son profil.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::task::JoinSet;

use crate::dev::logs::LogStore;
use crate::dev::net;
use crate::dev::types::{DevProfile, LogStream, ServiceConfig};
use crate::dev::{StartOptions, Supervisor};
use crate::store::{dev_profiles, settings, DbPool};

/// Clé du profil retenu par la fenêtre (`useSettingsStore`). C'est le seul lien
/// entre l'écran et le MCP : **on pilote ce que l'utilisateur a sous les yeux**.
const ACTIVE_PROFILE_KEY: &str = "dev_profile_id";

const DEFAULT_LOG_LINES: usize = 100;
const MAX_LOG_LINES: usize = 1_000;

/// Sonde d'existence d'un service non géré par nous. Courte : elle est jouée
/// pour chaque service du profil à chaque `list_services`.
const EXTERNAL_PROBE: Duration = Duration::from_millis(400);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(3);

/// Tout ce dont un outil a besoin. `Clone` est bon marché : le pool SQLite et
/// les deux `Arc` partagent leur état, ils ne le dupliquent pas.
#[derive(Clone)]
pub struct ToolContext {
    pub pool: DbPool,
    pub supervisor: Arc<Supervisor>,
    pub logs: Arc<LogStore>,
}

/// Ce qu'un outil rend : un texte pour le modèle, et de quoi tenir le journal.
pub struct Outcome {
    pub text: String,
    pub is_error: bool,
    /// Service visé, quand il y en a un — c'est ce que le journal affiche.
    pub target: Option<String>,
}

impl Outcome {
    fn ok(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: false,
            target: None,
        }
    }

    fn failed(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: true,
            target: None,
        }
    }

    fn about(mut self, service_id: impl AsRef<str>) -> Self {
        self.target = Some(service_id.as_ref().to_string());
        self
    }
}

fn json_text(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

// ── Catalogue ────────────────────────────────────────────────────────────

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        },
    })
}

fn service_id_property() -> Value {
    json!({
        "service_id": {
            "type": "string",
            "description": "Identifiant du service, tel que rendu par list_services.",
        }
    })
}

/// Les huit outils, sous la forme attendue par `tools/list`.
pub fn catalogue() -> Value {
    json!({
        "tools": [
            tool(
                "list_services",
                "Liste les services du profil actif du Dev Manager avec leur état : \
    statut, PID, port, et depuis combien de temps ils tournent. \
    Un service « external » écoute son port mais n'a pas été lancé par Lynk Dev.",
                json!({}),
                &[],
            ),
            tool(
                "get_service_logs",
                "Rend les dernières lignes de sortie d'un service, capturées depuis le \
    démarrage de Lynk Dev. Un service lancé hors de l'application n'a pas de lignes ici.",
                json!({
                    "service_id": service_id_property()["service_id"],
                    "lines": {
                        "type": "integer",
                        "description": "Nombre de lignes à rendre (défaut 100, maximum 1000).",
                        "minimum": 1,
                        "maximum": MAX_LOG_LINES,
                    },
                    "stream": {
                        "type": "string",
                        "description": "Filtre de flux. « system » regroupe les messages de Lynk Dev lui-même.",
                        "enum": ["stdout", "stderr", "system"],
                    },
                }),
                &["service_id"],
            ),
            tool(
                "check_port",
                "Indique si un port TCP local est libre, et quel service du profil actif le déclare.",
                json!({
                    "port": { "type": "integer", "description": "Port TCP.", "minimum": 1, "maximum": 65535 }
                }),
                &["port"],
            ),
            tool(
                "get_service_health",
                "Interroge la sonde d'un service : son URL de santé si elle est déclarée, \
    sinon l'ouverture de son port.",
                service_id_property(),
                &["service_id"],
            ),
            tool(
                "start_service",
                "Démarre un service du profil actif. Rend la main dès le lancement : \
    la montée en charge se suit avec list_services ou get_service_logs.",
                service_id_property(),
                &["service_id"],
            ),
            tool(
                "stop_service",
                "Arrête un service et tout son arbre de process (un `mvnw` laisse sinon un `java` vivant \
    qui garde le port).",
                service_id_property(),
                &["service_id"],
            ),
            tool(
                "restart_service",
                "Arrête puis redémarre un service, en laissant au port le temps de se libérer.",
                service_id_property(),
                &["service_id"],
            ),
            tool(
                "build_service",
                "Lance la commande de build déclarée par le service. \
    ⚠️ Bloque jusqu'à la fin du build, qui peut durer plusieurs minutes.",
                service_id_property(),
                &["service_id"],
            ),
        ]
    })
}

// ── Lecture des arguments ────────────────────────────────────────────────

fn string_arg(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("argument « {key} » manquant"))
}

/// Nombre de lignes demandé, borné. Une valeur hors bornes est **ramenée** dans
/// les bornes plutôt que refusée : un modèle qui demande 100 000 lignes veut
/// « le plus possible », pas une erreur.
fn lines_arg(args: &Value) -> usize {
    args.get("lines")
        .and_then(Value::as_u64)
        .map(|value| (value as usize).clamp(1, MAX_LOG_LINES))
        .unwrap_or(DEFAULT_LOG_LINES)
}

fn stream_arg(args: &Value) -> Result<Option<LogStream>, String> {
    match args.get("stream").and_then(Value::as_str) {
        None => Ok(None),
        Some("stdout") => Ok(Some(LogStream::Stdout)),
        Some("stderr") => Ok(Some(LogStream::Stderr)),
        Some("system") => Ok(Some(LogStream::System)),
        Some(other) => Err(format!(
            "flux « {other} » inconnu — attendu stdout, stderr ou system"
        )),
    }
}

fn port_arg(args: &Value) -> Result<u16, String> {
    let raw = args
        .get("port")
        .and_then(Value::as_u64)
        .ok_or_else(|| "argument « port » manquant".to_string())?;
    u16::try_from(raw)
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| format!("port {raw} hors bornes (1-65535)"))
}

/// Durée lisible depuis un instant en millisecondes.
pub fn format_uptime(elapsed_ms: i64) -> String {
    let seconds = (elapsed_ms / 1_000).max(0);
    match seconds {
        s if s < 60 => format!("{s} s"),
        s if s < 3_600 => format!("{} min", s / 60),
        s if s < 86_400 => format!("{} h {} min", s / 3_600, (s % 3_600) / 60),
        s => format!("{} j {} h", s / 86_400, (s % 86_400) / 3_600),
    }
}

// ── Profil actif ─────────────────────────────────────────────────────────

/// Le profil que la fenêtre a sous les yeux.
///
/// ⚠️ C'est **le seul périmètre d'écriture** du serveur. Un profil de secours
/// « le premier venu » exposerait des services que l'utilisateur ne regarde
/// pas, et un redémarrage tomberait au mauvais endroit.
async fn active_profile(ctx: &ToolContext) -> Result<DevProfile, String> {
    let id = settings::get(&ctx.pool, ACTIVE_PROFILE_KEY)
        .await
        .map_err(|err| format!("lecture des réglages : {err}"))?
        .and_then(|value| value.as_str().map(str::to_string))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            "aucun profil actif — ouvrez le Dev Manager dans Lynk Dev et choisissez un profil"
                .to_string()
        })?;

    dev_profiles::get(&ctx.pool, &id)
        .await
        .map_err(|err| format!("lecture du profil : {err}"))?
        .ok_or_else(|| format!("le profil actif ({id}) n'existe plus"))
}

/// Profil actif **et** service nommé, ou une erreur qui dit lequel manque.
async fn resolve(ctx: &ToolContext, args: &Value) -> Result<(DevProfile, ServiceConfig), String> {
    let service_id = string_arg(args, "service_id")?;
    let profile = active_profile(ctx).await?;
    let config = profile.service(&service_id).cloned().ok_or_else(|| {
        let known: Vec<&str> = profile.services.iter().map(|s| s.id.as_str()).collect();
        format!(
            "service « {service_id} » absent du profil actif — connus : {}",
            if known.is_empty() {
                "aucun".to_string()
            } else {
                known.join(", ")
            }
        )
    })?;
    Ok((profile, config))
}

// ── Exécution ────────────────────────────────────────────────────────────

pub async fn call(ctx: &ToolContext, name: &str, args: &Value) -> Outcome {
    match name {
        "list_services" => list_services(ctx).await,
        "get_service_logs" => get_service_logs(ctx, args).await,
        "check_port" => check_port(ctx, args).await,
        "get_service_health" => get_service_health(ctx, args).await,
        "start_service" => start_service(ctx, args).await,
        "stop_service" => stop_service(ctx, args).await,
        "restart_service" => restart_service(ctx, args).await,
        "build_service" => build_service(ctx, args).await,
        other => Outcome::failed(format!("outil inconnu : {other}")),
    }
}

async fn list_services(ctx: &ToolContext) -> Outcome {
    let profile = match active_profile(ctx).await {
        Ok(profile) => profile,
        Err(err) => return Outcome::failed(err),
    };

    let managed = ctx.supervisor.list(&profile.id);
    let now = chrono::Utc::now().timestamp_millis();

    // Les services que nous ne gérons pas peuvent tourner quand même (lancés
    // depuis un IDE, un terminal, une session précédente). On les sonde en
    // parallèle : en série, douze services à 400 ms font attendre cinq secondes.
    let mut probes = JoinSet::new();
    for config in &profile.services {
        let already_ours = managed.iter().any(|m| m.service_id == config.id);
        if let (false, Some(port)) = (already_ours, config.port) {
            let id = config.id.clone();
            probes.spawn(async move { (id, net::can_connect(port, EXTERNAL_PROBE).await) });
        }
    }
    let mut external = Vec::new();
    while let Some(result) = probes.join_next().await {
        if let Ok((id, true)) = result {
            external.push(id);
        }
    }

    let services: Vec<Value> = profile
        .services
        .iter()
        .map(|config| {
            let ours = managed.iter().find(|m| m.service_id == config.id);
            let status = match (&ours, external.contains(&config.id)) {
                (Some(_), _) => "running",
                (None, true) => "external",
                (None, false) => "stopped",
            };
            json!({
                "id": config.id,
                "name": config.name,
                "type": config.kind,
                "port": config.port,
                "status": status,
                "pid": ours.map(|m| m.pid),
                "uptime": ours.map(|m| format_uptime(now - m.started_at)),
                "workingDir": config.working_dir,
                "command": config.command,
                "autoRestart": config.auto_restart,
            })
        })
        .collect();

    Outcome::ok(json_text(&json!({
        "profile": { "id": profile.id, "name": profile.name, "rootPath": profile.root_path },
        "services": services,
    })))
}

async fn get_service_logs(ctx: &ToolContext, args: &Value) -> Outcome {
    let (_, config) = match resolve(ctx, args).await {
        Ok(found) => found,
        Err(err) => return Outcome::failed(err),
    };
    let stream = match stream_arg(args) {
        Ok(stream) => stream,
        Err(err) => return Outcome::failed(err),
    };

    let entries = ctx.logs.tail(&config.id, lines_arg(args), stream);
    if entries.is_empty() {
        return Outcome::ok(format!(
            "Aucune ligne pour « {} ». Le service n'a rien écrit depuis le démarrage de Lynk Dev, \
ou il tourne hors de l'application.",
            config.name
        ))
        .about(&config.id);
    }

    let body: String = entries
        .iter()
        .map(|entry| {
            let marker = match entry.stream {
                LogStream::Stdout => "  ",
                LogStream::Stderr => "E ",
                LogStream::System => "· ",
            };
            format!("{marker}{}", entry.text)
        })
        .collect::<Vec<_>>()
        .join("\n");

    Outcome::ok(format!(
        "{} — {} ligne(s) (E = flux d'erreur, · = message de Lynk Dev)\n\n{body}",
        config.name,
        entries.len()
    ))
    .about(&config.id)
}

async fn check_port(ctx: &ToolContext, args: &Value) -> Outcome {
    let port = match port_arg(args) {
        Ok(port) => port,
        Err(err) => return Outcome::failed(err),
    };

    let available = net::is_port_available(port).await;
    // Qui, dans le profil, revendique ce port ? C'est la question suivante dans
    // 100 % des cas ; autant y répondre tout de suite.
    let declared_by = active_profile(ctx)
        .await
        .ok()
        .and_then(|profile| {
            profile
                .services
                .iter()
                .find(|config| config.port == Some(port))
                .map(|config| json!({ "id": config.id, "name": config.name }))
        })
        .unwrap_or(Value::Null);

    Outcome::ok(json_text(&json!({
        "port": port,
        "available": available,
        "declaredBy": declared_by,
    })))
}

async fn get_service_health(ctx: &ToolContext, args: &Value) -> Outcome {
    let (profile, config) = match resolve(ctx, args).await {
        Ok(found) => found,
        Err(err) => return Outcome::failed(err),
    };

    let report = match (&config.health_check_url, config.port) {
        (Some(url), _) => json!({
            "probe": "healthCheckUrl",
            "url": url,
            "healthy": net::check_health_url(url, HEALTH_TIMEOUT).await,
        }),
        (None, Some(port)) => json!({
            "probe": "port",
            "port": port,
            "healthy": net::can_connect(port, HEALTH_TIMEOUT).await,
        }),
        (None, None) => json!({
            "probe": null,
            "healthy": null,
            "detail": "ni URL de santé ni port déclarés pour ce service",
        }),
    };

    Outcome::ok(json_text(&json!({
        "id": config.id,
        "name": config.name,
        "managedByLynkDev": ctx.supervisor.is_managed(&profile.id, &config.id),
        "health": report,
    })))
    .about(&config.id)
}

async fn start_service(ctx: &ToolContext, args: &Value) -> Outcome {
    let (profile, config) = match resolve(ctx, args).await {
        Ok(found) => found,
        Err(err) => return Outcome::failed(err),
    };
    if ctx.supervisor.is_managed(&profile.id, &config.id) {
        return Outcome::ok(format!("« {} » tourne déjà.", config.name)).about(&config.id);
    }

    let name = config.name.clone();
    let id = config.id.clone();
    ctx.supervisor
        .start(profile.id, config, StartOptions::default())
        .await;

    // Le superviseur rend la main dès le lancement ; la sonde de disponibilité
    // continue en tâche de fond. Le dire évite au modèle de conclure trop vite.
    Outcome::ok(format!(
        "Démarrage de « {name} » lancé. L'état devient « running » quand le port répond — \
à vérifier avec list_services."
    ))
    .about(id)
}

async fn stop_service(ctx: &ToolContext, args: &Value) -> Outcome {
    let (profile, config) = match resolve(ctx, args).await {
        Ok(found) => found,
        Err(err) => return Outcome::failed(err),
    };
    ctx.supervisor.stop(&profile.id, &config).await;
    Outcome::ok(format!("« {} » arrêté.", config.name)).about(&config.id)
}

async fn restart_service(ctx: &ToolContext, args: &Value) -> Outcome {
    let (profile, config) = match resolve(ctx, args).await {
        Ok(found) => found,
        Err(err) => return Outcome::failed(err),
    };
    ctx.supervisor.restart(&profile.id, &config).await;
    Outcome::ok(format!(
        "Redémarrage de « {} » lancé. L'état devient « running » quand le port répond.",
        config.name
    ))
    .about(&config.id)
}

async fn build_service(ctx: &ToolContext, args: &Value) -> Outcome {
    let (_, config) = match resolve(ctx, args).await {
        Ok(found) => found,
        Err(err) => return Outcome::failed(err),
    };
    let Some(command) = config.build_command.clone() else {
        return Outcome::failed(format!(
            "« {} » n'a pas de commande de build déclarée.",
            config.name
        ))
        .about(&config.id);
    };

    if ctx.supervisor.build(&config).await {
        Outcome::ok(format!("Build de « {} » réussi ({command}).", config.name)).about(&config.id)
    } else {
        Outcome::failed(format!(
            "Build de « {} » en échec ({command}) — le détail est dans get_service_logs.",
            config.name
        ))
        .about(&config.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool_names() -> Vec<String> {
        catalogue()["tools"]
            .as_array()
            .expect("tableau d'outils")
            .iter()
            .map(|tool| tool["name"].as_str().expect("nom").to_string())
            .collect()
    }

    #[test]
    fn the_catalogue_exposes_the_eight_expected_tools() {
        let names = tool_names();
        assert_eq!(names.len(), 8);
        for expected in [
            "list_services",
            "get_service_logs",
            "check_port",
            "get_service_health",
            "start_service",
            "stop_service",
            "restart_service",
            "build_service",
        ] {
            assert!(names.contains(&expected.to_string()), "{expected} absent");
        }
    }

    /// Le garde-fou tient par ce qui **n'est pas** exposé. Un outil qui prendrait
    /// une commande en argument ferait de ce serveur un shell distant.
    #[test]
    fn no_tool_executes_an_arbitrary_command() {
        for name in tool_names() {
            assert!(
                !name.contains("command") && !name.contains("exec") && !name.contains("shell"),
                "outil suspect : {name}"
            );
        }
        let raw = catalogue().to_string();
        assert!(!raw.contains("\"command\":{\"type\":\"string\""));
    }

    #[test]
    fn every_tool_declares_an_object_schema() {
        for tool in catalogue()["tools"].as_array().expect("outils") {
            assert_eq!(tool["inputSchema"]["type"], "object", "{}", tool["name"]);
            assert_eq!(tool["inputSchema"]["additionalProperties"], false);
            assert!(tool["description"].as_str().is_some_and(|d| !d.is_empty()));
        }
    }

    #[test]
    fn a_missing_string_argument_is_named_in_the_error() {
        let error = string_arg(&json!({}), "service_id").expect_err("doit échouer");
        assert!(error.contains("service_id"), "{error}");
        // Une chaîne vide ou d'espaces vaut absent.
        assert!(string_arg(&json!({ "service_id": "   " }), "service_id").is_err());
    }

    #[test]
    fn the_line_count_is_clamped_not_rejected() {
        assert_eq!(lines_arg(&json!({})), DEFAULT_LOG_LINES);
        assert_eq!(lines_arg(&json!({ "lines": 10 })), 10);
        assert_eq!(lines_arg(&json!({ "lines": 100_000 })), MAX_LOG_LINES);
        assert_eq!(lines_arg(&json!({ "lines": 0 })), 1);
    }

    #[test]
    fn an_unknown_stream_is_refused_with_the_allowed_values() {
        assert_eq!(stream_arg(&json!({})).expect("absent"), None);
        assert_eq!(
            stream_arg(&json!({ "stream": "stderr" })).expect("stderr"),
            Some(LogStream::Stderr)
        );
        let error = stream_arg(&json!({ "stream": "warn" })).expect_err("doit échouer");
        assert!(error.contains("stdout"), "{error}");
    }

    #[test]
    fn port_zero_and_out_of_range_are_refused() {
        assert_eq!(port_arg(&json!({ "port": 8080 })).expect("port"), 8080);
        assert!(port_arg(&json!({ "port": 0 })).is_err());
        assert!(port_arg(&json!({ "port": 70_000 })).is_err());
        assert!(port_arg(&json!({})).is_err());
    }

    #[test]
    fn uptime_reads_in_the_right_unit() {
        assert_eq!(format_uptime(12_000), "12 s");
        assert_eq!(format_uptime(90_000), "1 min");
        assert_eq!(format_uptime(3_600_000 * 2 + 60_000 * 13), "2 h 13 min");
        assert_eq!(format_uptime(86_400_000 * 3 + 3_600_000 * 5), "3 j 5 h");
        // Une horloge qui recule ne doit pas produire de durée négative.
        assert_eq!(format_uptime(-5_000), "0 s");
    }
}
