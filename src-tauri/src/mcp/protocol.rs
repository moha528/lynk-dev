//! Enveloppe JSON-RPC 2.0 et négociation MCP.
//!
//! Module **pur** : aucune entrée/sortie, aucun état. C'est ce qui permet de
//! vérifier la forme des messages sans lever de serveur.

use serde::Deserialize;
use serde_json::{json, Value};

/// Révision du protocole que ce serveur annonce.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// Révisions acceptées d'un client. On répond dans **sa** révision quand elle
/// est connue : un client qui parle une version antérieure n'a pas à être
/// éconduit tant que les messages échangés sont identiques.
pub const SUPPORTED_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

// Codes d'erreur JSON-RPC 2.0.
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;

/// Un message entrant.
#[derive(Debug, Deserialize)]
pub struct Request {
    #[serde(default)]
    pub jsonrpc: String,
    /// Absent sur une **notification** — à laquelle on ne répond jamais.
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl Request {
    /// Une notification n'a pas d'identifiant : le client n'attend rien en
    /// retour, et lui répondre est une faute de protocole.
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

pub fn success(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub fn failure(id: Option<Value>, code: i32, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message.into() },
    })
}

/// Réponse à `initialize`.
///
/// La révision demandée est reprise telle quelle si on la connaît ; sinon on
/// annonce la nôtre et c'est au client de décider s'il continue.
pub fn initialize_result(requested: Option<&str>, server_version: &str) -> Value {
    let version = match requested {
        Some(asked) if SUPPORTED_VERSIONS.contains(&asked) => asked,
        _ => PROTOCOL_VERSION,
    };
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": {
            "name": "lynk-dev",
            "title": "Lynk Dev",
            "version": server_version,
        },
        "instructions": "Supervision des services de développement locaux de Lynk Dev. \
    Les outils portent sur le profil actif de la fenêtre ouverte ; aucun n'exécute de commande arbitraire.",
    })
}

/// Résultat d'un appel d'outil. Le texte est rendu tel quel au modèle.
///
/// ⚠️ Un outil qui échoue rend un résultat **`isError`**, pas une erreur
/// JSON-RPC : les erreurs de protocole disent « je n'ai pas compris la
/// demande », celle-ci dit « j'ai compris, et voilà pourquoi ça n'a pas
/// marché » — c'est cette seconde forme que le modèle sait exploiter.
pub fn tool_result(text: impl Into<String>, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text.into() }],
        "isError": is_error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> Request {
        serde_json::from_str(raw).expect("requête")
    }

    #[test]
    fn a_message_without_an_id_is_a_notification() {
        assert!(
            parse(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_notification()
        );
        assert!(!parse(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).is_notification());
    }

    /// L'identifiant peut être un nombre **ou** une chaîne : le refuser sur la
    /// forme couperait la moitié des clients.
    #[test]
    fn an_id_can_be_a_string() {
        let request = parse(r#"{"jsonrpc":"2.0","id":"abc","method":"ping"}"#);
        assert_eq!(request.id, Some(json!("abc")));
        assert!(!request.is_notification());
    }

    #[test]
    fn missing_params_default_to_null() {
        assert_eq!(
            parse(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).params,
            Value::Null
        );
    }

    #[test]
    fn the_requested_protocol_version_is_echoed_when_known() {
        let result = initialize_result(Some("2025-03-26"), "0.1.0");
        assert_eq!(result["protocolVersion"], "2025-03-26");
    }

    #[test]
    fn an_unknown_protocol_version_falls_back_to_ours() {
        let result = initialize_result(Some("1999-01-01"), "0.1.0");
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        let absent = initialize_result(None, "0.1.0");
        assert_eq!(absent["protocolVersion"], PROTOCOL_VERSION);
    }

    #[test]
    fn a_failure_carries_its_code_and_the_request_id() {
        let error = failure(Some(json!(7)), METHOD_NOT_FOUND, "méthode inconnue");
        assert_eq!(error["id"], 7);
        assert_eq!(error["error"]["code"], METHOD_NOT_FOUND);
        assert_eq!(error["error"]["message"], "méthode inconnue");
        assert!(error.get("result").is_none());
    }

    #[test]
    fn a_tool_failure_is_a_result_not_a_protocol_error() {
        let value = tool_result("service introuvable", true);
        assert_eq!(value["isError"], true);
        assert_eq!(value["content"][0]["type"], "text");
        assert_eq!(value["content"][0]["text"], "service introuvable");
    }
}
