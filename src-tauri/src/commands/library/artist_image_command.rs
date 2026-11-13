//! Portrait d'un artiste choisi par l'utilisateur : image du disque, image trouvée en ligne (Deezer, Wikipédia), ou retiré.
//! Les passes Deezer automatiques ne cherchent que les artistes sans portrait : ce choix n'est pas écrasé.

use serde::Serialize;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, State};

use crate::commands::library::library_command::portraits_dir;
use crate::helper::library::image_fetch::download;
use crate::helper::library::thumbnail_helper::save_artist_image;
use crate::repository::artist::artist_repository::ArtistRepository;
use crate::state::AppState;

/// Wikimedia demande un client identifié ; aucune donnée personnelle.
const USER_AGENT: &str = concat!("RustMusic/", env!("CARGO_PKG_VERSION"), " (https://rustmusic.dev)");

#[derive(Debug, Serialize)]
pub struct PortraitCandidate {
    /// "deezer" | "wikipedia"
    pub source: &'static str,
    pub title: String,
    /// Fans (Deezer) ou description de l'article (Wikipédia).
    pub detail: Option<String>,
    pub thumb: String,
    pub full: String,
}

fn client(seconds: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(seconds))
        .build()
        .map_err(|e| e.to_string())
}

/// Enregistre l'image (miniatures comprises), met la base à jour et prévient l'interface.
async fn store(app: &AppHandle, pool: &SqlitePool, artist_id: &str, bytes: &[u8]) -> Result<String, String> {
    image::guess_format(bytes).map_err(|_| "not_an_image".to_string())?;
    // Nom neuf à chaque fois : l'ancien fichier reste en cache dans l'interface.
    let name = format!("artist_{}_{}.jpg", artist_id.replace('-', ""), chrono::Utc::now().timestamp_millis());
    let path = save_artist_image(&portraits_dir(), &name, bytes)?;
    ArtistRepository::update_image_url(pool, artist_id, &path).await.map_err(|e| e.to_string())?;
    let _ = app.emit("artist-image-ready", serde_json::json!({ "artist_id": artist_id, "image_url": path }));
    Ok(path)
}

#[tauri::command]
pub async fn set_artist_image(app: AppHandle, state: State<'_, AppState>, artist_id: String, image_path: String) -> Result<String, String> {
    let bytes = std::fs::read(&image_path).map_err(|e| format!("{image_path} : {e}"))?;
    store(&app, &state.pool, &artist_id, &bytes).await
}

/// Une image choisie parmi les résultats de `search_artist_portraits`.
#[tauri::command]
pub async fn apply_artist_image_url(app: AppHandle, state: State<'_, AppState>, artist_id: String, url: String) -> Result<String, String> {
    if !url.starts_with("https://") {
        return Err("bad_url".into());
    }
    let bytes = download(&client(15)?, &url).await.ok_or_else(|| "download_failed".to_string())?;
    store(&app, &state.pool, &artist_id, &bytes).await
}

/// Sans portrait : "" (cherché sans résultat), que les passes Deezer ne reprennent pas, sauf « Relancer ».
#[tauri::command]
pub async fn clear_artist_image(app: AppHandle, state: State<'_, AppState>, artist_id: String) -> Result<(), String> {
    ArtistRepository::update_image_url(&state.pool, &artist_id, "").await.map_err(|e| e.to_string())?;
    let _ = app.emit("artist-image-ready", serde_json::json!({ "artist_id": artist_id, "image_url": "" }));
    Ok(())
}

