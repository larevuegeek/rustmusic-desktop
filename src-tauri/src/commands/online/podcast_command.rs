//! Commandes des podcasts : recherche, abonnements, épisodes, progression d'écoute.

use tauri::{AppHandle, State};

use crate::core::podcast::directory::{self, DirectoryResult};
use crate::entity::podcast::podcast::{DownloadView, EpisodeView, LatestEpisode, PodcastPreview, PodcastView};
use crate::repository::podcast::podcast_repository::PodcastRepository;
use crate::repository::settings::settings_repository::SettingsRepository;
use crate::service::podcast::podcast_service::{self, ENABLED_KEY};
use crate::state::AppState;

fn db(e: sqlx::Error) -> String {
    e.to_string()
}

/// Active ou coupe les podcasts ; les abonnements sont gardés.
#[tauri::command]
pub fn set_podcasts_enabled(enabled: bool) {
    podcast_service::set_enabled(enabled);
}

#[tauri::command]
pub async fn podcast_search(term: String, country: String) -> Result<Vec<DirectoryResult>, String> {
    podcast_service::ensure_enabled()?;
    directory::search(&term, &country).await
}

/// Les plus écoutés du moment ; `genre` : identifiant de catégorie Apple.
#[tauri::command]
pub async fn podcast_top(country: String, genre: Option<u32>) -> Result<Vec<DirectoryResult>, String> {
    podcast_service::ensure_enabled()?;
    directory::top(&country, genre).await
}

/// Épisodes à écouter du profil : commencés, puis nouveaux.
#[tauri::command]
pub async fn get_podcast_latest(state: State<'_, AppState>, profil_id: i64, limit: Option<i64>) -> Result<Vec<LatestEpisode>, String> {
    PodcastRepository::latest(&state.pool, profil_id, limit.unwrap_or(20)).await.map_err(db)
}

/// Aperçu d'un flux avant l'abonnement.
#[tauri::command]
pub async fn podcast_preview(url: String) -> Result<PodcastPreview, String> {
    podcast_service::preview(&url).await
}

/// Favori : épinglé dans la barre latérale.
/// Abonnements depuis un fichier OPML (exporté d'une autre appli de podcasts).
#[tauri::command]
pub async fn podcast_import_opml(state: State<'_, AppState>, profil_id: i64, path: String) -> Result<podcast_service::OpmlImport, String> {
    let xml = tokio::fs::read_to_string(&path).await.map_err(|e| format!("fichier illisible : {e}"))?;
    podcast_service::import_opml(&state.pool, profil_id, &xml).await
}

/// Abonnements du profil écrits en OPML à `path` ; rend leur nombre.
#[tauri::command]
pub async fn podcast_export_opml(state: State<'_, AppState>, profil_id: i64, path: String) -> Result<usize, String> {
    let (xml, count) = podcast_service::export_opml(&state.pool, profil_id).await?;
    tokio::fs::write(&path, xml).await.map_err(|e| format!("fichier non écrit : {e}"))?;
    Ok(count)
}

/// Télécharge automatiquement les nouveaux épisodes de ce podcast (ceux qui paraîtront).
#[tauri::command]
pub async fn set_podcast_auto_download(state: State<'_, AppState>, podcast_id: i64, on: bool) -> Result<(), String> {
    PodcastRepository::set_auto_download(&state.pool, podcast_id, on).await.map_err(db)
}

#[tauri::command]
pub async fn set_podcast_pinned(state: State<'_, AppState>, podcast_id: i64, pinned: bool) -> Result<(), String> {
    PodcastRepository::set_pinned(&state.pool, podcast_id, pinned).await.map_err(db)
}

#[tauri::command]
pub async fn podcast_subscribe(state: State<'_, AppState>, profil_id: i64, url: String) -> Result<PodcastView, String> {
    let id = podcast_service::subscribe(&state.pool, profil_id, &url).await?;
    PodcastRepository::find_one(&state.pool, id).await.map_err(db)?.ok_or_else(|| "podcast introuvable".into())
}

#[tauri::command]
pub async fn podcast_unsubscribe(state: State<'_, AppState>, podcast_id: i64) -> Result<(), String> {
    let files = podcast_service::files_of(&state.pool, podcast_id).await;
    PodcastRepository::delete(&state.pool, podcast_id).await.map_err(db)?;
    podcast_service::remove_cover(podcast_id);
    podcast_service::remove_files(&files);
    Ok(())
}

#[tauri::command]
pub async fn get_podcasts(state: State<'_, AppState>, profil_id: i64) -> Result<Vec<PodcastView>, String> {
    PodcastRepository::find_by_profile(&state.pool, profil_id).await.map_err(db)
}

#[tauri::command]
pub async fn get_podcast(state: State<'_, AppState>, podcast_id: i64) -> Result<Option<PodcastView>, String> {
    PodcastRepository::find_one(&state.pool, podcast_id).await.map_err(db)
}

#[tauri::command]
pub async fn get_podcast_episodes(state: State<'_, AppState>, podcast_id: i64) -> Result<Vec<EpisodeView>, String> {
    PodcastRepository::episodes(&state.pool, podcast_id).await.map_err(db)
}

