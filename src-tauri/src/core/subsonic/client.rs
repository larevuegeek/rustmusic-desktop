//! Client OpenSubsonic (Navidrome, Gonic, Airsonic…) : connexion par empreinte ou clé d'API, réponses JSON.

use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::core::podcast::fetch::client;

const API_VERSION: &str = "1.16.1";
const CLIENT_NAME: &str = "RustMusic";

/// Preuve d'identité envoyée à chaque requête ; le mot de passe lui-même n'est jamais gardé.
#[derive(Debug, Clone, PartialEq)]
pub enum Auth {
    /// `t = md5(mot de passe + s)`, calculé une fois à l'ajout du serveur.
    Token { username: String, token: String, salt: String },
    /// Extension OpenSubsonic `apiKeyAuthentication`.
    ApiKey(String),
}

impl Auth {
    pub fn from_password(username: &str, password: &str) -> Self {
        let salt = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
        let token = format!("{:x}", md5::compute(format!("{password}{salt}")));
        Auth::Token { username: username.to_string(), token, salt }
    }

    fn params(&self) -> Vec<(&'static str, String)> {
        match self {
            Auth::Token { username, token, salt } => vec![("u", username.clone()), ("t", token.clone()), ("s", salt.clone())],
            Auth::ApiKey(key) => vec![("apiKey", key.clone())],
        }
    }
}

/// Un serveur joint : adresse de base (sans `/rest`) et identité.
#[derive(Debug, Clone)]
pub struct Server {
    pub base: String,
    pub auth: Auth,
}

/// Ce que le serveur dit de lui-même.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ServerInfo {
    /// Nom donné par le serveur lui-même (DLNA : « MiniDLNA Docker »).
    #[serde(skip)]
    pub name: Option<String>,
    /// Identifiant réseau (DLNA : UDN), pour le retrouver si son adresse change.
    #[serde(skip)]
    pub device_id: Option<String>,
    /// « navidrome », « gonic »… quand le serveur se nomme.
    pub kind: Option<String>,
    pub version: Option<String>,
    pub open_subsonic: bool,
    pub extensions: Vec<String>,
}

/// Un dossier musical du serveur : une bibliothèque côté RustMusic.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MusicFolder {
    pub id: String,
    pub name: String,
}

/// Adresse saisie → base de l'API : sans `/rest` ni `/` final ; schéma ajouté à part (voir `connect`).
fn trim_base(raw: &str) -> String {
    let mut s = raw.trim().trim_end_matches('/').to_string();
    for suffix in ["/rest", "/app"] {
        if s.to_ascii_lowercase().ends_with(suffix) {
            s.truncate(s.len() - suffix.len());
        }
    }
    s.trim_end_matches('/').to_string()
}

/// Message clair pour une erreur Subsonic (`code` du protocole).
fn explain(code: i64, message: &str) -> String {
    match code {
        40 => "identifiant ou mot de passe incorrect".into(),
        41 => "ce serveur refuse la connexion par mot de passe : utilisez une clé d'API".into(),
        42..=44 => "méthode de connexion refusée par le serveur".into(),
        50 => "accès refusé pour cet utilisateur".into(),
        20 | 30 => "version du serveur incompatible".into(),
        70 => "élément introuvable sur le serveur".into(),
        _ if !message.is_empty() => message.to_string(),
        _ => format!("erreur du serveur ({code})"),
    }
}

/// Le contenu de `subsonic-response`, ou l'erreur qu'il annonce.
const UNREACHABLE: &str = "serveur injoignable";

/// Erreur de transport (serveur éteint, réseau coupé, proxy en panne, téléchargement coupé) :
/// l'action peut être retentée.
pub fn is_unreachable(err: &str) -> bool {
    err.starts_with(UNREACHABLE)
        || err.starts_with("serveur : HTTP")
        || ["injoignable", "téléchargement interrompu", "téléchargement figé"].iter().any(|m| err.contains(m))
}

fn unwrap_response(json: Value) -> Result<Value, String> {
    let Some(resp) = json.get("subsonic-response") else {
        return Err("ce n'est pas un serveur Subsonic".into());
    };
    if resp.get("status").and_then(Value::as_str) == Some("ok") {
        return Ok(resp.clone());
    }
    let err = resp.get("error");
    let code = err.and_then(|e| e.get("code")).and_then(Value::as_i64).unwrap_or(0);
    let message = err.and_then(|e| e.get("message")).and_then(Value::as_str).unwrap_or("");
    Err(explain(code, message))
}

/// Identifiant en texte : numérique chez certains serveurs, chaîne chez d'autres.
pub(crate) fn id_of(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn parse_info(resp: &Value) -> ServerInfo {
    let text = |k: &str| resp.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string);
    ServerInfo {
        name: None,
        device_id: None,
        kind: text("type"),
        version: text("serverVersion").or_else(|| text("version")),
        open_subsonic: resp.get("openSubsonic").and_then(Value::as_bool).unwrap_or(false),
        extensions: Vec::new(),
    }
}

fn parse_folders(resp: &Value) -> Vec<MusicFolder> {
    resp.pointer("/musicFolders/musicFolder")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|f| {
                    let id = id_of(f.get("id")?)?;
                    let name = f.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).unwrap_or(&id).to_string();
                    Some(MusicFolder { id, name })
                })
                .collect()
        })
        .unwrap_or_default()
}