/// Portraits possibles : Deezer, puis Wikipédia dans la langue de l'interface et en anglais.
/// Une source qui ne répond pas est ignorée ; les deux en panne, c'est une erreur.
#[tauri::command]
pub async fn search_artist_portraits(query: String, lang: Option<String>) -> Result<Vec<PortraitCandidate>, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(vec![]);
    }
    let lang = lang.filter(|l| l.len() <= 3 && l.chars().all(|c| c.is_ascii_lowercase())).unwrap_or_else(|| "en".into());
    let http = client(8)?;
    let mut langs = vec![lang.clone()];
    if lang != "en" {
        langs.push("en".into());
    }
    let (deezer, wiki) = tokio::join!(deezer(&http, &query), async {
        let mut all = Vec::new();
        let mut failed = 0;
        for l in &langs {
            match wikipedia(&http, l, &query).await {
                Ok(found) => all.extend(found),
                Err(e) => {
                    failed += 1;
                    log::warn!("[portraits] Wikipédia {l} : {e}");
                }
            }
        }
        if failed == langs.len() { Err("wikipedia".to_string()) } else { Ok(all) }
    });
    if let (Err(a), Err(b)) = (&deezer, &wiki) {
        return Err(format!("{a} ; {b}"));
    }
    let mut out = deezer.unwrap_or_default();
    // Une même image (article présent dans les deux langues) une seule fois.
    for c in wiki.unwrap_or_default() {
        if !out.iter().any(|o| o.full == c.full) {
            out.push(c);
        }
    }
    Ok(out)
}

async fn deezer(http: &reqwest::Client, query: &str) -> Result<Vec<PortraitCandidate>, String> {
    let wanted = query.to_lowercase();
    let url = format!("https://api.deezer.com/search/artist?q={}&limit=12", urlencoding::encode(query));
    let json: serde_json::Value = http.get(&url).send().await.map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    let mut found: Vec<(bool, u64, PortraitCandidate)> = json["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let thumb = a["picture_medium"].as_str()?.to_string();
            let full = a["picture_xl"].as_str().or(a["picture_big"].as_str())?.to_string();
            // L'image par défaut de Deezer (silhouette) : pas un portrait.
            if full.contains("/artist//") {
                return None;
            }
            let title = a["name"].as_str().unwrap_or_default().to_string();
            let fans = a["nb_fan"].as_u64().unwrap_or(0);
            let candidate = PortraitCandidate { source: "deezer", detail: Some(fans.to_string()), title, thumb, full };
            Some((candidate.title.to_lowercase() == wanted, fans, candidate))
        })
        .collect();
    // Deezer classe mal les homonymes : le nom exact d'abord, puis les plus suivis.
    found.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    Ok(found.into_iter().map(|(_, _, c)| c).collect())
}

/// Mot ajouté à la recherche Wikipédia : sans lui, « Muse » donne aussi les Muses de la mythologie.
fn music_hint(lang: &str) -> &'static str {
    match lang {
        "fr" => "musique",
        "es" => "música",
        "de" => "Musik",
        "it" => "musica",
        _ => "music",
    }
}

async fn wikipedia(http: &reqwest::Client, lang: &str, query: &str) -> Result<Vec<PortraitCandidate>, String> {
    let url = format!(
        "https://{lang}.wikipedia.org/w/api.php?action=query&format=json&formatversion=2&generator=search&gsrsearch={}&gsrlimit=4\
         &prop=pageimages|description&piprop=thumbnail|original&pithumbsize=400",
        urlencoding::encode(&format!("{query} {}", music_hint(lang)))
    );
    let json: serde_json::Value = http.get(&url).send().await.map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    let mut pages: Vec<&serde_json::Value> = json["query"]["pages"].as_array().into_iter().flatten().collect();
    pages.sort_by_key(|p| p["index"].as_u64().unwrap_or(u64::MAX));
    Ok(pages
        .into_iter()
        .filter_map(|p| {
            let full = p["original"]["source"].as_str()?.to_string();
            // Logos et pictogrammes en SVG : pas des photos.
            if full.to_lowercase().ends_with(".svg") {
                return None;
            }
            Some(PortraitCandidate {
                source: "wikipedia",
                title: p["title"].as_str().unwrap_or_default().to_string(),
                detail: p["description"].as_str().map(str::to_string),
                thumb: p["thumbnail"]["source"].as_str().unwrap_or(&full).to_string(),
                full,
            })
        })
        .collect())
}
