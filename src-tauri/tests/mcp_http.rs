//! Épreuve **de bout en bout** du serveur MCP : un vrai socket, de vraies
//! requêtes HTTP, le vrai superviseur.
//!
//! Les tests unitaires des modules vérifient des fonctions pures — la forme
//! d'un message, un contrôle d'origine. Ils ne disent rien de ce qui compte
//! ici : que le transport tienne, que le jeton ferme réellement la porte, et
//! qu'un outil appelé par HTTP atteigne bien le superviseur.
//!
//! Aucun service n'est démarré : ces tests portent sur le **chemin**, pas sur
//! le cycle de vie des process (déjà couvert ailleurs, et impossible à jouer en
//! intégration continue sans Maven ni Docker).

use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use lynk_dev_lib::dev::logs::LogStore;
use lynk_dev_lib::dev::types::{DevProfile, ServiceConfig, ServiceType};
use lynk_dev_lib::dev::Supervisor;
use lynk_dev_lib::mcp::{Journal, McpServer, ToolContext};
use lynk_dev_lib::store::{self, dev_profiles, settings, DbPool};

const TOKEN: &str = "jeton-de-test-0123456789";

struct Harness {
    server: Arc<McpServer>,
    pool: DbPool,
    url: String,
    client: reqwest::Client,
}

impl Harness {
    async fn start() -> Self {
        let file = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .expect("fichier temporaire");
        let path = file.path().to_path_buf();
        drop(file);
        let pool = store::init_pool(&path).await.expect("pool");

        let supervisor = Supervisor::new(Duration::from_secs(1));
        let logs = LogStore::new();
        let server = McpServer::new(
            ToolContext {
                pool: pool.clone(),
                supervisor,
                logs,
            },
            Journal::new(),
            "0.0.0-test",
        );

        // Port 0 : c'est le système qui choisit. Figer un port dans un test le
        // rend tributaire de ce qui tourne déjà sur la machine.
        let port = server.start(0, TOKEN.to_string()).await.expect("écoute");

        Self {
            server,
            pool,
            url: format!("http://127.0.0.1:{port}/mcp"),
            client: reqwest::Client::new(),
        }
    }

    fn post(&self, body: Value) -> reqwest::RequestBuilder {
        self.client.post(&self.url).bearer_auth(TOKEN).json(&body)
    }

    /// Envoie une requête JSON-RPC et rend le corps de la réponse.
    async fn rpc(&self, method: &str, params: Value) -> Value {
        let response = self
            .post(json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
            .send()
            .await
            .expect("requête");
        assert_eq!(response.status(), 200, "méthode {method}");
        response.json().await.expect("corps JSON")
    }

    /// Appelle un outil et rend (texte, isError).
    async fn tool(&self, name: &str, arguments: Value) -> (String, bool) {
        let body = self
            .rpc(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            )
            .await;
        let result = &body["result"];
        (
            result["content"][0]["text"]
                .as_str()
                .expect("texte")
                .to_string(),
            result["isError"].as_bool().expect("isError"),
        )
    }

    /// Installe un profil et le désigne comme actif, exactement comme le ferait
    /// la fenêtre.
    async fn with_profile(&self) {
        let profile = DevProfile {
            id: "p1".into(),
            name: "zeitune".into(),
            root_path: "C:/work/zeitune".into(),
            services: vec![ServiceConfig {
                id: "auth".into(),
                name: "olive_auth_service".into(),
                kind: ServiceType::SpringBootMaven,
                working_dir: "C:/work/zeitune/back/olive_auth_service".into(),
                command: "mvnw.cmd spring-boot:run".into(),
                build_command: Some("mvnw.cmd clean package -DskipTests".into()),
                // Hors des plages éphémères (Windows 49152+, Linux 32768+) :
                // un port de cette bande ne peut pas être attribué par surprise
                // à un autre binaire de test qui tourne en parallèle.
                port: Some(24_411),
                health_check_url: None,
                group: None,
                depends_on: None,
                env_vars: None,
                auto_restart: false,
            }],
            created_at: 0,
        };
        dev_profiles::save(&self.pool, &profile)
            .await
            .expect("profil");
        settings::set(&self.pool, "dev_profile_id", &json!("p1"))
            .await
            .expect("profil actif");
    }
}

#[tokio::test]
async fn a_request_without_a_token_is_refused() {
    let harness = Harness::start().await;

    let anonymous = harness
        .client
        .post(&harness.url)
        .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }))
        .send()
        .await
        .expect("requête");
    assert_eq!(anonymous.status(), 401);

    let wrong = harness
        .client
        .post(&harness.url)
        .bearer_auth("un-autre-jeton-de-la-meme-longueur")
        .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }))
        .send()
        .await
        .expect("requête");
    assert_eq!(wrong.status(), 401);

    harness.server.stop().await;
}

/// Le cas qui motive le contrôle d'origine : une page web ouverte dans le
/// navigateur, qui poste vers la boucle locale. Même avec le bon jeton, elle
/// doit être éconduite — et elle est éconduite **avant** la lecture du jeton.
#[tokio::test]
async fn a_remote_origin_is_refused_even_with_the_right_token() {
    let harness = Harness::start().await;

    let response = harness
        .post(json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }))
        .header("Origin", "https://exemple.test")
        .send()
        .await
        .expect("requête");
    assert_eq!(response.status(), 403);

    harness.server.stop().await;
}

