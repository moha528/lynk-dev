//! Superviseur de process du Dev Manager.
//!
//! Traduction de `lynk-dev-electron/electron/dev-handlers.ts:110-560`.
//!
//! ⚠️ **Ce module ne connaît pas Tauri, et ne doit jamais le connaître.**
//! Il émet sur un canal `broadcast` ; l'application s'y abonne, et le futur
//! serveur MCP s'y abonnera pareillement. C'est le principe directeur du
//! chantier — *un seul superviseur, plusieurs façades*. Lui passer un
//! `AppHandle` obligerait le MCP à réécrire toute cette logique.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::{broadcast, watch};

use super::docker;
use super::net;
use super::types::{
    DevEvent, DevProfile, ExitReason, LogEntry, LogEvent, LogStream, ManagedProcessInfo,
    ProbeResult, ServiceConfig, ServiceStatus, ServiceType, StatusUpdate,
};

/// Nombre de redémarrages automatiques consécutifs avant de marquer « bloqué ».
const MAX_AUTO_RESTARTS: u32 = 5;
/// Regroupement des lignes de log. **Pas une micro-optimisation** : un `mvn` au
/// démarrage crache des milliers de lignes, et les émettre une par une noie le
/// canal d'événements.
const LOG_FLUSH: Duration = Duration::from_millis(100);
/// Durée maximale d'attente qu'un service ouvre son port / réponde.
const PROBE_TIMEOUT: Duration = Duration::from_secs(60);
/// Les conteneurs sont plus lents à lever que les process.
const DOCKER_PROBE_TIMEOUT: Duration = Duration::from_secs(120);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(3);
/// Délai laissé à un arbre de process pour mourir proprement avant la force.
const GRACEFUL_KILL: Duration = Duration::from_secs(5);

/// 2 s, 4 s, 8 s, 16 s, puis plafond à 30 s — identique à la version Electron.
fn backoff_delay(retry_count: u32) -> Duration {
    let exponent = retry_count.saturating_sub(1).min(16);
    let millis = 2_000u64.saturating_mul(1u64 << exponent);
    Duration::from_millis(millis.min(30_000))
}

type BoxFuture = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

#[derive(Debug, Clone, Copy, Default)]
pub struct StartOptions {
    pub retry_count: u32,
    /// Laisse au port le temps de se libérer avant de tenter le démarrage.
    /// Utilisé au redémarrage : l'OS peut encore démonter la socket d'écoute.
    pub wait_for_port: Option<Duration>,
}

struct Managed {
    profile_id: String,
    service_id: String,
    pid: u32,
    started_at: i64,
    auto_restart: Arc<AtomicBool>,
    intentional_stop: Arc<AtomicBool>,
    /// `true` tant que le process vit. Permet d'attendre sa mort sans course.
    running_rx: watch::Receiver<bool>,
}

pub struct Supervisor {
    procs: Mutex<HashMap<String, Managed>>,
    events: broadcast::Sender<DevEvent>,
    port_release_wait: Duration,
}

