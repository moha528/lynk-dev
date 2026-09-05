//! Démarrages, arrêts et redémarrages **groupés**, ordonnés par dépendances.
//!
//! Traduction de `lynk-dev-electron/electron/dev-handlers.ts:1127-1310`.
//!
//! Vit à côté du superviseur — et non dans la couche Tauri — pour que le futur
//! serveur MCP puisse déclencher un démarrage groupé sans réécrire l'ordre.

use std::sync::Arc;
use std::time::Duration;

use super::supervisor::{StartOptions, Supervisor};
use super::topo;
use super::types::{DevProfile, ServiceConfig, ServiceStatus, StatusUpdate};

/// Respiration entre deux couches, le temps que les dépendances lèvent.
const LAYER_GAP: Duration = Duration::from_secs(3);

fn selected<'a>(profile: &'a DevProfile, service_ids: &[String]) -> Vec<&'a ServiceConfig> {
    profile
        .services
        .iter()
        .filter(|s| service_ids.contains(&s.id))
        .collect()
}

/// Annonce en `waiting` tout ce qui n'est pas dans la première couche, pour que
/// l'utilisateur voie immédiatement *qui attend quoi* plutôt qu'une liste
/// figée.
fn announce_waiting(sup: &Supervisor, configs: &[ServiceConfig], layers: &[Vec<String>]) {
    for layer in layers.iter().skip(1) {
        for id in layer {
            let Some(config) = configs.iter().find(|c| &c.id == id) else {
                continue;
            };
            sup.announce(
                StatusUpdate::new(id, ServiceStatus::Waiting)
                    .waiting_for(topo::waiting_for_names(config, configs)),
            );
        }
    }
}

async fn start_layers(
    sup: &Arc<Supervisor>,
    profile_id: &str,
    configs: &[ServiceConfig],
    layers: &[Vec<String>],
    wait_for_port: Option<Duration>,
) {
    for (index, layer) in layers.iter().enumerate() {
        let mut set = tokio::task::JoinSet::new();
        for id in layer {
            let Some(config) = configs.iter().find(|c| &c.id == id).cloned() else {
                continue;
            };
            let sup = Arc::clone(sup);
            let profile_id = profile_id.to_string();
            set.spawn(async move {
                sup.start(
                    profile_id,
                    config,
                    StartOptions {
                        retry_count: 0,
                        wait_for_port,
                    },
                )
                .await;
            });
        }
        while set.join_next().await.is_some() {}

        if index + 1 < layers.len() {
            tokio::time::sleep(LAYER_GAP).await;
        }
    }
}

/// Démarre les services demandés, couche par couche.
pub async fn start(sup: &Arc<Supervisor>, profile: &DevProfile, service_ids: &[String]) {
    let configs: Vec<ServiceConfig> = selected(profile, service_ids)
        .into_iter()
        .cloned()
        .collect();
    if configs.is_empty() {
        return;
    }
    let layers = topo::layers(&configs);
    announce_waiting(sup, &configs, &layers);
    start_layers(sup, &profile.id, &configs, &layers, None).await;
}

/// Arrête les services demandés, tous en parallèle : l'ordre n'a pas de sens à
/// l'arrêt, et attendre en série rendrait la commande très lente.
pub async fn stop(sup: &Arc<Supervisor>, profile: &DevProfile, service_ids: &[String]) {
    let configs: Vec<ServiceConfig> = selected(profile, service_ids)
        .into_iter()
        .cloned()
        .collect();

    let mut set = tokio::task::JoinSet::new();
    for config in configs {
        let sup = Arc::clone(sup);
        let profile_id = profile.id.clone();
        set.spawn(async move {
            sup.stop(&profile_id, &config).await;
        });
    }
    while set.join_next().await.is_some() {}
}

/// Arrête tout, puis relance dans l'ordre des dépendances.
///
/// ⚠️ Le redémarrage passe `wait_for_port` : on vient de libérer ces ports, et
/// l'OS peut encore être en train de démonter les sockets d'écoute. Sans cette
/// attente, le redémarrage groupé échoue en « port déjà utilisé ».
pub async fn restart(sup: &Arc<Supervisor>, profile: &DevProfile, service_ids: &[String]) {
    let configs: Vec<ServiceConfig> = selected(profile, service_ids)
        .into_iter()
        .cloned()
        .collect();
    if configs.is_empty() {
        return;
    }

    stop(sup, profile, service_ids).await;

    let layers = topo::layers(&configs);
    announce_waiting(sup, &configs, &layers);
    start_layers(
        sup,
        &profile.id,
        &configs,
        &layers,
        Some(sup.port_release_wait()),
    )
    .await;
}
