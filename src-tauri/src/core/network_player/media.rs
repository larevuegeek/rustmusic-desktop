//! Serveur HTTP qui sert au lecteur réseau les fichiers de la lecture en cours.
//!
//! Il ne répond qu'à des jetons aléatoires enregistrés par la session : jamais à
//! un chemin de fichier. Indépendant du serveur DLNA de bibliothèque, qu'on peut
//! laisser éteint.

use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use axum::body::Body;
use axum::extract::{Path as UrlPath, Request};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::core::dlna_server::library::sqlite_provider::guess_mime_from_extension;

/// Port préféré (règle de pare-feu stable) ; un port libre sinon.
const PREFERRED_PORT: u16 = 8201;
/// Jetons gardés : la file récente suffit, les plus anciens sont oubliés.
const MAX_ENTRIES: usize = 64;
/// Pochettes gardées en mémoire : celles des dernières pistes seulement.
const MAX_COVERS: usize = 8;
/// Seek à l'octet et diffusion en continu, annoncés au lecteur (mêmes drapeaux que le serveur DLNA).
pub use crate::core::dlna_server::content_directory::didl::DLNA_FEATURES as DLNA_FLAGS;

#[derive(Clone)]
struct Entry {
    token: String,
    path: PathBuf,
    mime: String,
    /// Image et son type, servis à `/art/{jeton}`.
    cover: Option<std::sync::Arc<(Vec<u8>, String)>>,
}

static ENTRIES: Mutex<Vec<Entry>> = Mutex::new(Vec::new());
static PORT: OnceLock<u16> = OnceLock::new();

fn entries() -> std::sync::MutexGuard<'static, Vec<Entry>> {
    ENTRIES.lock().unwrap_or_else(|e| e.into_inner())
}

/// Ce que le lecteur doit charger pour un fichier.
#[derive(Debug, Clone)]
pub struct Published {
    pub url: String,
    pub cover_url: Option<String>,
    pub mime: String,
}

/// Démarre le serveur au premier appel ; rend son port.
pub async fn ensure_started() -> Result<u16, String> {
    if let Some(port) = PORT.get() {
        return Ok(*port);
    }
    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", PREFERRED_PORT)).await {
        Ok(l) => l,
        Err(_) => tokio::net::TcpListener::bind(("0.0.0.0", 0))
            .await
            .map_err(|e| format!("serveur de diffusion : {e}"))?,
    };
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    if PORT.set(port).is_err() {
        // Un autre appel l'a démarré entre-temps : on garde le sien.
        return Ok(*PORT.get().unwrap());
    }
    let app = Router::new()
        .route("/stream/{token}", get(stream))
        .route("/art/{token}", get(art));
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            log::error!("Serveur de diffusion arrêté : {e}");
        }
    });
    log::info!("📡 Serveur de diffusion réseau : port {port}");
    Ok(port)
}

/// Rend une adresse que `host` (notre IP vue du lecteur) peut servir pour `path`.
pub async fn publish(path: &Path, cover: Option<(Vec<u8>, String)>, host: Ipv4Addr) -> Result<Published, String> {
    let port = ensure_started().await?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let mime = guess_mime_from_extension(&ext);

    let has_cover = cover.is_some();
    let cover = cover.map(std::sync::Arc::new);
    // Jeton neuf à chaque publication : la même piste rejouée (répéter une)
    // doit avoir une autre adresse, sinon l'enchaînement est indiscernable.
    let token = {
        let mut list = entries();
        let token = uuid::Uuid::new_v4().simple().to_string();
        list.push(Entry { token: token.clone(), path: path.to_path_buf(), mime: mime.clone(), cover });
        if list.len() > MAX_ENTRIES {
            list.remove(0);
        }
        let keep_from = list.len().saturating_sub(MAX_COVERS);
        for old in &mut list[..keep_from] {
            old.cover = None;
        }
        token
    };

    // L'extension dans l'adresse rassure les lecteurs qui la regardent.
    let base = format!("http://{host}:{port}");
    Ok(Published {
        url: format!("{base}/stream/{token}.{ext}"),
        cover_url: has_cover.then(|| format!("{base}/art/{token}")),
        mime,
    })
}

fn find(token: &str) -> Option<Entry> {
    let token = token.split('.').next().unwrap_or(token);
    entries().iter().find(|e| e.token == token).cloned()
}

async fn stream(UrlPath(token): UrlPath<String>, request: Request<Body>) -> Response {
    let Some(entry) = find(&token) else { return StatusCode::NOT_FOUND.into_response() };
    log::debug!(
        "📡 Lecteur réseau : {} (plage {})",
        entry.path.display(),
        request.headers().get(header::RANGE).and_then(|v| v.to_str().ok()).unwrap_or("aucune")
    );
    let mut response = serve(&entry.path, request).await;
    let headers = response.headers_mut();
    if let Ok(v) = HeaderValue::from_str(&entry.mime) {
        headers.insert(header::CONTENT_TYPE, v);
    }
    headers.insert("transferMode.dlna.org", HeaderValue::from_static("Streaming"));
    headers.insert("contentFeatures.dlna.org", HeaderValue::from_static(DLNA_FLAGS));
    response
}

async fn art(UrlPath(token): UrlPath<String>) -> Response {
    match find(&token).and_then(|e| e.cover) {
        Some(cover) => ([(header::CONTENT_TYPE, cover.1.clone())], cover.0.clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Fichier servi avec plages, dates et `HEAD` (tower-http).
async fn serve(path: &Path, request: Request<Body>) -> Response {
    match ServeFile::new(path).oneshot(request).await {
        Ok(resp) => resp.map(Body::new).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn serves_only_published_files_with_dlna_headers() {
        let dir = std::env::temp_dir().join(format!("rm-media-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("piste.flac");
        std::fs::write(&file, b"fLaC0123456789").unwrap();

        let p = publish(&file, None, Ipv4Addr::LOCALHOST).await.unwrap();
        assert!(p.url.ends_with(".flac"));
        assert_eq!(p.mime, "audio/flac");
        // Republié (répéter une) : nouvelle adresse, l'ancienne reste servie.
        let again = publish(&file, None, Ipv4Addr::LOCALHOST).await.unwrap();
        assert_ne!(again.url, p.url);

        let client = reqwest::Client::new();
        assert_eq!(client.get(&again.url).send().await.unwrap().status(), 200);
        let r = client.get(&p.url).header("Range", "bytes=4-7").send().await.unwrap();
        assert_eq!(r.status(), 206);
        assert_eq!(r.headers()["content-type"], "audio/flac");
        assert_eq!(r.headers()["transfermode.dlna.org"], "Streaming");
        assert_eq!(r.bytes().await.unwrap().as_ref(), b"0123");

        let port = ensure_started().await.unwrap();
        let missing = client.get(format!("http://127.0.0.1:{port}/stream/inconnu.flac")).send().await.unwrap();
        assert_eq!(missing.status(), 404);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
