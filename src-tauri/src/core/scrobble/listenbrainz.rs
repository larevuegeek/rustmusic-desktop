//! Client ListenBrainz : jeton personnel (Réglages du compte sur listenbrainz.org), écoutes et « en cours ».

use std::time::Duration;

use serde_json::{json, Value};

use crate::core::podcast::fetch::client;

const API: &str = "https://api.listenbrainz.org/1";
/// Au plus 1000 écoutes par envoi (limite du service).
pub const BATCH: usize = 1000;

/// Une écoute à envoyer.
#[derive(Debug, Clone, PartialEq)]
pub struct Listen {
    /// Début de l'écoute, en secondes Unix.
    pub listened_at: i64,
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
}

/// Réponse du service : réessayer plus tard (réseau, panne) ou abandonner (refus définitif).
#[derive(Debug, PartialEq)]
pub enum SendError {
    Retry(String),
    Rejected(String),
}

fn metadata(l: &Listen) -> Value {
    let mut info = json!({ "submission_client": "RustMusic", "submission_client_version": env!("CARGO_PKG_VERSION"), "media_player": "RustMusic" });
    if let Some(ms) = l.duration_ms {
        info["duration_ms"] = json!(ms);
    }
    let mut m = json!({ "artist_name": l.artist, "track_name": l.title, "additional_info": info });
    if let Some(a) = &l.album {
        m["release_name"] = json!(a);
    }
    m
}

/// Corps d'un envoi : `single` (une écoute), `import` (plusieurs) ou `playing_now` (sans date).
pub fn payload(kind: &str, listens: &[Listen]) -> Value {
    let items: Vec<Value> = listens
        .iter()
        .map(|l| {
            let mut item = json!({ "track_metadata": metadata(l) });
            if kind != "playing_now" {
                item["listened_at"] = json!(l.listened_at);
            }
            item
        })
        .collect();
    json!({ "listen_type": kind, "payload": items })
}

/// Vérifie le jeton ; rend le nom d'utilisateur. Erreur : un code traduit par l'interface
/// (`unreachable`, `token_refused`, `no_account`).
pub async fn validate(token: &str) -> Result<String, String> {
    let unreachable = |e: reqwest::Error| {
        log::warn!("🎧 ListenBrainz injoignable : {e}");
        "unreachable".to_string()
    };
    let resp = client()
        .get(format!("{API}/validate-token"))
        .header("Authorization", format!("Token {}", token.trim()))
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(unreachable)?;
    let v: Value = resp.json().await.map_err(unreachable)?;
    if v.get("valid").and_then(Value::as_bool) != Some(true) {
        return Err("token_refused".into());
    }
    v.get("user_name").and_then(Value::as_str).map(str::to_string).ok_or_else(|| "no_account".into())
}

pub async fn submit(token: &str, body: &Value) -> Result<(), SendError> {
    let resp = client()
        .post(format!("{API}/submit-listens"))
        .header("Authorization", format!("Token {token}"))
        .json(body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| SendError::Retry(e.to_string()))?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let text = resp.text().await.unwrap_or_default();
    // 429 et 5xx : plus tard ; 401 : jeton révoqué, on garde pour après la reconnexion ; 400 : données refusées.
    if status.as_u16() == 400 {
        Err(SendError::Rejected(text))
    } else {
        Err(SendError::Retry(format!("HTTP {} {text}", status.as_u16())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_payloads() {
        let l = Listen { listened_at: 1_700_000_000, artist: "Queen".into(), title: "Bohemian Rhapsody".into(), album: Some("A Night at the Opera".into()), duration_ms: Some(354_000) };
        let single = payload("single", std::slice::from_ref(&l));
        assert_eq!(single["listen_type"], "single");
        assert_eq!(single["payload"][0]["listened_at"], 1_700_000_000);
        assert_eq!(single["payload"][0]["track_metadata"]["release_name"], "A Night at the Opera");
        assert_eq!(single["payload"][0]["track_metadata"]["additional_info"]["duration_ms"], 354_000);
        let now = payload("playing_now", &[l]);
        assert!(now["payload"][0].get("listened_at").is_none());
    }
}
