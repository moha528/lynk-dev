//! Tampon circulaire des sorties de service.
//!
//! Le superviseur **diffuse** ses lignes sur un canal ; il n'en garde aucune.
//! C'était suffisant tant que la fenêtre était le seul lecteur : elle accumule
//! dans son propre magasin. Le serveur MCP, lui, arrive après coup et pose une
//! question au passé — « les 100 dernières lignes de ce service » — à laquelle
//! un canal de diffusion ne sait pas répondre.
//!
//! D'où ce tampon : un abonné de plus, qui garde une fenêtre glissante par
//! service. Il vit dans `dev/` et non dans `mcp/` parce qu'il ne doit rien au
//! MCP — toute façade future y a droit.
//!
//! ⚠️ **Il ne commence à se remplir qu'au démarrage de l'application.** Un
//! service lancé hors de Lynk Dev n'a pas de lignes ici, et n'en aura jamais :
//! ce sont les sorties que *nous* avons capturées, pas un journal du système.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

use super::types::{DevEvent, LogEntry, LogStream};

/// Lignes conservées **par service**. Un `mvn` en boucle de redémarrage en
/// produit des milliers : sans plafond, la mémoire suit la durée de la session.
const MAX_LINES_PER_SERVICE: usize = 2_000;

/// Services suivis simultanément. Change de profil assez souvent et la table
/// grossit indéfiniment ; au-delà, on oublie le service dont la dernière ligne
/// est la plus ancienne.
const MAX_SERVICES: usize = 200;

#[derive(Default)]
pub struct LogStore {
    lines: Mutex<HashMap<String, VecDeque<LogEntry>>>,
}

impl LogStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Boucle d'alimentation, à confier à l'exécuteur de l'appelant.
    ///
    /// Rend une future plutôt que de lancer la tâche elle-même : ce module ne
    /// connaît pas l'exécuteur de l'application, et n'a pas à le connaître.
    ///
    /// ⚠️ Un retard (`Lagged`) ne **doit pas** rompre la boucle — sinon un
    /// service bavard au démarrage fige le tampon pour toute la session.
    pub fn drain(
        self: &Arc<Self>,
        mut events: broadcast::Receiver<DevEvent>,
    ) -> impl std::future::Future<Output = ()> + Send + 'static {
        let this = Arc::clone(self);
        async move {
            loop {
                match events.recv().await {
                    Ok(DevEvent::Log(event)) => this.push(&event.service_id, event.entry),
                    Ok(DevEvent::Status(_)) => {}
                    Err(broadcast::error::RecvError::Lagged(missed)) => {
                        tracing::warn!("tampon de logs : {missed} lignes perdues");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    pub fn push(&self, service_id: &str, entry: LogEntry) {
        let mut lines = self.lines.lock().expect("logs");

        if lines.len() >= MAX_SERVICES && !lines.contains_key(service_id) {
            let oldest = lines
                .iter()
                .min_by_key(|(_, entries)| entries.back().map(|e| e.timestamp).unwrap_or(0))
                .map(|(id, _)| id.clone());
            if let Some(id) = oldest {
                lines.remove(&id);
            }
        }

        let buffer = lines.entry(service_id.to_string()).or_default();
        if buffer.len() == MAX_LINES_PER_SERVICE {
            buffer.pop_front();
        }
        buffer.push_back(entry);
    }

    /// Les `lines` dernières lignes, filtrées par flux quand `stream` est fourni.
    ///
    /// Le filtre s'applique **avant** la coupure : demander 50 lignes d'erreur
    /// doit rendre 50 lignes d'erreur, pas ce qui reste des 50 dernières lignes
    /// tous flux confondus.
    pub fn tail(&self, service_id: &str, lines: usize, stream: Option<LogStream>) -> Vec<LogEntry> {
        let store = self.lines.lock().expect("logs");
        let Some(buffer) = store.get(service_id) else {
            return Vec::new();
        };
        let matching: Vec<&LogEntry> = buffer
            .iter()
            .filter(|entry| stream.is_none_or(|wanted| entry.stream == wanted))
            .collect();
        let start = matching.len().saturating_sub(lines);
        matching[start..]
            .iter()
            .map(|entry| (*entry).clone())
            .collect()
    }

    /// Oublie les lignes d'un service — l'écran peut vider sa vue.
    pub fn clear(&self, service_id: &str) {
        self.lines.lock().expect("logs").remove(service_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(stream: LogStream, text: &str) -> LogEntry {
        LogEntry::now(stream, text)
    }

    #[test]
    fn tail_returns_the_last_lines_in_order() {
        let store = LogStore::new();
        for i in 0..10 {
            store.push("svc", entry(LogStream::Stdout, &format!("ligne {i}")));
        }
        let tail = store.tail("svc", 3, None);
        assert_eq!(tail.len(), 3);
        assert_eq!(tail[0].text, "ligne 7");
        assert_eq!(tail[2].text, "ligne 9");
    }

    #[test]
    fn tail_of_an_unknown_service_is_empty_not_an_error() {
        let store = LogStore::new();
        assert!(store.tail("jamais-vu", 100, None).is_empty());
    }

    /// Le filtre porte sur l'ensemble du tampon, pas sur la dernière tranche.
    #[test]
    fn the_stream_filter_applies_before_the_cut() {
        let store = LogStore::new();
        store.push("svc", entry(LogStream::Stderr, "erreur 1"));
        for i in 0..50 {
            store.push("svc", entry(LogStream::Stdout, &format!("bruit {i}")));
        }
        store.push("svc", entry(LogStream::Stderr, "erreur 2"));

        let errors = store.tail("svc", 10, Some(LogStream::Stderr));
        assert_eq!(
            errors.len(),
            2,
            "les deux erreurs, malgré les 50 lignes entre"
        );
        assert_eq!(errors[0].text, "erreur 1");
        assert_eq!(errors[1].text, "erreur 2");
    }

    #[test]
    fn the_buffer_is_bounded_per_service() {
        let store = LogStore::new();
        for i in 0..(MAX_LINES_PER_SERVICE + 500) {
            store.push("svc", entry(LogStream::Stdout, &format!("l{i}")));
        }
        let all = store.tail("svc", usize::MAX, None);
        assert_eq!(all.len(), MAX_LINES_PER_SERVICE);
        // Ce sont les plus anciennes qui partent.
        assert_eq!(all[0].text, "l500");
    }

    #[test]
    fn clearing_forgets_a_service() {
        let store = LogStore::new();
        store.push("svc", entry(LogStream::Stdout, "x"));
        store.clear("svc");
        assert!(store.tail("svc", 10, None).is_empty());
    }
}
