//! Types du Dev Manager.
//!
//! Miroir de `lynk-dev-electron/packages/dev-manager/src/types.ts` : les noms de
//! champs sont sérialisés en **camelCase** pour que le front porté n'ait rien à
//! renommer, et les valeurs d'énumération gardent leurs libellés d'origine.

use serde::{Deserialize, Serialize};

/// Famille d'un service détecté.
///
/// ⚠️ **Seul `DockerCompose` change un comportement** (sondes et arrêt passent
/// par `docker compose`). Tous les autres sont cosmétiques : ils servent à
/// nommer ce qu'on a reconnu et à proposer la bonne commande. En ajouter un
/// n'a donc aucun effet de bord sur le superviseur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceType {
    // JVM
    SpringBootMaven,
    SpringBootGradle,
    // JavaScript / TypeScript
    Next,
    Nuxt,
    Angular,
    Nest,
    SvelteKit,
    Astro,
    Remix,
    Vite,
    Node,
    // Python
    Django,
    Fastapi,
    Flask,
    Python,
    // Autres écosystèmes
    Go,
    Rust,
    Dotnet,
    Laravel,
    Rails,
    // Conteneurs
    DockerCompose,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ServiceStatus {
    #[default]
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
    /// Détecté sur son port mais démarré hors de Lynk Dev.
    External,
    /// En attente de ses dépendances (démarrage groupé).
    Waiting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExitReason {
    Normal,
    Crash,
    Killed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogStream {
    Stdout,
    Stderr,
    /// Messages de Lynk Dev lui-même (démarrage, sondes, arrêt).
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// Millisecondes depuis l'époque Unix (le front attend un `number`).
    pub timestamp: i64,
    pub stream: LogStream,
    pub text: String,
}

impl LogEntry {
    pub fn now(stream: LogStream, text: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Utc::now().timestamp_millis(),
            stream,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceConfig {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ServiceType,
    pub working_dir: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_check_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub auto_restart: bool,
}

impl ServiceConfig {
    /// Fichier compose visé par la commande, comme le faisait la regex
    /// `-f\s+(\S+)` de la version Electron. `docker-compose.yml` par défaut.
    pub fn compose_file(&self) -> String {
        let mut parts = self.command.split_whitespace();
        while let Some(token) = parts.next() {
            if token == "-f" {
                if let Some(file) = parts.next() {
                    return file.to_string();
                }
            }
        }
        "docker-compose.yml".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevProfile {
    pub id: String,
    pub name: String,
    pub root_path: String,
    #[serde(default)]
    pub services: Vec<ServiceConfig>,
    /// Millisecondes depuis l'époque Unix.
    pub created_at: i64,
}

impl DevProfile {
    pub fn service(&self, service_id: &str) -> Option<&ServiceConfig> {
        self.services.iter().find(|s| s.id == service_id)
    }
}

/// Mise à jour d'état poussée vers le front (canal `dev:service:status`).
///
/// Les champs optionnels sont omis quand ils ne s'appliquent pas, exactement
/// comme l'objet `extra` de `sendStatus` côté Electron.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusUpdate {
    pub service_id: String,
    pub status: ServiceStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_reason: Option<ExitReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stuck: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_for: Option<Vec<String>>,
}

impl StatusUpdate {
    pub fn new(service_id: impl Into<String>, status: ServiceStatus) -> Self {
        Self {
            service_id: service_id.into(),
            status,
            pid: None,
            error: None,
            exit_reason: None,
            exit_code: None,
            retry_count: None,
            stuck: None,
            waiting_for: None,
        }
    }

    pub fn pid(mut self, pid: Option<u32>) -> Self {
        self.pid = pid;
        self
    }

    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn exit(mut self, reason: ExitReason, code: Option<i32>) -> Self {
        self.exit_reason = Some(reason);
        self.exit_code = code;
        self
    }

    pub fn retries(mut self, count: u32, stuck: bool) -> Self {
        self.retry_count = Some(count);
        self.stuck = Some(stuck);
        self
    }

    pub fn waiting_for(mut self, names: Vec<String>) -> Self {
        self.waiting_for = Some(names);
        self
    }
}

/// Ligne de log poussée vers le front (canal `dev:service:log`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub service_id: String,
    pub entry: LogEntry,
}

/// Tout ce que le superviseur émet. **Volontairement indépendant de Tauri** :
/// l'app et le futur serveur MCP s'y abonnent de la même façon.
#[derive(Debug, Clone)]
pub enum DevEvent {
    Log(LogEvent),
    Status(StatusUpdate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceScanResult {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ServiceType,
    pub working_dir: String,
    pub suggested_command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_build_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub current: String,
    pub scanned: usize,
    pub found: usize,
}

/// Une demande de vérification de port (`dev_port_check_batch`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortRequest {
    pub service_id: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortCheckResult {
    pub service_id: String,
    pub port: u16,
    pub available: bool,
}

/// Une entrée de `dev:process:list` — ce que le front utilise pour se
/// réconcilier avec la réalité au démarrage.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedProcessInfo {
    pub service_id: String,
    pub pid: u32,
    pub started_at: i64,
}

/// Résultat d'une sonde de service non géré par nous.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub service_id: String,
    pub detected: bool,
    pub via_health_check: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerContainer {
    pub name: String,
    pub state: String,
    pub health: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DockerHealth {
    Up,
    Partial,
    Down,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerHealthReport {
    pub status: DockerHealth,
    pub services: Vec<DockerContainer>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(command: &str) -> ServiceConfig {
        ServiceConfig {
            id: "s1".into(),
            name: "svc".into(),
            kind: ServiceType::DockerCompose,
            working_dir: ".".into(),
            command: command.into(),
            build_command: None,
            port: None,
            health_check_url: None,
            group: None,
            depends_on: None,
            env_vars: None,
            auto_restart: false,
        }
    }

    #[test]
    fn compose_file_defaults_when_absent() {
        assert_eq!(
            config("docker compose up").compose_file(),
            "docker-compose.yml"
        );
    }

    #[test]
    fn compose_file_reads_the_dash_f_flag() {
        assert_eq!(
            config("docker compose -f infra/compose.yml up").compose_file(),
            "infra/compose.yml"
        );
    }

    /// Un `-f` en dernier token ne doit pas paniquer.
    #[test]
    fn compose_file_survives_a_dangling_flag() {
        assert_eq!(
            config("docker compose -f").compose_file(),
            "docker-compose.yml"
        );
    }

    /// Le front porté envoie du camelCase : la désérialisation doit l'accepter
    /// tel quel, sinon tout le module casse silencieusement.
    #[test]
    fn service_config_round_trips_camel_case() {
        let raw = serde_json::json!({
            "id": "auth",
            "name": "olive_auth_service",
            "type": "spring-boot-maven",
            "workingDir": "C:/work/back/olive_auth_service",
            "command": "mvnw.cmd spring-boot:run",
            "buildCommand": "mvnw.cmd clean package -DskipTests",
            "port": 8010,
            "healthCheckUrl": "http://localhost:8010/actuator/health",
            "dependsOn": ["postgres"],
            "envVars": { "SPRING_PROFILES_ACTIVE": "local" },
            "autoRestart": true
        });
        let cfg: ServiceConfig = serde_json::from_value(raw).expect("deserialise");
        assert_eq!(cfg.kind, ServiceType::SpringBootMaven);
        assert_eq!(cfg.port, Some(8010));
        assert!(cfg.auto_restart);
        assert_eq!(
            cfg.depends_on.as_deref(),
            Some(&["postgres".to_string()][..])
        );

        let back = serde_json::to_value(&cfg).expect("serialise");
        assert_eq!(back["workingDir"], "C:/work/back/olive_auth_service");
        assert_eq!(back["type"], "spring-boot-maven");
    }

    /// Les champs absents sont omis, pas rendus à `null`.
    #[test]
    fn status_update_omits_empty_fields() {
        let json =
            serde_json::to_value(StatusUpdate::new("s1", ServiceStatus::Running).pid(Some(42)))
                .expect("serialise");
        assert_eq!(json["serviceId"], "s1");
        assert_eq!(json["status"], "running");
        assert_eq!(json["pid"], 42);
        assert!(json.get("error").is_none());
        assert!(json.get("waitingFor").is_none());
    }
}
