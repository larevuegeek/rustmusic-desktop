use tauri::State;
use crate::repository::library::library_files_repository::LibraryFilesRepository;

use crate::core::audio_lyrics::{lrclib_client, sidecar};
use crate::entity::audio_lyrics::lyrics::{Lyrics, LyricsSource, LyricsUpsert};
use crate::repository::audio_lyrics::lyrics_repository::LyricsRepository;
use crate::repository::library::library_track_repository::LibraryTrackRepository;
use crate::state::AppState;

/// Récupère le `library_files.id` d'un chemin sans décoder l'entité LibraryFile
/// complète (qui a un champ `modified_at` parfois stocké en INTEGER mais déclaré
/// en `Option<String>` dans l'entity → conflit de décodage).
async fn find_file_id_by_path(
    pool: &sqlx::SqlitePool,
    path: &str,
) -> Result<Option<String>, sqlx::Error> {
    LibraryFilesRepository::find_id_by_path(pool, path).await
}

/// Récupère les paroles d'un morceau via son chemin de fichier audio.
///
/// On accepte le `path` plutôt qu'un track_id parce que le player garde
/// le `path` en référence stable, alors que `trackId` côté front correspond
/// au queueId (pas à `library_tracks.id`).
///
/// Cascade :
/// 1. Lookup library_files via path → file_id
/// 2. Lookup library_tracks via file_id → métadonnées + library_track_id
/// 3. Cache DB (renvoyé tel quel même si partiel ou "none")
/// 4. Sidecar `.lrc` à côté du fichier
/// 5. LRCLIB en ligne
/// 6. Sinon → marque `source=none` pour ne pas re-spammer
#[tauri::command]
pub async fn get_lyrics(
    state: State<'_, AppState>,
    path: String,
) -> Result<Option<Lyrics>, String> {

    // 1. Lookup file_id via path (requête scalaire)
    let file_id = find_file_id_by_path(&state.pool, &path)
        .await
        .map_err(|e| format!("DB error : {}", e))?
        .ok_or_else(|| format!("Fichier introuvable en base : {}", path))?;

    // 2. Lookup track via file_id
    let track = LibraryTrackRepository::find_track_view_by_file_id(&state.pool, &file_id)
        .await
        .map_err(|e| format!("DB error : {}", e))?
        .ok_or_else(|| format!("Track introuvable pour ce fichier : {}", path))?;

    let track_id = track.id.clone();

    // 3. Cache DB
    if let Ok(Some(cached)) = LyricsRepository::find_by_track_id(&state.pool, &track_id).await {
        return Ok(Some(cached));
    }

    // 4. Sidecar .lrc
    if let Some(lrc) = sidecar::read_sidecar_lrc(&track.path) {
        let synced = if sidecar::has_synced_timestamps(&lrc) {
            Some(lrc.clone())
        } else {
            None
        };
        let plain = if synced.is_none() { Some(lrc) } else { None };

        let saved = LyricsRepository::upsert(
            &state.pool,
            LyricsUpsert {
                track_id: track_id.clone(),
                plain,
                synced,
                source: LyricsSource::Sidecar,
                lrclib_id: None,
            },
        )
        .await
        .map_err(|e| format!("DB upsert error : {}", e))?;

        return Ok(Some(saved));
    }

    // 5. LRCLIB
    let artist = track.artist.unwrap_or_default();
    let title = track.title.clone();
    let album = track.album;
    let duration = track.duration.unwrap_or(0.0).round() as u32;

    if artist.is_empty() || title.is_empty() {
        let _ = LyricsRepository::mark_not_found(&state.pool, &track_id).await;
        return Ok(None);
    }

    match lrclib_client::fetch_lyrics(&artist, &title, album.as_deref(), duration).await {
        Ok(Some(resp)) => {
            let saved = LyricsRepository::upsert(
                &state.pool,
                LyricsUpsert {
                    track_id: track_id.clone(),
                    plain: resp.plain_lyrics,
                    synced: resp.synced_lyrics,
                    source: LyricsSource::Lrclib,
                    lrclib_id: resp.id,
                },
            )
            .await
            .map_err(|e| format!("DB upsert error : {}", e))?;

            Ok(Some(saved))
        }
        Ok(None) => {
            let _ = LyricsRepository::mark_not_found(&state.pool, &track_id).await;
            Ok(None)
        }
        Err(e) => {
            log::warn!("LRCLIB fetch failed for {}: {}", path, e);
            Err(e)
        }
    }
}

/// Force un re-fetch en supprimant le cache puis en relançant `get_lyrics`.
#[tauri::command]
pub async fn refresh_lyrics(
    state: State<'_, AppState>,
    path: String,
) -> Result<Option<Lyrics>, String> {
    // On a besoin du track_id pour delete : même lookup que get_lyrics
    let file_id = find_file_id_by_path(&state.pool, &path)
        .await
        .map_err(|e| format!("DB error : {}", e))?
        .ok_or_else(|| format!("Fichier introuvable en base : {}", path))?;

    let track = LibraryTrackRepository::find_track_view_by_file_id(&state.pool, &file_id)
        .await
        .map_err(|e| format!("DB error : {}", e))?
        .ok_or_else(|| format!("Track introuvable pour ce fichier : {}", path))?;

    LyricsRepository::delete_by_track_id(&state.pool, &track.id)
        .await
        .map_err(|e| format!("DB delete error : {}", e))?;

    get_lyrics(state, path).await
}