impl Server {
    /// Joint le serveur à l'adresse saisie ; sans schéma, essaie https puis http (réseau local).
    pub async fn connect(raw_url: &str, auth: Auth) -> Result<(Server, ServerInfo), String> {
        let base = trim_base(raw_url);
        if base.is_empty() {
            return Err("adresse du serveur vide".into());
        }
        let candidates = if base.contains("://") { vec![base] } else { vec![format!("https://{base}"), format!("http://{base}")] };
        let mut last = String::new();
        for base in candidates {
            if reqwest::Url::parse(&base).is_err() {
                last = "adresse du serveur invalide".into();
                continue;
            }
            let server = Server { base, auth: auth.clone() };
            match server.ping().await {
                Ok(info) => return Ok((server, info)),
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    pub(crate) fn url(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<reqwest::Url, String> {
        let mut all: Vec<(&str, String)> = self.auth.params();
        all.extend([("v", API_VERSION.to_string()), ("c", CLIENT_NAME.to_string()), ("f", "json".to_string())]);
        all.extend(params.iter().map(|(k, v)| (*k, v.to_string())));
        reqwest::Url::parse_with_params(&format!("{}/rest/{endpoint}", self.base), &all).map_err(|e| e.to_string())
    }

    /// Appel d'une méthode de l'API ; rend le contenu de `subsonic-response`.
    pub async fn call(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<Value, String> {
        let resp = client()
            .get(self.url(endpoint, params)?)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("{UNREACHABLE} : {e}"))?;
        let status = resp.status();
        let json: Value = resp.json().await.map_err(|_| {
            if status.is_success() { "ce n'est pas un serveur Subsonic".to_string() } else { format!("serveur : HTTP {}", status.as_u16()) }
        })?;
        unwrap_response(json)
    }

    /// Vérifie la connexion et l'identité ; complète les extensions OpenSubsonic.
    pub async fn ping(&self) -> Result<ServerInfo, String> {
        let mut info = parse_info(&self.call("ping", &[]).await?);
        if info.open_subsonic {
            if let Ok(resp) = self.call("getOpenSubsonicExtensions", &[]).await {
                info.extensions = resp
                    .get("openSubsonicExtensions")
                    .and_then(Value::as_array)
                    .map(|l| l.iter().filter_map(|e| e.get("name")?.as_str().map(str::to_string)).collect())
                    .unwrap_or_default();
            }
        }
        Ok(info)
    }

    pub async fn music_folders(&self) -> Result<Vec<MusicFolder>, String> {
        Ok(parse_folders(&self.call("getMusicFolders", &[]).await?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_md5_of_password_and_salt() {
        let Auth::Token { username, token, salt } = Auth::from_password("demo", "secret") else { panic!() };
        assert_eq!(username, "demo");
        assert_eq!(salt.len(), 12);
        assert_eq!(token, format!("{:x}", md5::compute(format!("secret{salt}"))));
        assert_ne!(Auth::from_password("demo", "secret"), Auth::from_password("demo", "secret"), "un sel neuf à chaque fois");
    }

    #[test]
    fn trims_the_entered_address() {
        assert_eq!(trim_base(" https://music.example.com/rest/ "), "https://music.example.com");
        assert_eq!(trim_base("http://nas:4533/app/"), "http://nas:4533");
        assert_eq!(trim_base("nas.local:4533"), "nas.local:4533");
    }

    #[test]
    fn reads_responses_and_errors() {
        let ok = serde_json::json!({ "subsonic-response": { "status": "ok", "type": "navidrome", "serverVersion": "0.64.2", "openSubsonic": true,
            "musicFolders": { "musicFolder": [ { "id": 1, "name": "Music Library" }, { "id": "hd", "name": " " } ] } } });
        let resp = unwrap_response(ok).unwrap();
        let info = parse_info(&resp);
        assert_eq!((info.kind.as_deref(), info.version.as_deref(), info.open_subsonic), (Some("navidrome"), Some("0.64.2"), true));
        assert_eq!(parse_folders(&resp), vec![MusicFolder { id: "1".into(), name: "Music Library".into() }, MusicFolder { id: "hd".into(), name: "hd".into() }]);
        let failed = serde_json::json!({ "subsonic-response": { "status": "failed", "error": { "code": 40, "message": "Wrong username or password" } } });
        assert_eq!(unwrap_response(failed).unwrap_err(), "identifiant ou mot de passe incorrect");
        assert!(unwrap_response(serde_json::json!({ "hello": 1 })).is_err());
    }

    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_demo_server() {
        let (server, info) = Server::connect("demo.navidrome.org", Auth::from_password("demo", "demo")).await.unwrap();
        let folders = server.music_folders().await.unwrap();
        println!("{} : {info:?} ; {folders:?}", server.base);
        assert!(server.base.starts_with("https://"));
        assert!(!folders.is_empty());
        let wrong = Server::connect("https://demo.navidrome.org", Auth::from_password("demo", "faux")).await;
        assert_eq!(wrong.unwrap_err(), "identifiant ou mot de passe incorrect");
    }
}