#[tauri::command]
pub async fn get_podcast_episode(state: State<'_, AppState>, episode_id: i64) -> Result<Option<EpisodeView>, String> {
    PodcastRepository::episode(&state.pool, episode_id).await.map_err(db)
}

/// Relit un flux (`podcast_id`) ou tous ; renvoie le nombre d'épisodes parus.
#[tauri::command]
pub async fn podcast_refresh(state: State<'_, AppState>, podcast_id: Option<i64>) -> Result<usize, String> {
    match podcast_id {
        Some(id) => podcast_service::refresh_one(&state.pool, id).await,
        None => {
            podcast_service::ensure_enabled()?;
            Ok(podcast_service::refresh_all(&state.pool).await)
        }
    }
}

#[tauri::command]
pub async fn podcast_set_position(state: State<'_, AppState>, episode_id: i64, position: f64) -> Result<(), String> {
    PodcastRepository::set_position(&state.pool, episode_id, position).await.map_err(db)
}

#[tauri::command]
pub async fn podcast_set_played(state: State<'_, AppState>, episode_id: i64, played: bool) -> Result<(), String> {
    PodcastRepository::set_played(&state.pool, episode_id, played).await.map_err(db)
}

/// Vitesse de l'épisode en cours (retenue pour son émission).
#[tauri::command]
pub async fn podcast_set_speed(state: State<'_, AppState>, path: String, speed: f32) -> Result<(), String> {
    podcast_service::set_speed(&state.pool, std::path::Path::new(&path), speed).await
}

/// Chapitres de l'épisode `path` (vide s'il n'en a pas).
#[tauri::command]
pub async fn podcast_chapters(state: State<'_, AppState>, path: String) -> Result<Vec<crate::core::podcast::chapters::Chapter>, String> {
    Ok(podcast_service::chapters(&state.pool, std::path::Path::new(&path)).await)
}

/// Vitesse de lecture en vigueur.
#[tauri::command]
pub fn playback_speed() -> f32 {
    crate::core::audio_player::time_stretch::speed()
}

/// Retire une émission de « À écouter » jusqu'à son prochain épisode.
#[tauri::command]
pub async fn podcast_hide_from_listen(state: State<'_, AppState>, podcast_id: i64) -> Result<(), String> {
    PodcastRepository::hide_from_listen(&state.pool, podcast_id).await.map_err(db)
}

/// Vide « À écouter » (les émissions reviennent avec leur prochain épisode).
#[tauri::command]
pub async fn podcast_empty_listen(state: State<'_, AppState>, profil_id: i64) -> Result<(), String> {
    PodcastRepository::hide_all_from_listen(&state.pool, profil_id).await.map_err(db)
}

/// Efface l'historique d'écoute des podcasts du profil.
#[tauri::command]
pub async fn podcast_clear_history(state: State<'_, AppState>, profil_id: i64) -> Result<(), String> {
    PodcastRepository::clear_history(&state.pool, profil_id).await.map_err(db)
}

#[tauri::command]
pub async fn podcast_set_all_played(state: State<'_, AppState>, podcast_id: i64) -> Result<(), String> {
    PodcastRepository::set_all_played(&state.pool, podcast_id).await.map_err(db)
}

/// Télécharge un épisode sans l'écouter.
#[tauri::command]
pub async fn podcast_download(state: State<'_, AppState>, episode_id: i64) -> Result<(), String> {
    podcast_service::download_episode(&state.pool, episode_id).await
}

/// Efface le fichier téléchargé d'un épisode ; renvoie les octets libérés.
#[tauri::command]
pub async fn podcast_delete_download(state: State<'_, AppState>, episode_id: i64) -> Result<u64, String> {
    podcast_service::delete_download(&state.pool, episode_id).await.map(Option::unwrap_or_default)
}

/// Vide les téléchargements (d'un podcast, ou de tous) ; renvoie (fichiers, octets).
#[tauri::command]
pub async fn podcast_delete_downloads(
    state: State<'_, AppState>,
    profil_id: i64,
    podcast_id: Option<i64>,
    played_only: bool,
) -> Result<(usize, u64), String> {
    podcast_service::delete_downloads(&state.pool, profil_id, podcast_id, played_only).await
}

#[tauri::command]
pub async fn get_podcast_downloads(state: State<'_, AppState>, profil_id: i64) -> Result<Vec<DownloadView>, String> {
    podcast_service::downloads(&state.pool, profil_id).await
}

/// Où reprendre un épisode ; `None` si `path` n'en est pas un.
#[tauri::command]
pub async fn podcast_resume_position(state: State<'_, AppState>, path: String) -> Result<Option<f64>, String> {
    Ok(podcast_service::resume_position(&state.pool, std::path::Path::new(&path)).await)
}

/// Au lancement : lit le réglage et démarre la vérification de fond.
pub async fn init(state: &AppState, app: AppHandle) {
    let on = SettingsRepository::get(&state.pool, ENABLED_KEY).await.ok().flatten().as_deref() == Some("true");
    podcast_service::set_enabled(on);
    podcast_service::set_app(app.clone());
    podcast_service::start_background(app, state.pool.clone());
}