fn key_of(profile_id: &str, service_id: &str) -> String {
    format!("{profile_id}:{service_id}")
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

impl Supervisor {
    pub fn new(port_release_wait: Duration) -> Arc<Self> {
        let (events, _) = broadcast::channel(2_048);
        Arc::new(Self {
            procs: Mutex::new(HashMap::new()),
            events,
            port_release_wait,
        })
    }

    /// S'abonner au flux d'événements. Plusieurs façades peuvent le faire.
    pub fn subscribe(&self) -> broadcast::Receiver<DevEvent> {
        self.events.subscribe()
    }

    fn emit(&self, event: DevEvent) {
        // Sans abonné, l'envoi échoue : ce n'est pas une erreur.
        let _ = self.events.send(event);
    }

    fn log(&self, service_id: &str, stream: LogStream, text: impl Into<String>) {
        self.emit(DevEvent::Log(LogEvent {
            service_id: service_id.to_string(),
            entry: LogEntry::now(stream, text),
        }));
    }

    fn status(&self, update: StatusUpdate) {
        self.emit(DevEvent::Status(update));
    }

    /// Émettre un état depuis l'extérieur — utilisé par les démarrages groupés
    /// pour annoncer `waiting` avant même d'avoir lancé quoi que ce soit.
    pub fn announce(&self, update: StatusUpdate) {
        self.status(update);
    }

    /// Délai accordé à un port pour se libérer lors d'un redémarrage.
    pub fn port_release_wait(&self) -> Duration {
        self.port_release_wait
    }

    // ── Interrogation ────────────────────────────────────────────────────

    pub fn is_managed(&self, profile_id: &str, service_id: &str) -> bool {
        self.procs
            .lock()
            .expect("procs")
            .contains_key(&key_of(profile_id, service_id))
    }

    /// Process gérés pour un profil — ce que le front utilise pour se
    /// réconcilier avec la réalité après un rechargement de fenêtre.
    pub fn list(&self, profile_id: &str) -> Vec<ManagedProcessInfo> {
        let procs = self.procs.lock().expect("procs");
        let mut out: Vec<ManagedProcessInfo> = procs
            .values()
            .filter(|m| m.profile_id == profile_id)
            .map(|m| ManagedProcessInfo {
                service_id: m.service_id.clone(),
                pid: m.pid,
                started_at: m.started_at,
            })
            .collect();
        out.sort_by(|a, b| a.service_id.cmp(&b.service_id));
        out
    }

    // ── Démarrage ────────────────────────────────────────────────────────

    /// Démarre un service.
    ///
    /// Rend une future **boxée** : le redémarrage automatique rappelle `start`
    /// depuis la tâche qui surveille la sortie du process, ce qui serait une
    /// récursion de type infinie avec un `async fn` ordinaire.
    pub fn start(
        self: &Arc<Self>,
        profile_id: String,
        config: ServiceConfig,
        options: StartOptions,
    ) -> BoxFuture {
        let this = Arc::clone(self);
        Box::pin(async move { this.start_inner(profile_id, config, options).await })
    }

    async fn start_inner(
        self: Arc<Self>,
        profile_id: String,
        config: ServiceConfig,
        options: StartOptions,
    ) {
        let key = key_of(&profile_id, &config.id);

        if self.procs.lock().expect("procs").contains_key(&key) {
            self.stop_managed(&key).await;
        }

        if let Some(port) = config.port {
            if let Some(wait) = options.wait_for_port {
                if !wait.is_zero() {
                    self.log(
                        &config.id,
                        LogStream::System,
                        format!("Attente liberation port {port}..."),
                    );
                    if !net::wait_for_port_free(port, wait).await {
                        // Dernier recours : un orphelin d'une session précédente
                        // tient encore le port.
                        self.log(
                            &config.id,
                            LogStream::System,
                            format!("Port {port} occupe - force kill"),
                        );
                        net::kill_by_port(port).await;
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }

            if !net::is_port_available(port).await {
                self.status(
                    StatusUpdate::new(&config.id, ServiceStatus::Error)
                        .error(format!("Port {port} already in use")),
                );
                self.log(
                    &config.id,
                    LogStream::System,
                    format!("Port {port} deja utilise"),
                );
                return;
            }
        }

        let retry_count = options.retry_count;
        self.status(
            StatusUpdate::new(&config.id, ServiceStatus::Starting).retries(retry_count, false),
        );
        if retry_count > 0 {
            self.log(
                &config.id,
                LogStream::System,
                format!("Tentative de redemarrage #{retry_count}/{MAX_AUTO_RESTARTS}"),
            );
        }
        self.log(
            &config.id,
            LogStream::System,
            format!("Demarrage: {}", config.command),
        );
        self.log(
            &config.id,
            LogStream::System,
            format!("Repertoire: {}", config.working_dir),
        );

        let mut child = match shell_command(&config, &config.command).spawn() {
            Ok(child) => child,
            Err(err) => {
                self.log(&config.id, LogStream::System, format!("Erreur: {err}"));
                self.status(
                    StatusUpdate::new(&config.id, ServiceStatus::Error)
                        .error(err.to_string())
                        .exit(ExitReason::Crash, None),
                );
                return;
            }
        };

        let Some(pid) = child.id() else {
            self.log(&config.id, LogStream::System, "Process sans PID - abandon");
            self.status(
                StatusUpdate::new(&config.id, ServiceStatus::Error).error("process sans PID"),
            );
            return;
        };

        let auto_restart = Arc::new(AtomicBool::new(config.auto_restart));
        let intentional_stop = Arc::new(AtomicBool::new(false));
        let (running_tx, running_rx) = watch::channel(true);

        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(pump(
                stdout,
                LogStream::Stdout,
                config.id.clone(),
                self.events.clone(),
            ));
        }
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(pump(
                stderr,
                LogStream::Stderr,
                config.id.clone(),
                self.events.clone(),
            ));
        }

        self.procs.lock().expect("procs").insert(
            key.clone(),
            Managed {
                profile_id: profile_id.clone(),
                service_id: config.id.clone(),
                pid,
                started_at: now_ms(),
                auto_restart: Arc::clone(&auto_restart),
                intentional_stop: Arc::clone(&intentional_stop),
                running_rx,
            },
        );

        // Surveillance de la sortie + redémarrage automatique.
        {
            let this = Arc::clone(&self);
            let profile_id = profile_id.clone();
            let config = config.clone();
            let key = key.clone();
            tokio::spawn(async move {
                let mut child = child;
                let status = child.wait().await;
                this.procs.lock().expect("procs").remove(&key);
                let _ = running_tx.send(false);

                let code = status.as_ref().ok().and_then(|s| s.code());
                let was_intentional = intentional_stop.load(Ordering::SeqCst);

                let reason = if was_intentional {
                    this.log(
                        &config.id,
                        LogStream::System,
                        "Processus arrete volontairement",
                    );
                    ExitReason::Killed
                } else if matches!(code, Some(0) | None) {
                    this.log(
                        &config.id,
                        LogStream::System,
                        format!("Processus termine normalement (code {})", code.unwrap_or(0)),
                    );
                    ExitReason::Normal
                } else {
                    this.log(
                        &config.id,
                        LogStream::System,
                        format!("Processus crashe (code {})", code.unwrap_or(-1)),
                    );
                    ExitReason::Crash
                };

                if reason != ExitReason::Crash {
                    this.status(
                        StatusUpdate::new(&config.id, ServiceStatus::Stopped)
                            .exit(reason, code)
                            .retries(0, false),
                    );
                    return;
                }

                let can_retry =
                    auto_restart.load(Ordering::SeqCst) && retry_count < MAX_AUTO_RESTARTS;
                if !can_retry {
                    let stuck =
                        auto_restart.load(Ordering::SeqCst) && retry_count >= MAX_AUTO_RESTARTS;
                    if stuck {
                        this.log(
                            &config.id,
                            LogStream::System,
                            format!(
                                "Bloque apres {MAX_AUTO_RESTARTS} tentatives - redemarrez manuellement"
                            ),
                        );
                    }
                    this.status(
                        StatusUpdate::new(&config.id, ServiceStatus::Error)
                            .error(format!("Exit code {}", code.unwrap_or(-1)))
                            .exit(ExitReason::Crash, code)
                            .retries(retry_count, stuck),
                    );
                    return;
                }

                let next = retry_count + 1;
                let delay = backoff_delay(next);
                this.status(
                    StatusUpdate::new(&config.id, ServiceStatus::Error)
                        .error(format!("Exit code {}", code.unwrap_or(-1)))
                        .exit(ExitReason::Crash, code)
                        .retries(next, false),
                );
                this.log(
                    &config.id,
                    LogStream::System,
                    format!(
                        "Redemarrage automatique dans {}s ({next}/{MAX_AUTO_RESTARTS})",
                        delay.as_secs()
                    ),
                );
                tokio::time::sleep(delay).await;
                this.start(
                    profile_id,
                    config,
                    StartOptions {
                        retry_count: next,
                        wait_for_port: None,
                    },
                )
                .await;
            });
        }

        // Sonde de disponibilité.
        {
            let this = Arc::clone(&self);
            let config = config.clone();
            tokio::spawn(async move { this.probe_until_ready(key, config, pid).await });
        }
    }

    /// Attend qu'un service fraîchement lancé soit réellement joignable.
    async fn probe_until_ready(self: Arc<Self>, key: String, config: ServiceConfig, pid: u32) {
        let still_managed = || self.procs.lock().expect("procs").contains_key(&key);

        if config.kind == ServiceType::DockerCompose {
            let compose_file = config.compose_file();
            let dir = std::path::PathBuf::from(&config.working_dir);
            // 5 s avant la première sonde : les conteneurs ne lèvent pas tout de suite.
            tokio::time::sleep(Duration::from_secs(5)).await;
            let deadline = tokio::time::Instant::now() + DOCKER_PROBE_TIMEOUT;
            while tokio::time::Instant::now() < deadline {
                if !still_managed() {
                    return;
                }
                if docker::compose_running(&dir, &compose_file).await {
                    self.log(
                        &config.id,
                        LogStream::System,
                        "Containers prets - service running",
                    );
                    self.status(
                        StatusUpdate::new(&config.id, ServiceStatus::Running).pid(Some(pid)),
                    );
                    return;
                }
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            if still_managed() {
                self.log(
                    &config.id,
                    LogStream::System,
                    "Timeout: containers non prets apres 2min",
                );
                self.status(StatusUpdate::new(&config.id, ServiceStatus::Running).pid(Some(pid)));
            }
            return;
        }

        if let Some(port) = config.port {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let deadline = tokio::time::Instant::now() + PROBE_TIMEOUT;
            let mut announced_port = false;
            while tokio::time::Instant::now() < deadline {
                if !still_managed() {
                    return;
                }
                if net::can_connect(port, Duration::from_secs(1)).await {
                    let Some(url) = config.health_check_url.as_deref() else {
                        self.log(
                            &config.id,
                            LogStream::System,
                            format!("Port {port} accessible - service pret"),
                        );
                        self.status(
                            StatusUpdate::new(&config.id, ServiceStatus::Running).pid(Some(pid)),
                        );
                        return;
                    };
                    if !announced_port {
                        announced_port = true;
                        self.log(
                            &config.id,
                            LogStream::System,
                            format!("Port {port} ouvert - verification health check..."),
                        );
                    }
                    if net::check_health_url(url, HEALTH_TIMEOUT).await {
                        self.log(
                            &config.id,
                            LogStream::System,
                            "Health check OK - service pret",
                        );
                        self.status(
                            StatusUpdate::new(&config.id, ServiceStatus::Running).pid(Some(pid)),
                        );
                        return;
                    }
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            if still_managed() {
                // Le port répond mais pas le health check : on marque quand même
                // « running », comme la version Electron — sinon un service sain
                // dont l'URL de santé est mal configurée resterait bloqué en
                // « starting » pour toujours.
                self.log(
                    &config.id,
                    LogStream::System,
                    format!("Timeout sonde - service marque running (port {port})"),
                );
                self.status(StatusUpdate::new(&config.id, ServiceStatus::Running).pid(Some(pid)));
            }
            return;
        }

        if let Some(url) = config.health_check_url.clone() {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let deadline = tokio::time::Instant::now() + PROBE_TIMEOUT;
            while tokio::time::Instant::now() < deadline {
                if !still_managed() {
                    return;
                }
                if net::check_health_url(&url, HEALTH_TIMEOUT).await {
                    self.log(
                        &config.id,
                        LogStream::System,
                        "Health check OK - service pret",
                    );
                    self.status(
                        StatusUpdate::new(&config.id, ServiceStatus::Running).pid(Some(pid)),
                    );
                    return;
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            if still_managed() {
                self.log(
                    &config.id,
                    LogStream::System,
                    "Timeout health check - service marque running",
                );
                self.status(StatusUpdate::new(&config.id, ServiceStatus::Running).pid(Some(pid)));
            }
            return;
        }

        // Ni port ni URL : rien à sonder, on considère le service lancé.
        tokio::time::sleep(Duration::from_secs(1)).await;
        if still_managed() {
            self.status(StatusUpdate::new(&config.id, ServiceStatus::Running).pid(Some(pid)));
        }
    }

    // ── Arrêt ────────────────────────────────────────────────────────────

    /// Tue un process que **nous** supervisons, arbre compris.
    async fn stop_managed(&self, key: &str) {
        let Some((pid, mut running_rx)) = ({
            let procs = self.procs.lock().expect("procs");
            procs.get(key).map(|m| {
                // Couper l'auto-restart AVANT de tuer, sinon l'arrêt volontaire
                // relance le service qu'on vient d'arrêter.
                m.auto_restart.store(false, Ordering::SeqCst);
                m.intentional_stop.store(true, Ordering::SeqCst);
                (m.pid, m.running_rx.clone())
            })
        }) else {
            return;
        };

        terminate_tree(pid).await;

        if tokio::time::timeout(GRACEFUL_KILL, running_rx.wait_for(|running| !*running))
            .await
            .is_err()
        {
            force_kill_tree(pid).await;
            let _ = tokio::time::timeout(
                Duration::from_secs(3),
                running_rx.wait_for(|running| !*running),
            )
            .await;
        }
    }

    /// Arrête un service « externe » : lancé hors de Lynk Dev, donc sans PID
    /// connu. On passe par le compose ou par le port.
    async fn stop_external(&self, config: &ServiceConfig) -> bool {
        if config.kind == ServiceType::DockerCompose {
            let dir = std::path::PathBuf::from(&config.working_dir);
            return docker::compose_stop(&dir, &config.compose_file()).await;
        }
        if let Some(port) = config.port {
            let killed = net::kill_by_port(port).await;
            if killed {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            return killed;
        }
        false
    }

    /// Arrête un service, qu'il soit géré par nous ou externe.
    pub async fn stop(&self, profile_id: &str, config: &ServiceConfig) {
        let key = key_of(profile_id, &config.id);
        self.status(StatusUpdate::new(&config.id, ServiceStatus::Stopping));

        if self.procs.lock().expect("procs").contains_key(&key) {
            self.stop_managed(&key).await;
        } else {
            self.log(&config.id, LogStream::System, "Arret du service externe...");
            self.stop_external(config).await;
        }

        self.status(StatusUpdate::new(&config.id, ServiceStatus::Stopped));
    }

    /// Arrête puis relance, en laissant au port le temps de se libérer.
    pub async fn restart(self: &Arc<Self>, profile_id: &str, config: &ServiceConfig) {
        let key = key_of(profile_id, &config.id);
        self.status(StatusUpdate::new(&config.id, ServiceStatus::Stopping));

        if self.procs.lock().expect("procs").contains_key(&key) {
            self.stop_managed(&key).await;
        } else {
            self.log(&config.id, LogStream::System, "Arret du service externe...");
            self.stop_external(config).await;
        }

        self.start(
            profile_id.to_string(),
            config.clone(),
            StartOptions {
                retry_count: 0,
                wait_for_port: Some(self.port_release_wait),
            },
        )
        .await;
    }

    /// Coupe tout ce que nous supervisons. Appelé à la fermeture de l'app :
    /// sans ça, les services survivent à la fenêtre et gardent leurs ports.
    pub async fn stop_all(&self) {
        let keys: Vec<String> = self.procs.lock().expect("procs").keys().cloned().collect();
        for key in keys {
            self.stop_managed(&key).await;
        }
    }

    // ── Build ────────────────────────────────────────────────────────────

    /// Lance la commande de build en un coup, en diffusant sa sortie.
    /// Ce n'est **pas** un process supervisé : il n'entre pas dans la table.
    pub async fn build(&self, config: &ServiceConfig) -> bool {
        let Some(command) = config.build_command.clone() else {
            return false;
        };

        self.log(&config.id, LogStream::System, format!("Build: {command}"));
        self.log(
            &config.id,
            LogStream::System,
            format!("Repertoire: {}", config.working_dir),
        );
        self.status(StatusUpdate::new(&config.id, ServiceStatus::Starting));

        let mut child = match shell_command(config, &command).spawn() {
            Ok(child) => child,
            Err(err) => {
                self.log(
                    &config.id,
                    LogStream::System,
                    format!("Erreur build: {err}"),
                );
                self.status(
                    StatusUpdate::new(&config.id, ServiceStatus::Error).error(err.to_string()),
                );
                return false;
            }
        };

        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(pump(
                stdout,
                LogStream::Stdout,
                config.id.clone(),
                self.events.clone(),
            ));
        }
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(pump(
                stderr,
                LogStream::Stderr,
                config.id.clone(),
                self.events.clone(),
            ));
        }

        let code = child.wait().await.ok().and_then(|s| s.code());
        if code == Some(0) {
            self.log(&config.id, LogStream::System, "Build termine avec succes");
            self.status(StatusUpdate::new(&config.id, ServiceStatus::Stopped));
            true
        } else {
            self.log(
                &config.id,
                LogStream::System,
                format!("Build echoue (code {})", code.unwrap_or(-1)),
            );
            self.status(
                StatusUpdate::new(&config.id, ServiceStatus::Error)
                    .error(format!("Build exit code {}", code.unwrap_or(-1))),
            );
            false
        }
    }

    // ── Sonde des services non gérés ─────────────────────────────────────

    /// Cherche, parmi les services **que nous ne gérons pas**, ceux qui tournent
    /// déjà (lancés depuis un terminal, un IDE, une session précédente).
    pub async fn probe(&self, profile: &DevProfile) -> Vec<ProbeResult> {
        let mut results = Vec::new();
        for config in &profile.services {
            if self.is_managed(&profile.id, &config.id) {
                continue;
            }

            let mut detected = false;
            let mut via_health_check = false;

            // L'URL de santé est la preuve la plus solide : elle confirme
            // l'identité du service, pas seulement l'occupation d'un port.
            if let Some(url) = &config.health_check_url {
                if net::check_health_url(url, HEALTH_TIMEOUT).await {
                    detected = true;
                    via_health_check = true;
                }
            }

            if !detected {
                if config.kind == ServiceType::DockerCompose {
                    let dir = std::path::PathBuf::from(&config.working_dir);
                    detected = docker::compose_running(&dir, &config.compose_file()).await;
                    // `docker compose ps` nomme les conteneurs : c'est aussi
                    // une confirmation d'identité.
                    via_health_check = detected;
                } else if let Some(port) = config.port {
                    detected = !net::is_port_available(port).await;
                }
            }

            results.push(ProbeResult {
                service_id: config.id.clone(),
                detected,
                via_health_check,
            });
        }
        results
    }
}

/// Lit un flux ligne à ligne et l'émet par paquets de `LOG_FLUSH`.
async fn pump<R: AsyncRead + Unpin + Send + 'static>(
    reader: R,
    stream: LogStream,
    service_id: String,
    events: broadcast::Sender<DevEvent>,
) {
    let mut lines = BufReader::new(reader).lines();
    let mut buffer: Vec<String> = Vec::new();
    let mut ticker = tokio::time::interval(LOG_FLUSH);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let flush = |buffer: &mut Vec<String>| {
        if buffer.is_empty() {
            return;
        }
        let text = std::mem::take(buffer).join("\n");
        let _ = events.send(DevEvent::Log(LogEvent {
            service_id: service_id.clone(),
            entry: LogEntry::now(stream, text),
        }));
    };

    loop {
        tokio::select! {
            line = lines.next_line() => match line {
                Ok(Some(line)) => buffer.push(line),
                // Fin de flux ou flux illisible : on vide et on sort.
                _ => break,
            },
            _ = ticker.tick() => flush(&mut buffer),
        }
    }
    flush(&mut buffer);
}

/// Construit la commande shell d'un service.
///
/// La version Electron utilisait `shell: true`, ce qui revient exactement à
/// `cmd /C` sur Windows et `sh -c` ailleurs. Le conserver garantit que les
/// commandes déjà enregistrées par l'utilisateur continuent de fonctionner
/// (pipes, `&&`, variables du shell).
fn shell_command(config: &ServiceConfig, command: &str) -> Command {
    #[cfg(windows)]
    let mut cmd = {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]);
        cmd
    };
    #[cfg(unix)]
    let mut cmd = {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", command]);
        cmd
    };

    cmd.current_dir(&config.working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(env) = &config.env_vars {
        cmd.envs(env);
    }

    #[cfg(windows)]
    {
        // Sans ce drapeau, chaque service ouvre une fenêtre de console.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Le fils devient **chef de groupe** : c'est la seule façon de tuer
        // ensuite `mvnw` → `mvn` → `java` d'un seul `kill(-pid)`. Sans ça, le
        // shell meurt et la JVM survit en gardant le port.
        cmd.as_std_mut().process_group(0);
    }

    cmd
}

/// Demande poliment à tout l'arbre de s'arrêter.
async fn terminate_tree(pid: u32) {
    #[cfg(windows)]
    {
        // Windows ne connaît pas SIGTERM : `taskkill /T /F` est la seule façon
        // fiable de descendre l'arbre `cmd.exe` → `mvn` → `java`.
        let _ = crate::process::run_raw(
            std::path::Path::new("."),
            "taskkill",
            &["/pid", &pid.to_string(), "/T", "/F"],
            Duration::from_secs(10),
        )
        .await;
    }
    #[cfg(unix)]
    {
        // SAFETY: `kill(2)` sur un groupe de process. Un groupe déjà mort rend
        // simplement `ESRCH`, qu'on ignore volontairement.
        unsafe {
            libc::kill(-(pid as i32), libc::SIGTERM);
        }
    }
}

/// Ne laisse plus le choix.
async fn force_kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        // `taskkill /F` était déjà sans appel : rien à escalader.
        let _ = pid;
    }
    #[cfg(unix)]
    {
        // SAFETY: idem `terminate_tree`, en SIGKILL.
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::types::ServiceType;

    fn config(id: &str, command: &str) -> ServiceConfig {
        ServiceConfig {
            id: id.into(),
            name: id.into(),
            kind: ServiceType::Custom,
            working_dir: std::env::temp_dir().to_string_lossy().to_string(),
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

    /// Commande qui se termine tout de suite. `echo` existe dans `cmd` comme
    /// dans `sh` : la même chaîne convient aux deux.
    fn noop_command() -> &'static str {
        "echo bonjour"
    }

    /// Commande qui tourne assez longtemps pour être tuée.
    fn sleep_command() -> &'static str {
        if cfg!(windows) {
            "ping -n 60 127.0.0.1 > nul"
        } else {
            "sleep 60"
        }
    }

    #[test]
    fn backoff_follows_the_electron_schedule() {
        assert_eq!(backoff_delay(1), Duration::from_secs(2));
        assert_eq!(backoff_delay(2), Duration::from_secs(4));
        assert_eq!(backoff_delay(3), Duration::from_secs(8));
        assert_eq!(backoff_delay(4), Duration::from_secs(16));
        // Plafond.
        assert_eq!(backoff_delay(5), Duration::from_secs(30));
        assert_eq!(backoff_delay(50), Duration::from_secs(30));
    }

    #[test]
    fn keys_scope_a_service_to_its_profile() {
        assert_eq!(key_of("p1", "auth"), "p1:auth");
        assert_ne!(key_of("p1", "auth"), key_of("p2", "auth"));
    }

    /// Un process qui se termine seul doit sortir de la table et annoncer
    /// « stopped », sans redémarrage.
    #[tokio::test]
    async fn a_short_lived_process_reports_stopped_and_deregisters() {
        let sup = Supervisor::new(Duration::from_secs(1));
        let mut events = sup.subscribe();
        let cfg = config("noop", noop_command());

        sup.start("p1".into(), cfg, StartOptions::default()).await;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        let mut stopped = false;
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(Duration::from_secs(5), events.recv()).await {
                Ok(Ok(DevEvent::Status(update)))
                    if update.status == ServiceStatus::Stopped && update.service_id == "noop" =>
                {
                    stopped = true;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert!(stopped, "le service doit annoncer son arret");
        assert!(!sup.is_managed("p1", "noop"), "il doit sortir de la table");
    }

    /// Le chemin qui compte le plus : arrêter volontairement un process vivant.
    #[tokio::test]
    async fn stopping_a_running_process_deregisters_it() {
        let sup = Supervisor::new(Duration::from_secs(1));
        let cfg = config("sleeper", sleep_command());

        sup.start("p1".into(), cfg.clone(), StartOptions::default())
            .await;
        assert!(sup.is_managed("p1", "sleeper"), "il doit etre supervise");
        assert_eq!(sup.list("p1").len(), 1);

        sup.stop("p1", &cfg).await;
        assert!(
            !sup.is_managed("p1", "sleeper"),
            "apres arret, plus rien dans la table"
        );
        assert!(sup.list("p1").is_empty());
    }

    /// Un profil sans service géré ne remonte rien, et la sonde ne plante pas
    /// sur un port fermé.
    #[tokio::test]
    async fn probe_reports_a_stopped_service_as_undetected() {
        let sup = Supervisor::new(Duration::from_secs(1));
        let mut cfg = config("api", noop_command());
        // Port très improbablement occupé.
        cfg.port = Some(1);
        let profile = DevProfile {
            id: "p1".into(),
            name: "profil".into(),
            root_path: ".".into(),
            services: vec![cfg],
            created_at: now_ms(),
        };

        let results = sup.probe(&profile).await;
        assert_eq!(results.len(), 1);
        assert!(!results[0].detected);
        assert!(!results[0].via_health_check);
    }

    #[tokio::test]
    async fn build_without_a_command_is_a_no_op() {
        let sup = Supervisor::new(Duration::from_secs(1));
        assert!(!sup.build(&config("api", noop_command())).await);
    }

    #[tokio::test]
    async fn build_reports_success_and_failure() {
        let sup = Supervisor::new(Duration::from_secs(1));

        let mut ok = config("api", noop_command());
        ok.build_command = Some(noop_command().to_string());
        assert!(sup.build(&ok).await);

        let mut ko = config("api", noop_command());
        ko.build_command = Some("exit 3".to_string());
        assert!(!sup.build(&ko).await);
    }
}
