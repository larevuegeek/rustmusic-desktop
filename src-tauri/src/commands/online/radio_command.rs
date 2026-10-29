//! Commandes des radios : annuaire, adresse collée, favoris.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::commands::playback::playback::{self, Target};
use crate::core::radio::live;

use crate::core::radio::directory::{self, RadioResult, StreamVariant};
use crate::core::radio::stream;
use crate::entity::radio::radio::{RadioRecent, RadioTitle, RadioView};
use crate::repository::radio::radio_repository::RadioRepository;
use crate::repository::settings::settings_repository::SettingsRepository;
use crate::service::radio::radio_service::{self, ENABLED_KEY};
use crate::state::AppState;

fn db(e: sqlx::Error) -> String {
    e.to_string()
}

/// Active ou coupe les radios ; les favoris sont gardés.
#[tauri::command]
pub fn set_radios_enabled(enabled: bool) {
    radio_service::set_enabled(enabled);
}

#[tauri::command]
pub async fn radio_search(term: String, country: Option<String>, tag: Option<String>) -> Result<Vec<RadioResult>, String> {
    radio_service::ensure_enabled()?;
    directory::search(&term, country.as_deref(), tag.as_deref()).await
}

/// Les plus écoutées d'un pays ; `tag` : un genre (jazz, news…).
#[tauri::command]
pub async fn radio_top(country: String, tag: Option<String>) -> Result<Vec<RadioResult>, String> {
    radio_service::ensure_enabled()?;
    directory::top(&country, tag.as_deref()).await
}

/// Décrit une adresse collée (flux direct, .m3u, .pls).
#[tauri::command]
pub async fn radio_resolve(url: String) -> Result<RadioResult, String> {
    radio_service::ensure_enabled()?;
    stream::resolve(&url).await
}

/// Logo d'une station de l'annuaire, mis en cache ; `None` sans logo lisible.
#[tauri::command]
pub async fn radio_logo(url: String) -> Result<Option<String>, String> {
    radio_service::ensure_enabled()?;
    Ok(radio_service::logo(&url).await)
}

#[tauri::command]
pub async fn get_radios(state: State<'_, AppState>, profil_id: i64) -> Result<Vec<RadioView>, String> {
    RadioRepository::find_by_profile(&state.pool, profil_id).await.map_err(db)
}

#[tauri::command]
pub async fn radio_add(state: State<'_, AppState>, profil_id: i64, station: RadioResult) -> Result<RadioView, String> {
    radio_service::ensure_enabled()?;
    radio_service::add(&state.pool, profil_id, &station).await
}

#[tauri::command]
pub async fn radio_remove(state: State<'_, AppState>, radio_id: i64) -> Result<(), String> {
    RadioRepository::delete(&state.pool, radio_id).await.map_err(db)
}

#[tauri::command]
pub async fn radio_rename(state: State<'_, AppState>, radio_id: i64, name: String) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("nom vide".into());
    }
    RadioRepository::rename(&state.pool, radio_id, name).await.map_err(db)
}

#[tauri::command]
pub async fn reorder_radios(state: State<'_, AppState>, profil_id: i64, ids: Vec<i64>) -> Result<(), String> {
    RadioRepository::reorder(&state.pool, profil_id, &ids).await.map_err(db)
}

/// Lance la radio ; ce qui jouait s'arrête (une seule source à la fois).
/// L'écoute est retenue dans « Dernières écoutées » (`radios-recent-updated` une fois le logo en cache).
#[tauri::command]
pub async fn radio_play(app: AppHandle, state: State<'_, AppState>, profil_id: i64, station: RadioResult, radio_id: Option<i64>) -> Result<(), String> {
    radio_service::ensure_enabled()?;
    if playback::target() == Target::Upnp {
        // Lecteur réseau : il lit le flux lui-même (pas de différé ni de titres ici).
        live::stop();
        crate::core::network_player::session::play_stream(app.clone(), &station.stream_url, &station.name, station.codec.as_deref(), station.favicon.as_deref())
            .await?;
        *listening().lock().unwrap_or_else(|e| e.into_inner()) = None;
    } else {
        playback::stop_local();
        live::start(&station.stream_url, station.bitrate).await?;
        *listening().lock().unwrap_or_else(|e| e.into_inner()) =
            Some(Listening { profil_id, station: station.name.clone(), stream_url: station.stream_url.clone(), last_title: None });
        playback::play(app.clone(), PathBuf::from(live::LIVE_PATH)).await;
    }
    if let Some(id) = radio_id {
        RadioRepository::set_played(&state.pool, id).await.map_err(db)?;
    }
    let pool = state.pool.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(uuid) = station.uuid.as_deref() {
            directory::count_click(uuid).await;
        }
        let logo = match station.favicon.as_deref() {
            Some(url) => radio_service::logo(url).await,
            None => None,
        };
        if RadioRepository::record_recent(&pool, profil_id, &station, logo.as_deref()).await.is_ok() {
            let _ = app.emit("radios-recent-updated", profil_id);
        }
    });
    Ok(())
}

