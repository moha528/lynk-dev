//! Ordonnancement des services par dépendances, en couches.
//!
//! Traduction de `lynk-dev-electron/electron/dev-handlers.ts:882-936`.
//!
//! ⚠️ **Tolérant aux cycles** : si plus aucun service n'est de degré entrant nul,
//! le reste part en une seule couche au lieu de boucler. Un profil mal
//! configuré ne doit jamais figer l'application.

use std::collections::{HashMap, HashSet};

use super::types::ServiceConfig;

/// Découpe `services` en couches successives : tout ce qui est dans la couche
/// *n* peut démarrer en parallèle, une fois la couche *n-1* lancée.
///
/// L'ordre d'entrée est conservé à l'intérieur d'une couche — les démarrages
/// groupés restent donc reproductibles d'une exécution à l'autre.
pub fn layers(services: &[ServiceConfig]) -> Vec<Vec<String>> {
    let ids: HashSet<&str> = services.iter().map(|s| s.id.as_str()).collect();
    let mut in_degree: HashMap<&str, usize> =
        services.iter().map(|s| (s.id.as_str(), 0usize)).collect();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

    for service in services {
        let Some(deps) = &service.depends_on else {
            continue;
        };
        for dep in deps {
            // Une dépendance hors profil est ignorée, comme côté Electron :
            // elle bloquerait un démarrage pour un service qu'on ne gère pas.
            if !ids.contains(dep.as_str()) {
                continue;
            }
            adj.entry(dep.as_str()).or_default().push(&service.id);
            if let Some(degree) = in_degree.get_mut(service.id.as_str()) {
                *degree += 1;
            }
        }
    }

    let mut out: Vec<Vec<String>> = Vec::new();
    let mut remaining = services.len();

    while remaining > 0 {
        let layer: Vec<&str> = services
            .iter()
            .map(|s| s.id.as_str())
            .filter(|id| in_degree.get(id) == Some(&0))
            .collect();

        if layer.is_empty() {
            // Cycle : on rend le reliquat d'un bloc plutôt que de tourner.
            let leftover: Vec<String> = services
                .iter()
                .map(|s| s.id.as_str())
                .filter(|id| in_degree.contains_key(id))
                .map(str::to_string)
                .collect();
            if !leftover.is_empty() {
                out.push(leftover);
            }
            break;
        }

        for id in &layer {
            in_degree.remove(id);
            if let Some(children) = adj.get(id) {
                for child in children {
                    if let Some(degree) = in_degree.get_mut(child) {
                        *degree = degree.saturating_sub(1);
                    }
                }
            }
        }

        remaining -= layer.len();
        out.push(layer.into_iter().map(str::to_string).collect());
    }

    out
}

/// Noms (et non identifiants) des dépendances encore attendues par `service`,
/// pour l'affichage de l'état `waiting`.
pub fn waiting_for_names(service: &ServiceConfig, all: &[ServiceConfig]) -> Vec<String> {
    let Some(deps) = &service.depends_on else {
        return Vec::new();
    };
    deps.iter()
        .filter_map(|dep| all.iter().find(|s| &s.id == dep))
        .map(|s| s.name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::types::ServiceType;

    fn svc(id: &str, deps: &[&str]) -> ServiceConfig {
        ServiceConfig {
            id: id.into(),
            name: format!("service {id}"),
            kind: ServiceType::Custom,
            working_dir: ".".into(),
            command: "true".into(),
            build_command: None,
            port: None,
            health_check_url: None,
            group: None,
            depends_on: if deps.is_empty() {
                None
            } else {
                Some(deps.iter().map(|d| (*d).to_string()).collect())
            },
            env_vars: None,
            auto_restart: false,
        }
    }

    #[test]
    fn independent_services_share_one_layer() {
        let services = vec![svc("a", &[]), svc("b", &[]), svc("c", &[])];
        assert_eq!(layers(&services), vec![vec!["a", "b", "c"]]);
    }

    #[test]
    fn dependencies_produce_ordered_layers() {
        // postgres <- auth <- gateway ; settings depend aussi de postgres
        let services = vec![
            svc("gateway", &["auth"]),
            svc("auth", &["postgres"]),
            svc("settings", &["postgres"]),
            svc("postgres", &[]),
        ];
        assert_eq!(
            layers(&services),
            vec![vec!["postgres"], vec!["auth", "settings"], vec!["gateway"],]
        );
    }

    #[test]
    fn unknown_dependencies_are_ignored() {
        let services = vec![svc("a", &["not-in-profile"])];
        assert_eq!(layers(&services), vec![vec!["a"]]);
    }

    /// Le cas qui doit surtout ne pas figer l'app.
    #[test]
    fn a_cycle_falls_back_to_a_single_layer() {
        let services = vec![svc("a", &["b"]), svc("b", &["a"])];
        assert_eq!(layers(&services), vec![vec!["a", "b"]]);
    }

    #[test]
    fn a_cycle_after_a_valid_layer_keeps_what_was_resolved() {
        let services = vec![svc("root", &[]), svc("a", &["b"]), svc("b", &["a"])];
        assert_eq!(layers(&services), vec![vec!["root"], vec!["a", "b"]]);
    }

    #[test]
    fn self_dependency_does_not_hang() {
        let services = vec![svc("a", &["a"])];
        assert_eq!(layers(&services), vec![vec!["a"]]);
    }

    #[test]
    fn empty_input_yields_no_layer() {
        assert!(layers(&[]).is_empty());
    }

    #[test]
    fn waiting_names_resolve_ids_to_labels() {
        let services = vec![svc("auth", &["postgres"]), svc("postgres", &[])];
        assert_eq!(
            waiting_for_names(&services[0], &services),
            vec!["service postgres"]
        );
    }
}
