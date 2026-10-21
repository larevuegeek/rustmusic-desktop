//! Radios : réglage d'activation, favoris, logos en cache.
//! Coupées dans les réglages, rien ne part sur le réseau.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use sqlx::SqlitePool;

use crate::core::podcast::fetch::client;
use crate::core::radio::directory::RadioResult;
use crate::entity::radio::radio::RadioView;
use crate::helper::library::image_fetch;
use crate::repository::radio::radio_repository::RadioRepository;

/// Réglage « Radios » : désactivé par défaut.
pub const ENABLED_KEY: &str = "radios";
/// Côté le plus long d'un logo gardé.
const LOGO_SIDE: u32 = 256;
const LOGO_MAX_BYTES: usize = 3 * 1024 * 1024;

static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn ensure_enabled() -> Result<(), String> {
    if ENABLED.load(Ordering::Relaxed) { Ok(()) } else { Err("radios désactivées dans les réglages".into()) }
}

/// Logos : `covers/radios/<md5 de l'adresse>.png`, partagés entre profils et annuaire.
fn logos_dir() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_default();
    dir.push("com.larevuegeek.rustmusic");
    dir.push("covers");
    dir.push("radios");
    dir
}

/// Miniature Wikimedia ramenée à une taille standard : les autres (« 1200px- ») sont refusées (HTTP 400).
fn standard_thumb(url: &str) -> String {
    if !url.contains("upload.wikimedia.org/") || !url.contains("/thumb/") {
        return url.to_string();
    }
    let Some((dir, file)) = url.rsplit_once('/') else { return url.to_string() };
    match file.split_once("px-") {
        Some((size, rest)) if size.ends_with(|c: char| c.is_ascii_digit()) => {
            // « langfr-1280px-… » : le préfixe de langue reste.
            let prefix = size.trim_end_matches(|c: char| c.is_ascii_digit());
            format!("{dir}/{prefix}330px-{rest}")
        }
        _ => url.to_string(),
    }
}

/// Logo de la station en cache (PNG, transparence gardée) ; `None` s'il est introuvable ou illisible.
pub async fn logo(url: &str) -> Option<String> {
    // Échecs retenus pour la session : un logo cassé n'est pas redemandé à chaque affichage.
    static FAILED: Mutex<Option<HashSet<String>>> = Mutex::new(None);
    if !url.starts_with("http") {
        return None;
    }
    let path = logos_dir().join(format!("{:x}.png", md5::compute(url)));
    if path.exists() {
        return Some(path.to_string_lossy().into_owned());
    }
    if FAILED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashSet::new).contains(url) {
        return None;
    }
    let saved = async {
        let bytes = tokio::time::timeout(Duration::from_secs(10), image_fetch::download(client(), &standard_thumb(url))).await.ok()??;
        if bytes.len() > LOGO_MAX_BYTES {
            return None;
        }
        let target = path.clone();
        tokio::task::spawn_blocking(move || {
            let img = image::load_from_memory(&bytes).ok()?;
            let img = if img.width() > LOGO_SIDE || img.height() > LOGO_SIDE { img.thumbnail(LOGO_SIDE, LOGO_SIDE) } else { img };
            std::fs::create_dir_all(target.parent()?).ok()?;
            img.save_with_format(&target, image::ImageFormat::Png).ok()
        })
        .await
        .ok()?
    }
    .await;
    if saved.is_some() {
        Some(path.to_string_lossy().into_owned())
    } else {
        FAILED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashSet::new).insert(url.to_string());
        None
    }
}

/// Ajoute la station aux favoris du profil, logo compris ; déjà là, la rend telle quelle.
pub async fn add(pool: &SqlitePool, profil_id: i64, station: &RadioResult) -> Result<RadioView, String> {
    let id = RadioRepository::add(pool, profil_id, station).await.map_err(|e| e.to_string())?;
    let current = RadioRepository::find_one(pool, id).await.map_err(|e| e.to_string())?.ok_or("radio introuvable")?;
    if current.logo_path.is_none() {
        if let Some(path) = match station.favicon.as_deref() {
            Some(url) => logo(url).await,
            None => None,
        } {
            let _ = RadioRepository::set_logo(pool, id, &path).await;
        }
    }
    RadioRepository::find_one(pool, id).await.map_err(|e| e.to_string())?.ok_or_else(|| "radio introuvable".into())
}

#[cfg(test)]
mod tests {
    #[test]
    fn standard_wikimedia_thumbs() {
        let a = "https://upload.wikimedia.org/wikipedia/commons/thumb/f/fb/Logo.svg/langfr-1280px-Logo.svg.png";
        assert_eq!(super::standard_thumb(a), "https://upload.wikimedia.org/wikipedia/commons/thumb/f/fb/Logo.svg/langfr-330px-Logo.svg.png");
        let b = "https://upload.wikimedia.org/wikipedia/fr/thumb/d/d5/FIP.svg/1024px-FIP.svg.png";
        assert_eq!(super::standard_thumb(b), "https://upload.wikimedia.org/wikipedia/fr/thumb/d/d5/FIP.svg/330px-FIP.svg.png");
        assert_eq!(super::standard_thumb("https://ex.com/1200px-logo.png"), "https://ex.com/1200px-logo.png");
    }

    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_logo() {
        let url = "https://upload.wikimedia.org/wikipedia/fr/thumb/d/d5/FIP_logo_2005.svg/1024px-FIP_logo_2005.svg.png";
        let path = super::logo(url).await.expect("logo");
        let img = image::open(&path).unwrap();
        println!("{path} : {}x{}", img.width(), img.height());
        assert!(img.width() <= super::LOGO_SIDE && img.height() <= super::LOGO_SIDE);
    }
}
