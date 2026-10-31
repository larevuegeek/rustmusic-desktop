//! Client Last.fm : connexion par le navigateur (`auth.getToken` → autorisation → `auth.getSession`),
//! écoutes (`track.scrobble`, 50 par envoi) et morceau en cours (`track.updateNowPlaying`).
//!
//! Chaque appel est signé avec le secret d'une clé d'API : celle de RustMusic, injectée à la
//! compilation (variables `RUSTMUSIC_LASTFM_API_KEY` / `_SECRET` ou `keys/lastfm.local`, voir build.rs ;
//! jamais dans le dépôt), ou celle de l'utilisateur.

use std::time::Duration;

use serde_json::Value;

use crate::core::podcast::fetch::client;
use crate::core::scrobble::listenbrainz::{Listen, SendError};

const API: &str = "https://ws.audioscrobbler.com/2.0/";
const AUTH_PAGE: &str = "https://www.last.fm/api/auth/";
/// Au plus 50 écoutes par envoi (limite du service).
pub const BATCH: usize = 50;

/// Clé d'API et son secret.
#[derive(Debug, Clone, PartialEq)]
pub struct Credentials {
    pub key: String,
    pub secret: String,
}

/// La clé de RustMusic, si cette compilation en a reçu une.
pub fn builtin() -> Option<Credentials> {
    match (option_env!("RUSTMUSIC_LASTFM_API_KEY"), option_env!("RUSTMUSIC_LASTFM_API_SECRET")) {
        (Some(key), Some(secret)) if !key.trim().is_empty() && !secret.trim().is_empty() => {
            Some(Credentials { key: key.trim().into(), secret: secret.trim().into() })
        }
        _ => None,
    }
}

#[derive(Debug, PartialEq)]
pub enum ApiError {
    /// Réseau, délai dépassé, réponse illisible.
    Network(String),
    /// Erreur rendue par Last.fm : son code et son message.
    Service(i64, String),
}

/// `api_sig` : paramètres triés, clés et valeurs bout à bout (sauf `format`), puis le secret, en MD5.
pub fn sign(params: &[(String, String)], secret: &str) -> String {
    let mut sorted: Vec<&(String, String)> = params.iter().filter(|(k, _)| k != "format" && k != "callback").collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut s = String::new();
    for (k, v) in sorted {
        s.push_str(k);
        s.push_str(v);
    }
    s.push_str(secret);
    format!("{:x}", md5::compute(s.as_bytes()))
}

async fn call(creds: &Credentials, mut params: Vec<(String, String)>) -> Result<Value, ApiError> {
    params.push(("api_key".into(), creds.key.clone()));
    let sig = sign(&params, &creds.secret);
    params.push(("api_sig".into(), sig));
    params.push(("format".into(), "json".into()));
    let body = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    let net = |e: reqwest::Error| ApiError::Network(e.to_string());
    let resp = client()
        .post(API)
        .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(net)?;
    let v: Value = resp.json().await.map_err(net)?;
    if let Some(code) = v.get("error").and_then(Value::as_i64) {
        return Err(ApiError::Service(code, v.get("message").and_then(Value::as_str).unwrap_or_default().to_string()));
    }
    Ok(v)
}

fn p(k: &str, v: impl Into<String>) -> (String, String) {
    (k.to_string(), v.into())
}

/// Premier temps de la connexion : un jeton à faire autoriser dans le navigateur.
pub async fn get_token(creds: &Credentials) -> Result<String, ApiError> {
    let v = call(creds, vec![p("method", "auth.getToken")]).await?;
    v.get("token").and_then(Value::as_str).map(str::to_string).ok_or_else(|| ApiError::Network("jeton absent".into()))
}

/// La page où l'utilisateur autorise RustMusic.
pub fn auth_url(creds: &Credentials, token: &str) -> String {
    format!("{AUTH_PAGE}?api_key={}&token={}", urlencoding::encode(&creds.key), urlencoding::encode(token))
}

/// Second temps : une fois autorisé, la clé de session et le nom du compte.
/// Code 14 tant que l'utilisateur n'a pas encore cliqué « Autoriser ».
pub async fn get_session(creds: &Credentials, token: &str) -> Result<(String, String), ApiError> {
    let v = call(creds, vec![p("method", "auth.getSession"), p("token", token)]).await?;
    let s = &v["session"];
    match (s.get("name").and_then(Value::as_str), s.get("key").and_then(Value::as_str)) {
        (Some(name), Some(key)) => Ok((name.to_string(), key.to_string())),
        _ => Err(ApiError::Network("session absente".into())),
    }
}

