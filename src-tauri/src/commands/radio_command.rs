//! Commandes des radios : annuaire, adresse collée, favoris.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::commands::playback::{self, Target};
use crate::core::radio::live;

use crate::core::radio::directory::{self, RadioResult};
use crate::core::radio::stream;
use crate::entity::radio::radio::{RadioRecent, RadioView};
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
        return Err("la radio n'est pas encore disponible sur un lecteur réseau".into());
    }
    playback::stop_local();
    live::start(&station.stream_url, station.bitrate).await?;
    playback::play(app.clone(), PathBuf::from(live::LIVE_PATH)).await;
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
pub fn radio_status() -> Option<RadioStatus> {
    live::status().map(|s| RadioStatus { title: s.title, bitrate: s.bitrate, behind_secs: s.behind_secs, paused: s.paused })
}

/// Au lancement : lit le réglage.
pub async fn init(state: &AppState) {
    let on = SettingsRepository::get(&state.pool, ENABLED_KEY).await.ok().flatten().as_deref() == Some("true");
    radio_service::set_enabled(on);
}
