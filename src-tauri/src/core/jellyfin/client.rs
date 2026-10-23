//! Client Jellyfin : jeton de session obtenu à la connexion (le mot de passe n'est pas gardé),
//! ou clé d'API créée dans le tableau de bord.

use std::time::Duration;

use serde_json::{json, Value};

use crate::core::podcast::fetch::client;
use crate::core::subsonic::client::{MusicFolder, ServerInfo};

/// Un serveur joint : adresse de base, jeton (ou clé d'API) et utilisateur.
#[derive(Debug, Clone, PartialEq)]
pub struct Server {
    pub base: String,
    pub token: String,
    pub user_id: String,
    /// Le jeton est une clé d'API (sinon un jeton de session).
    pub api_key: bool,
}

/// Identité de l'appli auprès du serveur (affichée dans ses « appareils »).
fn authorization(token: Option<&str>) -> String {
    let host = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_else(|_| "ordinateur".into());
    let device_id = format!("{:x}", md5::compute(format!("rustmusic-{host}")));
    let mut h = format!(
        "MediaBrowser Client=\"RustMusic\", Device=\"{host}\", DeviceId=\"{device_id}\", Version=\"{}\"",
        env!("CARGO_PKG_VERSION")
    );
    if let Some(t) = token {
        h.push_str(&format!(", Token=\"{t}\""));
    }
    h
}

/// Adresse saisie → base : sans `/web` ni `/` final.
fn trim_base(raw: &str) -> String {
    let mut s = raw.trim().trim_end_matches('/').to_string();
    if s.to_ascii_lowercase().ends_with("/web") {
        s.truncate(s.len() - 4);
    }
    s.trim_end_matches('/').to_string()
}

/// Réponse JSON, ou l'erreur qu'elle annonce (formulée comme pour Subsonic : « serveur injoignable »…).
async fn read(resp: Result<reqwest::Response, reqwest::Error>) -> Result<Value, String> {
    let resp = resp.map_err(|e| format!("serveur injoignable : {e}"))?;
    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err("identifiant, mot de passe ou jeton refusé".into());
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err("élément introuvable sur le serveur".into());
    }
    if !status.is_success() {
        return Err(format!("serveur : HTTP {}", status.as_u16()));
    }
    let text = resp.text().await.map_err(|e| format!("serveur injoignable : {e}"))?;
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|_| "ce n'est pas un serveur Jellyfin".into())
}

fn info_of(v: &Value) -> ServerInfo {
    ServerInfo {
        kind: Some("jellyfin".into()),
        name: None,
        device_id: None,
        version: v.get("Version").and_then(Value::as_str).map(str::to_string),
        open_subsonic: false,
        extensions: Vec::new(),
    }
}