/// Paramètres d'une écoute ; `index` pour les envois groupés (`artist[0]`…).
fn track_params(l: &Listen, index: Option<usize>) -> Vec<(String, String)> {
    let key = |k: &str| match index {
        Some(i) => format!("{k}[{i}]"),
        None => k.to_string(),
    };
    let mut out = vec![(key("artist"), l.artist.clone()), (key("track"), l.title.clone())];
    if let Some(a) = &l.album {
        out.push((key("album"), a.clone()));
    }
    if let Some(ms) = l.duration_ms {
        out.push((key("duration"), (ms / 1000).to_string()));
    }
    if index.is_some() {
        out.push((key("timestamp"), l.listened_at.to_string()));
    }
    out
}

/// Données refusées : on écarte. Le reste (réseau, panne, session ou clé à revoir) : on réessaiera.
fn send_error(e: ApiError) -> SendError {
    match e {
        ApiError::Service(6 | 7, m) => SendError::Rejected(m),
        ApiError::Service(code, m) => SendError::Retry(format!("Last.fm {code} : {m}")),
        ApiError::Network(m) => SendError::Retry(m),
    }
}

pub async fn scrobble(creds: &Credentials, session: &str, listens: &[Listen]) -> Result<(), SendError> {
    let mut params = vec![p("method", "track.scrobble"), p("sk", session)];
    for (i, l) in listens.iter().enumerate() {
        params.extend(track_params(l, Some(i)));
    }
    call(creds, params).await.map(|_| ()).map_err(send_error)
}

pub async fn now_playing(creds: &Credentials, session: &str, l: &Listen) -> Result<(), SendError> {
    let mut params = vec![p("method", "track.updateNowPlaying"), p("sk", session)];
    params.extend(track_params(l, None));
    call(creds, params).await.map(|_| ()).map_err(send_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_matches_the_last_fm_rule() {
        // Valeurs de référence calculées à part (Python hashlib).
        let params = vec![p("method", "auth.getToken"), p("api_key", "abc"), p("format", "json")];
        assert_eq!(sign(&params, "s3cr3t"), "07891ad0f3e704b2a937e8150e1246ce", "format est exclu, l'ordre est alphabétique");

        let l = Listen { listened_at: 1_700_000_000, artist: "Queen".into(), title: "Bohemian".into(), album: Some("Opéra".into()), duration_ms: None };
        let mut params = vec![p("method", "track.scrobble"), p("sk", "sess"), p("api_key", "abc")];
        params.extend(track_params(&l, Some(0)));
        assert_eq!(sign(&params, "s3cr3t"), "a495b9ee0797f9dd0b2e818ba9684eff", "les [0] et l'UTF-8 sont signés tels quels");
    }

    #[test]
    fn scrobble_and_now_playing_params() {
        let l = Listen { listened_at: 42, artist: "A".into(), title: "T".into(), album: None, duration_ms: Some(215_900) };
        let indexed = track_params(&l, Some(3));
        assert!(indexed.contains(&p("timestamp[3]", "42")));
        assert!(indexed.contains(&p("duration[3]", "215")), "durée en secondes");
        let now = track_params(&l, None);
        assert!(now.iter().all(|(k, _)| k != "timestamp"), "le morceau en cours n'a pas d'heure");
    }

    #[test]
    fn errors_split_between_rejected_and_retry() {
        assert!(matches!(send_error(ApiError::Service(6, "x".into())), SendError::Rejected(_)));
        assert!(matches!(send_error(ApiError::Service(9, "session".into())), SendError::Retry(_)), "session révoquée : on garde pour après la reconnexion");
        assert!(matches!(send_error(ApiError::Service(29, "quota".into())), SendError::Retry(_)));
        assert!(matches!(send_error(ApiError::Network("dns".into())), SendError::Retry(_)));
    }

    /// Le vrai serveur, avec une fausse clé : la requête doit être comprise et refusée pour sa clé (10).
    /// `cargo test --lib lastfm_server -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn lastfm_server_understands_our_requests() {
        let fake = Credentials { key: "0123456789abcdef0123456789abcdef".into(), secret: "x".into() };
        assert!(matches!(get_token(&fake).await, Err(ApiError::Service(10, _))), "{:?}", get_token(&fake).await);
    }

    #[test]
    fn auth_page_carries_the_key_and_token() {
        let c = Credentials { key: "k e y".into(), secret: "s".into() };
        assert_eq!(auth_url(&c, "tok"), "https://www.last.fm/api/auth/?api_key=k%20e%20y&token=tok");
    }
}