#[tokio::test]
async fn the_handshake_and_the_catalogue_answer() {
    let harness = Harness::start().await;

    let init = harness
        .rpc(
            "initialize",
            json!({ "protocolVersion": "2025-06-18", "capabilities": {} }),
        )
        .await;
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "lynk-dev");
    assert_eq!(init["result"]["serverInfo"]["version"], "0.0.0-test");
    assert!(init["result"]["capabilities"]["tools"].is_object());

    let tools = harness.rpc("tools/list", json!({})).await;
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .expect("outils")
        .iter()
        .map(|tool| tool["name"].as_str().expect("nom"))
        .collect();
    assert_eq!(names.len(), 8);
    assert!(names.contains(&"list_services"));
    assert!(names.contains(&"restart_service"));

    harness.server.stop().await;
}

/// Une notification n'attend pas de réponse : le serveur accuse réception et
/// n'envoie **rien**. Un corps JSON-RPC ici est une faute de protocole.
#[tokio::test]
async fn a_notification_gets_an_empty_acknowledgement() {
    let harness = Harness::start().await;

    let response = harness
        .post(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
        .send()
        .await
        .expect("requête");
    assert_eq!(response.status(), 202);
    assert!(response.text().await.expect("corps").is_empty());

    harness.server.stop().await;
}

#[tokio::test]
async fn an_unknown_method_is_a_protocol_error() {
    let harness = Harness::start().await;

    let body = harness.rpc("resources/list", json!({})).await;
    assert_eq!(body["error"]["code"], -32601);
    assert!(body.get("result").is_none());

    harness.server.stop().await;
}

/// Sans profil actif, tout outil doit **dire pourquoi** plutôt que de choisir
/// un profil au hasard : un redémarrage tomberait au mauvais endroit.
#[tokio::test]
async fn without_an_active_profile_the_tools_explain_themselves() {
    let harness = Harness::start().await;

    let (text, is_error) = harness.tool("list_services", json!({})).await;
    assert!(is_error, "{text}");
    assert!(text.contains("aucun profil actif"), "{text}");

    harness.server.stop().await;
}

#[tokio::test]
async fn list_services_reads_the_active_profile() {
    let harness = Harness::start().await;
    harness.with_profile().await;

    let (text, is_error) = harness.tool("list_services", json!({})).await;
    assert!(!is_error, "{text}");

    let value: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(value["profile"]["name"], "zeitune");
    let service = &value["services"][0];
    assert_eq!(service["id"], "auth");
    assert_eq!(service["type"], "spring-boot-maven");
    assert_eq!(service["port"], 24_411);
    // Rien n'a été démarré, et rien n'écoute ce port.
    assert_eq!(service["status"], "stopped");
    assert!(service["pid"].is_null());

    harness.server.stop().await;
}

/// L'erreur doit nommer les services connus : un modèle qui se trompe d'
/// identifiant doit pouvoir se corriger sans un second appel.
#[tokio::test]
async fn an_unknown_service_is_refused_with_the_known_ones() {
    let harness = Harness::start().await;
    harness.with_profile().await;

    let (text, is_error) = harness
        .tool("get_service_logs", json!({ "service_id": "inexistant" }))
        .await;
    assert!(is_error, "{text}");
    assert!(text.contains("inexistant"), "{text}");
    assert!(
        text.contains("auth"),
        "les services connus doivent être cités : {text}"
    );

    harness.server.stop().await;
}

#[tokio::test]
async fn a_missing_argument_is_named() {
    let harness = Harness::start().await;
    harness.with_profile().await;

    let (text, is_error) = harness.tool("stop_service", json!({})).await;
    assert!(is_error, "{text}");
    assert!(text.contains("service_id"), "{text}");

    harness.server.stop().await;
}

#[tokio::test]
async fn check_port_names_the_service_that_declares_it() {
    let harness = Harness::start().await;
    harness.with_profile().await;

    let (text, is_error) = harness.tool("check_port", json!({ "port": 24_411 })).await;
    assert!(!is_error, "{text}");
    let value: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(value["port"], 24_411);
    assert_eq!(value["available"], true);
    assert_eq!(value["declaredBy"]["id"], "auth");

    harness.server.stop().await;
}

/// Chaque appel laisse une trace : c'est ce qui permet à un humain de savoir
/// qui a redémarré quoi.
#[tokio::test]
async fn every_call_lands_in_the_journal() {
    let harness = Harness::start().await;
    harness.with_profile().await;

    harness.tool("list_services", json!({})).await;
    harness
        .tool("get_service_logs", json!({ "service_id": "auth" }))
        .await;

    let journal = harness.server.journal().recent();
    assert_eq!(journal.len(), 2);
    // Du plus récent au plus ancien.
    assert_eq!(journal[0].tool, "get_service_logs");
    assert_eq!(journal[0].target.as_deref(), Some("auth"));
    assert!(journal[0].ok);
    assert_eq!(journal[1].tool, "list_services");

    harness.server.stop().await;
}

/// Le flux serveur → client n'existe pas ici, et le serveur doit le dire au
/// lieu de laisser le client attendre une connexion qui ne viendra pas.
#[tokio::test]
async fn the_get_stream_is_declined() {
    let harness = Harness::start().await;

    let response = harness
        .client
        .get(&harness.url)
        .bearer_auth(TOKEN)
        .send()
        .await
        .expect("requête");
    assert_eq!(response.status(), 405);

    harness.server.stop().await;
}

/// Sans attente de la fin de la tâche, un redémarrage immédiat sur le même
/// port échouerait en « adresse déjà utilisée ». C'est ce que ce test protège.
#[tokio::test]
async fn stopping_releases_the_port_immediately() {
    let harness = Harness::start().await;
    let port: u16 = harness
        .url
        .rsplit(':')
        .next()
        .and_then(|tail| tail.trim_end_matches("/mcp").parse().ok())
        .expect("port");

    harness.server.stop().await;
    assert!(harness.server.port().await.is_none());

    harness
        .server
        .start(port, TOKEN.to_string())
        .await
        .expect("reprise du même port");
    harness.server.stop().await;
}
