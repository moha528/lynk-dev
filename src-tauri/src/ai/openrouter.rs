//! Client OpenRouter.
//!
//! OpenRouter expose une API compatible OpenAI et donne accès à des dizaines de
//! modèles derrière une seule clé — dont plusieurs à très bas coût, ce qui est
//! tout l'intérêt pour des tâches courtes et répétées comme un message de
//! commit.
//!
//! ⚠️ **Aucun identifiant de modèle n'est figé dans le code.** Le catalogue et
//! les tarifs bougent tous les mois ; l'application charge la liste en direct et
//! l'utilisateur choisit. Coder « le modèle pas cher du moment » en dur, c'est
//! garantir qu'il sera périmé dans trois mois.

use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

const ENDPOINT: &str = "https://openrouter.ai/api/v1";

/// Une requête de complétion courte n'a aucune raison de durer plus longtemps.
const TIMEOUT: Duration = Duration::from_secs(60);

/// En-têtes recommandés par OpenRouter pour identifier l'application appelante.
const REFERER: &str = "https://github.com/moha528/lynk-dev";
const TITLE: &str = "Lynk Dev";

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    max_tokens: u32,
    /// Peu de créativité : on veut un message de commit reproductible, pas une
    /// variation littéraire à chaque appel.
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
    #[serde(default)]
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    #[serde(default, rename = "prompt_tokens")]
    pub prompt_tokens: u32,
    #[serde(default, rename = "completion_tokens")]
    pub completion_tokens: u32,
    #[serde(default, rename = "total_tokens")]
    pub total_tokens: u32,
}

/// Ce qu'une complétion rend à l'appelant.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Completion {
    pub text: String,
    pub usage: Usage,
    pub model: String,
}

/// Un modèle du catalogue, tel que l'écran de réglages l'affiche.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    /// Coût en dollars par **million** de jetons d'entrée. `None` si absent.
    pub prompt_price: Option<f64>,
    /// Idem, en sortie.
    pub completion_price: Option<f64>,
    pub context_length: Option<u64>,
    /// Gratuit d'après le tarif annoncé — pratique pour un premier essai.
    pub free: bool,
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .context("construction du client HTTP")
}

fn authorized(builder: reqwest::RequestBuilder, api_key: &str) -> reqwest::RequestBuilder {
    builder
        .bearer_auth(api_key)
        .header("HTTP-Referer", REFERER)
        .header("X-Title", TITLE)
}

/// Demande une complétion et rend le texte, sans enrobage.
pub async fn complete(
    api_key: &str,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u32,
) -> Result<Completion> {
    if api_key.trim().is_empty() {
        bail!("aucune clé OpenRouter configurée");
    }

    let request = ChatRequest {
        model,
        messages: vec![
            Message {
                role: "system",
                content: system,
            },
            Message {
                role: "user",
                content: user,
            },
        ],
        max_tokens,
        temperature: 0.2,
    };

    let response = authorized(
        client()?.post(format!("{ENDPOINT}/chat/completions")),
        api_key,
    )
    .json(&request)
    .send()
    .await
    .context("appel à OpenRouter")?;

    let status = response.status();
    let body: ChatResponse = response
        .json()
        .await
        .context("réponse OpenRouter illisible")?;

    // OpenRouter place son message d'erreur dans le corps, y compris sur un 200
    // quand le modèle refuse : on le remonte tel quel, il est explicite.
    if let Some(error) = body.error {
        bail!("{}", error.message);
    }
    if !status.is_success() {
        bail!("OpenRouter a répondu {status}");
    }

    let text = body
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .unwrap_or_default();
    if text.trim().is_empty() {
        bail!("le modèle n'a rien répondu");
    }

    Ok(Completion {
        text: clean(&text),
        usage: body.usage.unwrap_or_default(),
        model: model.to_string(),
    })
}

/// Retire l'enrobage que les modèles ajoutent malgré la consigne : blocs de
/// code, guillemets encadrants, espaces de tête et de queue.
pub fn clean(text: &str) -> String {
    let trimmed = text.trim();

    let without_fence = if trimmed.starts_with("```") {
        let inner = trimmed.trim_start_matches("```");
        // La première ligne d'un bloc porte parfois le langage : on la saute.
        let inner = inner
            .split_once('\n')
            .map(|(_, rest)| rest)
            .unwrap_or(inner);
        inner.trim_end().trim_end_matches("```").trim()
    } else {
        trimmed
    };

    let unquoted = without_fence
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(without_fence);

    unquoted.trim().to_string()
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<RawModel>,
}