/// Radios écoutées récemment, la dernière d'abord.
#[tauri::command]
pub async fn get_radio_recent(state: State<'_, AppState>, profil_id: i64) -> Result<Vec<RadioRecent>, String> {
    RadioRepository::recent(&state.pool, profil_id).await.map_err(db)
}

/// Coupe la radio.
#[tauri::command]
pub async fn radio_stop() {
    playback::stop().await;
}

/// Après une pause : abandonne le différé et repart du direct.
#[tauri::command]
pub async fn radio_go_live(app: AppHandle) -> Result<(), String> {
    if live::current().is_none() {
        return Err("aucune radio en cours".into());
    }
    live::rewind_to_live();
    playback::stop_local();
    playback::play(app, PathBuf::from(live::LIVE_PATH)).await;
    Ok(())
}

/// La station écoutée et le dernier titre noté : un titre n'est enregistré qu'une fois.
struct Listening {
    profil_id: i64,
    station: String,
    stream_url: String,
    last_title: Option<String>,
}

fn listening() -> &'static std::sync::Mutex<Option<Listening>> {
    static NOW: std::sync::OnceLock<std::sync::Mutex<Option<Listening>>> = std::sync::OnceLock::new();
    NOW.get_or_init(Default::default)
}

/// Titre à noter : ni vide, ni le nom de la station (certaines l'annoncent entre deux morceaux).
fn worth_recording(title: &str, station: &str) -> bool {
    let t = title.trim();
    !t.is_empty() && !t.eq_ignore_ascii_case(station.trim()) && t.chars().any(char::is_alphanumeric)
}

#[derive(Serialize)]
pub struct RadioStatus {
    /// Titre annoncé par la station pour ce qu'on entend.
    pub title: Option<String>,
    pub bitrate: u32,
    /// Retard sur le direct, dû aux pauses (s).
    pub behind_secs: f64,
    pub paused: bool,
}

/// État de la radio en cours ; `None` sans radio.
#[tauri::command]
pub async fn radio_status(app: AppHandle, state: State<'_, AppState>) -> Result<Option<RadioStatus>, String> {
    let Some(s) = live::status() else { return Ok(None) };
    // Nouveau titre entendu : noté dans l'historique.
    let record = s.title.as_deref().and_then(|title| {
        let mut now = listening().lock().unwrap_or_else(|e| e.into_inner());
        let l = now.as_mut()?;
        if l.last_title.as_deref() == Some(title) {
            return None;
        }
        l.last_title = Some(title.to_string());
        worth_recording(title, &l.station).then(|| (l.profil_id, l.station.clone(), l.stream_url.clone(), title.to_string()))
    });
    if let Some((profil_id, station, url, title)) = record {
        if RadioRepository::record_title(&state.pool, profil_id, &station, &url, &title).await.is_ok() {
            let _ = app.emit("radio-titles-updated", profil_id);
        }
    }
    Ok(Some(RadioStatus { title: s.title, bitrate: s.bitrate, behind_secs: s.behind_secs, paused: s.paused }))
}

/// Les versions du flux d'une station, pour choisir la qualité.
#[tauri::command]
pub async fn radio_variants(name: String, country: Option<String>) -> Result<Vec<StreamVariant>, String> {
    directory::variants(&name, country.as_deref()).await
}

/// Une favorite change de flux (autre format, autre débit de la même station).
#[tauri::command]
pub async fn radio_set_stream(state: State<'_, AppState>, radio_id: i64, variant: StreamVariant) -> Result<(), String> {
    RadioRepository::set_stream(&state.pool, radio_id, &variant).await.map_err(db)
}

/// Titres passés à l'antenne, le plus récent d'abord.
#[tauri::command]
pub async fn get_radio_titles(state: State<'_, AppState>, profil_id: i64) -> Result<Vec<RadioTitle>, String> {
    RadioRepository::titles(&state.pool, profil_id, 100).await.map_err(db)
}

#[tauri::command]
pub async fn clear_radio_titles(state: State<'_, AppState>, profil_id: i64) -> Result<(), String> {
    RadioRepository::clear_titles(&state.pool, profil_id).await.map_err(db)
}

/// Efface l'historique : dernières radios écoutées et titres passés (les favorites restent).
#[tauri::command]
pub async fn clear_radio_history(state: State<'_, AppState>, profil_id: i64) -> Result<(), String> {
    RadioRepository::clear_history(&state.pool, profil_id).await.map_err(db)
}

/// Au lancement : lit le réglage.
pub async fn init(state: &AppState) {
    let on = SettingsRepository::get(&state.pool, ENABLED_KEY).await.ok().flatten().as_deref() == Some("true");
    radio_service::set_enabled(on);
}

#[cfg(test)]
mod title_tests {
    use super::worth_recording;

    #[test]
    fn station_names_and_blanks_are_not_titles() {
        assert!(worth_recording("Daft Punk - Around the World", "FIP"));
        assert!(!worth_recording("  ", "FIP"));
        assert!(!worth_recording("fip", "FIP"));
        assert!(!worth_recording(" - ", "FIP"));
    }
}