impl Server {
    /// Joint le serveur ; `password` (jeton de session) ou `api_key` (et le nom d'utilisateur pour s'y rattacher).
    pub async fn connect(raw_url: &str, username: &str, password: Option<&str>, api_key: Option<&str>) -> Result<(Server, ServerInfo), String> {
        let base = trim_base(raw_url);
        if base.is_empty() {
            return Err("adresse du serveur vide".into());
        }
        let candidates = if base.contains("://") { vec![base] } else { vec![format!("https://{base}"), format!("http://{base}")] };
        let mut last = String::new();
        for base in candidates {
            match Self::login(&base, username, password, api_key).await {
                Ok(found) => return Ok(found),
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    async fn login(base: &str, username: &str, password: Option<&str>, api_key: Option<&str>) -> Result<(Server, ServerInfo), String> {
        reqwest::Url::parse(base).map_err(|_| "adresse du serveur invalide".to_string())?;
        let public = read(client().get(format!("{base}/System/Info/Public")).timeout(Duration::from_secs(10)).send().await).await?;
        if public.get("Id").is_none() {
            return Err("ce n'est pas un serveur Jellyfin".into());
        }
        let info = info_of(&public);
        if let Some(key) = api_key.map(str::trim).filter(|k| !k.is_empty()) {
            // Clé d'API : l'utilisateur est retrouvé par son nom.
            let users = read(client().get(format!("{base}/Users")).header("Authorization", authorization(Some(key))).timeout(Duration::from_secs(10)).send().await).await?;
            let user = users
                .as_array()
                .and_then(|l| l.iter().find(|u| u.get("Name").and_then(Value::as_str).is_some_and(|n| n.eq_ignore_ascii_case(username.trim()))))
                .ok_or("utilisateur introuvable sur ce serveur")?;
            let user_id = user.get("Id").and_then(Value::as_str).ok_or("utilisateur sans identifiant")?.to_string();
            return Ok((Server { base: base.to_string(), token: key.to_string(), user_id, api_key: true }, info));
        }
        let auth = read(
            client()
                .post(format!("{base}/Users/AuthenticateByName"))
                .header("Authorization", authorization(None))
                .json(&json!({ "Username": username.trim(), "Pw": password.unwrap_or("") }))
                .timeout(Duration::from_secs(15))
                .send()
                .await,
        )
        .await?;
        let token = auth.get("AccessToken").and_then(Value::as_str).ok_or("le serveur n'a pas rendu de jeton")?;
        let user_id = auth.pointer("/User/Id").and_then(Value::as_str).ok_or("le serveur n'a pas rendu l'utilisateur")?;
        Ok((Server { base: base.to_string(), token: token.to_string(), user_id: user_id.to_string(), api_key: false }, info))
    }

    /// Adresse complète, jeton en paramètre (flux, images : sans en-tête possible côté lecteur).
    pub(crate) fn url(&self, path: &str, params: &[(&str, &str)]) -> Result<reqwest::Url, String> {
        let mut all: Vec<(&str, &str)> = params.to_vec();
        all.push(("api_key", &self.token));
        reqwest::Url::parse_with_params(&format!("{}{path}", self.base), &all).map_err(|e| e.to_string())
    }

    pub(crate) async fn get(&self, path: &str, params: &[(&str, &str)]) -> Result<Value, String> {
        let url = reqwest::Url::parse_with_params(&format!("{}{path}", self.base), params).map_err(|e| e.to_string())?;
        read(client().get(url).header("Authorization", authorization(Some(&self.token))).timeout(Duration::from_secs(30)).send().await).await
    }

    pub(crate) async fn send(&self, method: reqwest::Method, path: &str, params: &[(&str, &str)], body: Option<Value>) -> Result<Value, String> {
        let url = reqwest::Url::parse_with_params(&format!("{}{path}", self.base), params).map_err(|e| e.to_string())?;
        let mut req = client().request(method, url).header("Authorization", authorization(Some(&self.token))).timeout(Duration::from_secs(30));
        if let Some(b) = body {
            req = req.json(&b);
        }
        read(req.send().await).await
    }

    /// Vérifie la connexion et le jeton.
    pub async fn ping(&self) -> Result<ServerInfo, String> {
        Ok(info_of(&self.get("/System/Info", &[]).await?))
    }

    /// Les bibliothèques musicales de l'utilisateur.
    pub async fn music_folders(&self) -> Result<Vec<MusicFolder>, String> {
        let views = self.get(&format!("/Users/{}/Views", self.user_id), &[]).await?;
        Ok(views["Items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|v| v.get("CollectionType").and_then(Value::as_str) == Some("music"))
            .filter_map(|v| {
                Some(MusicFolder { id: v.get("Id")?.as_str()?.to_string(), name: v.get("Name").and_then(Value::as_str).unwrap_or("Musique").to_string() })
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_the_entered_address() {
        assert_eq!(trim_base(" http://nas:8096/web/ "), "http://nas:8096");
        assert_eq!(trim_base("jellyfin.example.com/"), "jellyfin.example.com");
    }

    #[test]
    fn identifies_the_app() {
        let h = authorization(Some("abc"));
        assert!(h.starts_with("MediaBrowser Client=\"RustMusic\""));
        assert!(h.ends_with("Token=\"abc\""));
    }

    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_demo_login() {
        let (server, info) = Server::connect("demo.jellyfin.org/stable", "demo", Some(""), None).await.unwrap();
        let folders = server.music_folders().await.unwrap();
        println!("Jellyfin {:?} ; utilisateur {} ; bibliothèques {folders:?}", info.version, server.user_id);
        assert!(!folders.is_empty());
        assert!(server.ping().await.is_ok());
    }
}