#[derive(Debug, Deserialize)]
struct RawModel {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    context_length: Option<u64>,
    #[serde(default)]
    pricing: Option<RawPricing>,
}

#[derive(Debug, Deserialize)]
struct RawPricing {
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    completion: Option<String>,
}

/// Les tarifs arrivent en **dollars par jeton**, sous forme de chaîne. On les
/// ramène au million, seule échelle où les chiffres sont lisibles.
fn price_per_million(raw: Option<&String>) -> Option<f64> {
    raw?.parse::<f64>().ok().map(|value| value * 1_000_000.0)
}

/// Catalogue des modèles disponibles, trié du moins cher au plus cher.
pub async fn list_models(api_key: &str) -> Result<Vec<ModelInfo>> {
    let response = authorized(client()?.get(format!("{ENDPOINT}/models")), api_key)
        .send()
        .await
        .context("catalogue OpenRouter")?;

    let body: ModelsResponse = response.json().await.context("catalogue illisible")?;

    let mut models: Vec<ModelInfo> = body
        .data
        .into_iter()
        .map(|raw| {
            let pricing = raw.pricing.unwrap_or(RawPricing {
                prompt: None,
                completion: None,
            });
            let prompt_price = price_per_million(pricing.prompt.as_ref());
            let completion_price = price_per_million(pricing.completion.as_ref());
            ModelInfo {
                name: raw.name.unwrap_or_else(|| raw.id.clone()),
                id: raw.id,
                free: prompt_price == Some(0.0) && completion_price == Some(0.0),
                prompt_price,
                completion_price,
                context_length: raw.context_length,
            }
        })
        .collect();

    // Le moins cher d'abord : c'est le critère qui a motivé le choix
    // d'OpenRouter. Un tarif absent part à la fin plutôt qu'au début, pour ne
    // pas faire passer un modèle inconnu pour gratuit.
    models.sort_by(|a, b| {
        let left = a.prompt_price.unwrap_or(f64::MAX);
        let right = b.prompt_price.unwrap_or(f64::MAX);
        left.partial_cmp(&right)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_strips_a_fenced_block_with_its_language() {
        let raw = "```\nfeat: ajoute X\n```";
        assert_eq!(clean(raw), "feat: ajoute X");

        let with_language = "```text\nfeat: ajoute X\n```";
        assert_eq!(clean(with_language), "feat: ajoute X");
    }

    #[test]
    fn clean_strips_surrounding_quotes() {
        assert_eq!(clean("\"feat: ajoute X\""), "feat: ajoute X");
    }

    #[test]
    fn clean_keeps_a_multiline_body() {
        let raw = "feat: ajoute X\n\nParce que Y.";
        assert_eq!(clean(raw), "feat: ajoute X\n\nParce que Y.");
    }

    #[test]
    fn clean_leaves_inner_quotes_alone() {
        assert_eq!(
            clean("fix: gere le cas \"vide\""),
            "fix: gere le cas \"vide\""
        );
    }

    #[test]
    fn prices_are_scaled_to_a_million_tokens() {
        // Comparaison a tolerance : `0.0000001 * 1e6` vaut 0.09999999999999999
        // en binaire. Exiger l'egalite exacte sur un flottant est une faute,
        // pas un defaut du calcul.
        let scaled = price_per_million(Some(&"0.0000001".to_string())).expect("tarif");
        assert!(
            (scaled - 0.1).abs() < 1e-9,
            "0,1 $ le million, obtenu {scaled}"
        );
        assert_eq!(price_per_million(Some(&"0".to_string())), Some(0.0));
        assert_eq!(price_per_million(None), None);
        assert_eq!(price_per_million(Some(&"gratuit".to_string())), None);
    }

    #[tokio::test]
    async fn an_empty_key_fails_before_any_network_call() {
        let error = complete("   ", "un/modele", "s", "u", 100)
            .await
            .expect_err("doit refuser sans cle");
        assert!(format!("{error:#}").contains("clé"));
    }
}
