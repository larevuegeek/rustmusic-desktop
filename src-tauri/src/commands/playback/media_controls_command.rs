//! Tauri commands pour piloter SMTC (System Media Transport Controls) depuis
//! le frontend. SMTC est géré côté backend (souvlaki) parce que c'est l'OS
//! qui dialogue avec lui ; le frontend ne fait que pousser les métadonnées
//! et l'état au fil de la lecture.

use serde::Deserialize;
use tauri::AppHandle;

use crate::core::media_controls;

/// Active SMTC. Idempotent côté Rust.
/// À appeler depuis le frontend si le réglage `system_media_controls` est ON.
#[tauri::command]
pub fn enable_media_controls(app: AppHandle) -> Result<(), String> {
    media_controls::init(&app)
}

/// Désactive SMTC. Idempotent.
#[tauri::command]
pub fn disable_media_controls() {
    media_controls::shutdown();
}

/// Indique si SMTC est actuellement actif (utile au démarrage pour synchroniser
/// l'état du toggle dans les réglages sans avoir à le persister deux fois).
#[tauri::command]
pub fn is_media_controls_active() -> bool {
    media_controls::is_active()
}

#[derive(Debug, Deserialize)]
pub struct MediaMetadataPayload {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// Chemin fichier absolu OU URL distante. Convertie en URL utilisable
    /// par souvlaki via `cover_to_url` (typiquement servie par notre serveur
    /// HTTP local sous `http://127.0.0.1:port/cover/...`).
    pub cover_path: Option<String>,
    pub duration_secs: Option<f64>,
}

#[tauri::command]
pub fn update_media_metadata(payload: MediaMetadataPayload) {
    let cover_url: Option<String> = payload.cover_path.as_deref().and_then(cover_to_url);

    media_controls::set_metadata(
        &payload.title,
        payload.artist.as_deref(),
        payload.album.as_deref(),
        cover_url.as_deref(),
        payload.duration_secs,
    );
}

/// Convertit l'entrée cover en URL utilisable par souvlaki/SMTC.
/// Stratégie cross-OS uniforme :
/// - URL distante (`http://`, `https://`) : passe telle quelle.
/// - Chemin fichier sous `covers/` : transformé en `http://127.0.0.1:port/cover/...`
///   via le serveur HTTP local démarré au moment de l'init SMTC. Évite le
///   piège Windows SMTC qui rejette `file://` (HRESULT 0x800700A1) faute de
///   capability `internetClient` en contexte unpackaged. Bonus : marche
///   identique sur Linux/macOS, un seul code path.
/// - Data URI base64 / `asset://` / `blob:` / chemin hors `covers/` /
///   chemin relatif / vide → `None` (SMTC affichera l'icône de l'app à la
///   place, plutôt qu'un crash).
fn cover_to_url(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    if path.starts_with("http://") || path.starts_with("https://") {
        return Some(path.to_string());
    }
    if path.starts_with("data:") || path.starts_with("asset:") || path.starts_with("blob:") {
        return None;
    }
    media_controls::cover_url_for_path(path)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Deserialize)]
pub struct MediaPlaybackPayload {
    pub state: PlaybackState,
    /// Position courante en secondes (utilisée pour les UI OS qui affichent
    /// une barre de progression — Windows volume flyout, MPRIS). Optionnelle.
    pub position_secs: Option<f64>,
}

#[tauri::command]
pub fn update_media_playback(payload: MediaPlaybackPayload) {
    let state = match payload.state {
        PlaybackState::Playing => media_controls::PlaybackState::Playing,
        PlaybackState::Paused => media_controls::PlaybackState::Paused,
        PlaybackState::Stopped => media_controls::PlaybackState::Stopped,
    };
    media_controls::set_playback(state, payload.position_secs);
}
