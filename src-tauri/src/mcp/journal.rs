//! Journal des appels MCP.
//!
//! Un serveur qui démarre et arrête des services pour le compte d'un modèle
//! doit laisser une trace lisible par un humain : **qui a redémarré quoi, et
//! quand**. Sans ce journal, un service qui tombe pendant qu'on regarde
//! ailleurs n'a aucune explication.
//!
//! Le journal vit en mémoire et meurt avec l'application : c'est un fil
//! d'activité, pas un audit. Le persister demanderait une rétention, une purge
//! et une migration — pour une information dont la valeur est immédiate.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::sync::broadcast;

/// Appels conservés. Au-delà, les plus anciens sortent.
const MAX_RECORDS: usize = 200;

/// Longueur au-delà de laquelle le détail est coupé : le journal est une liste,
/// pas une console.
const MAX_DETAIL: usize = 240;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallRecord {
    /// Millisecondes depuis l'époque Unix.
    pub at: i64,
    pub tool: String,
    /// Service visé, quand l'outil en désigne un.
    pub target: Option<String>,
    pub ok: bool,
    pub detail: String,
    pub duration_ms: u64,
}

impl CallRecord {
    pub fn new(
        tool: &str,
        target: Option<String>,
        ok: bool,
        detail: &str,
        duration_ms: u64,
    ) -> Self {
        Self {
            at: chrono::Utc::now().timestamp_millis(),
            tool: tool.to_string(),
            target,
            ok,
            detail: truncate(detail),
            duration_ms,
        }
    }
}

/// Coupe sur une **frontière de caractère** : trancher au milieu d'un « é »
/// produirait une chaîne invalide et ferait paniquer la sérialisation.
fn truncate(text: &str) -> String {
    let flat = text.replace('\n', " ");
    if flat.chars().count() <= MAX_DETAIL {
        return flat;
    }
    let cut: String = flat.chars().take(MAX_DETAIL).collect();
    format!("{cut}…")
}

pub struct Journal {
    records: Mutex<VecDeque<CallRecord>>,
    events: broadcast::Sender<CallRecord>,
}

impl Journal {
    pub fn new() -> Arc<Self> {
        let (events, _) = broadcast::channel(64);
        Arc::new(Self {
            records: Mutex::new(VecDeque::new()),
            events,
        })
    }

    /// S'abonner au flux — l'application y branche un pont vers la fenêtre.
    pub fn subscribe(&self) -> broadcast::Receiver<CallRecord> {
        self.events.subscribe()
    }

    pub fn record(&self, record: CallRecord) {
        {
            let mut records = self.records.lock().expect("journal");
            if records.len() == MAX_RECORDS {
                records.pop_front();
            }
            records.push_back(record.clone());
        }
        // Sans abonné, l'envoi échoue : ce n'est pas une erreur.
        let _ = self.events.send(record);
    }

    /// Les appels, **du plus récent au plus ancien** — l'ordre de lecture.
    pub fn recent(&self) -> Vec<CallRecord> {
        self.records
            .lock()
            .expect("journal")
            .iter()
            .rev()
            .cloned()
            .collect()
    }

    pub fn clear(&self) {
        self.records.lock().expect("journal").clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(tool: &str) -> CallRecord {
        CallRecord::new(tool, None, true, "ok", 3)
    }

    #[test]
    fn the_most_recent_call_comes_first() {
        let journal = Journal::new();
        journal.record(record("list_services"));
        journal.record(record("stop_service"));
        let recent = journal.recent();
        assert_eq!(recent[0].tool, "stop_service");
        assert_eq!(recent[1].tool, "list_services");
    }

    #[test]
    fn the_journal_is_bounded() {
        let journal = Journal::new();
        for i in 0..(MAX_RECORDS + 50) {
            journal.record(record(&format!("outil{i}")));
        }
        let recent = journal.recent();
        assert_eq!(recent.len(), MAX_RECORDS);
        assert_eq!(recent[0].tool, format!("outil{}", MAX_RECORDS + 49));
    }

    /// La coupure porte sur des caractères, pas des octets.
    #[test]
    fn a_long_accented_detail_is_cut_without_breaking_it() {
        let long = "é".repeat(MAX_DETAIL + 100);
        let record = CallRecord::new("t", None, false, &long, 0);
        assert_eq!(
            record.detail.chars().count(),
            MAX_DETAIL + 1,
            "coupe + ellipse"
        );
        assert!(record.detail.ends_with('…'));
    }

    #[test]
    fn newlines_are_flattened_so_the_list_stays_a_list() {
        let record = CallRecord::new("t", None, true, "ligne 1\nligne 2", 0);
        assert_eq!(record.detail, "ligne 1 ligne 2");
    }

    #[test]
    fn clearing_empties_the_journal() {
        let journal = Journal::new();
        journal.record(record("x"));
        journal.clear();
        assert!(journal.recent().is_empty());
    }
}
