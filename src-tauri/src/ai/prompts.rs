//! Consignes envoyées au modèle.
//!
//! Isolées et **pures** : elles sont la partie qu'on relira et ajustera le plus,
//! et elles doivent être vérifiables sans appel réseau.
//!
//! Deux règles gouvernent toutes les consignes de ce fichier :
//!
//! 1. **Le modèle ne rend que le résultat**, sans préambule ni bloc de code.
//!    Tout ce qui arrive est destiné à être collé tel quel dans un champ.
//! 2. **Ce qu'on envoie est borné.** Un diff de 40 000 lignes coûte cher, sort
//!    de la fenêtre de contexte, et ne donne pas un meilleur message qu'un
//!    extrait représentatif.

/// Au-delà, on tronque l'entrée. Généreux pour un diff de travail normal,
/// suffisant pour rester dans la fenêtre des modèles bon marché.
pub const MAX_INPUT_CHARS: usize = 24_000;

/// Tronque en signalant la coupure, pour que le modèle sache qu'il ne voit pas
/// tout et n'affirme rien sur ce qui manque.
pub fn truncate(input: &str) -> String {
    if input.len() <= MAX_INPUT_CHARS {
        return input.to_string();
    }
    // On coupe sur une frontière de caractère : `input[..n]` paniquerait au
    // milieu d'un caractère accentué.
    let mut end = MAX_INPUT_CHARS;
    while end > 0 && !input.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}\n\n[…tronqué : {} caractères de plus non transmis]",
        &input[..end],
        input.len() - end
    )
}

pub const COMMIT_SYSTEM: &str = "\
Tu rédiges des messages de commit Git au format Conventional Commits.

Règles :
- Première ligne : `type(portée facultative): description`, en français, à \
l'impératif, sans point final, 72 caractères au maximum.
- Types autorisés : feat, fix, refactor, perf, docs, test, build, ci, chore.
- Si le changement le mérite, ajoute une ligne vide puis un corps qui explique \
le POURQUOI, pas le comment — le diff dit déjà le comment.
- N'invente rien qui ne soit pas dans le diff.
- Réponds UNIQUEMENT par le message. Aucun préambule, aucun bloc de code, \
aucun guillemet autour.";

pub fn commit_user(diff: &str) -> String {
    format!(
        "Voici le diff indexé. Rédige le message de commit.\n\n{}",
        truncate(diff)
    )
}

pub const EXPLAIN_SYSTEM: &str = "\
Tu expliques un diff Git à un développeur qui connaît le projet.

Règles :
- Va droit au but : ce que ce changement fait, et ce qu'il implique.
- Signale un risque ou un effet de bord seulement s'il est réel et visible \
dans le diff.
- Cinq lignes au maximum, en français.
- Pas de préambule, pas de reformulation du diff ligne à ligne.";

pub fn explain_user(diff: &str) -> String {
    format!("Explique ce diff.\n\n{}", truncate(diff))
}

pub const LOGS_SYSTEM: &str = "\
Tu analyses la sortie d'un service de développement local.

Règles :
- Dis d'abord s'il y a une erreur, et laquelle — cite la ligne qui compte.
- Puis, si c'est identifiable, la cause probable et le geste à faire.
- Si rien ne cloche, dis-le en une ligne. N'invente pas de problème.
- Huit lignes au maximum, en français.";

pub fn logs_user(logs: &str) -> String {
    format!(
        "Voici les dernières lignes du service. Que s'est-il passé ?\n\n{}",
        truncate(logs)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_input_is_untouched() {
        assert_eq!(truncate("diff court"), "diff court");
    }

    #[test]
    fn long_input_is_cut_and_says_so() {
        let long = "a".repeat(MAX_INPUT_CHARS + 500);
        let cut = truncate(&long);
        assert!(cut.len() < long.len());
        assert!(cut.contains("tronqué"), "la coupure doit etre annoncee");
        assert!(cut.contains("500 caractères de plus"));
    }

    /// Le piège : couper au milieu d'un caractère multi-octets fait paniquer
    /// l'indexation d'une `String`.
    #[test]
    fn truncation_never_splits_a_character() {
        // Chaque « é » fait deux octets : la coupure tombe forcément au milieu
        // de l'un d'eux pour au moins une longueur de départ.
        for extra in 0..4 {
            let input = "é".repeat(MAX_INPUT_CHARS / 2 + extra);
            let cut = truncate(&input);
            assert!(cut.starts_with('é'));
        }
    }

    #[test]
    fn prompts_carry_the_payload() {
        assert!(commit_user("mon diff").contains("mon diff"));
        assert!(explain_user("mon diff").contains("mon diff"));
        assert!(logs_user("ma sortie").contains("ma sortie"));
    }
}
