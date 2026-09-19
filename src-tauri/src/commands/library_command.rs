use std::path::PathBuf;
use serde::Serialize;
use tauri::State;
use base64::engine::general_purpose;
use base64::Engine;

use tauri::{Emitter, Manager, path::BaseDirectory};
use crate::entity::audio::file_tags_info::FileTagsInfo;
use crate::entity::library::genre_view::GenreView;
use crate::entity::library::library_stats::*;
use crate::entity::library::dir_entry::DirEntry;
use crate::entity::library::library::{Library, LibraryCreate};
use crate::entity::library::library_cache::{LibraryCache, LibraryCacheCreate};
use crate::helper::library::thumbnail_helper::{thumbnail_saver, save_artist_image, resolve_thumbnail};
use crate::mapper::library::album::album_detail_view::AlbumDetailView;
use crate::mapper::library::album::album_list_view::AlbumListView;
use crate::mapper::library::artist::artist_detail_view::ArtistDetailView;
use crate::mapper::library::artist::artist_list_view::ArtistListView;
use crate::mapper::library::track::track_detail_view::TrackDetailView;
use crate::repository::artist::artist_repository::ArtistRepository;
use crate::repository::library::library_album_repository::LibraryAlbumRepository;
use crate::repository::library::library_artist_repository::LibraryArtistRepository;
use crate::repository::library::library_cache_repository::LibraryCacheRepository;
use crate::repository::library::library_dirs_repository::LibraryDirRepository;
use crate::repository::library::library_genre_repository::LibraryGenreRepository;
use crate::repository::library::library_repository::LibraryRepository;
use crate::repository::library::library_stats_repository::LibraryStatsRepository;
use crate::repository::library::library_track_repository::LibraryTrackRepository;
use crate::mapper::library::artist::track_artist_view::TrackArtistView;
use crate::repository::library::library_track_artist_repository::LibraryTrackArtistRepository;
use crate::service::library::artist_link_repair::ArtistLinkReport;
use crate::service::library::library_service::{LibrarySaveContext, create_context, save_dir_to_library, save_track_to_library};
use crate::{state::AppState};
use crate::mapper::library::track::track_list_item_view::TrackListView;

#[tauri::command]
pub async fn add_files(
    app: tauri::AppHandle,
    state: State<'_, AppState>, 
    library_id: i64,
    files: Vec<String>
) -> Result<Vec<TrackListView>, String> {
    
    let mut tracks: Vec<TrackListView> = Vec::new();

    let ctx: LibrarySaveContext = create_context(app, &state.pool);

    for file in files {
        let track_list_view = match save_track_to_library(&ctx, library_id, None, file.clone()).await {
            Ok(track) => track,
            Err(e) => {
                log::error!("Failed to save track {} : {}", file, e);
                continue;
            }
        };

        tracks.push(track_list_view);
    }

    Ok(tracks)
}

#[tauri::command]
pub async fn add_directory(
    app: tauri::AppHandle,
    state: State<'_, AppState>, 
    library_id: i64,
    directory: String
) -> Result<Vec<TrackListView>, String> {
    
    let tracks: Vec<TrackListView> = save_dir_to_library(app, &state.pool, library_id, directory).await?;

    Ok(tracks)
}

/// Rejoue les liaisons piste ↔ artiste. Voir `artist_link_repair`.
/// Les artistes crédités sur une piste. Vide tant que les liaisons ne sont pas
/// posées — la fiche retombe alors sur le tag brut.
#[tauri::command]
pub async fn get_track_artists(
    state: State<'_, AppState>,
    track_id: String,
) -> Result<Vec<TrackArtistView>, String> {
    LibraryTrackArtistRepository::find_artists_of_track(&state.pool, &track_id)
        .await
        .map_err(|e| format!("Failed to get track artists: {}", e))
}

#[tauri::command]
pub async fn repair_artist_links(
    state: State<'_, AppState>,
    library_id: Option<i64>,
) -> Result<ArtistLinkReport, String> {
    crate::service::library::artist_link_repair::repair(&state.pool, library_id).await
}

#[tauri::command]
pub async fn rescan_library(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<Vec<TrackListView>, String> {
    // Récupérer tous les dossiers actifs de la bibliothèque
    let dirs = LibraryDirRepository::find_active(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get library dirs: {}", e))?;

    if dirs.is_empty() {
        return Err("Aucun dossier enregistré dans cette bibliothèque".to_string());
    }

    let mut all_tracks: Vec<TrackListView> = Vec::new();

    // Re-scanner chaque dossier (save_dir_to_library skip les fichiers déjà importés via le cache)
    for dir in dirs {
        match save_dir_to_library(app.clone(), &state.pool, library_id, dir.path).await {
            Ok(tracks) => all_tracks.extend(tracks),
            Err(e) => log::error!("Erreur rescan dossier {}: {}", dir.name, e),
        }
    }

    Ok(all_tracks)
}

#[tauri::command]
pub async fn get_library_dirs(
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<Vec<crate::entity::library::library_dirs::LibraryDir>, String> {
    LibraryDirRepository::find_all_by_library_id(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get library dirs: {}", e))
}

#[tauri::command]
pub async fn get_tracks_by_dir(
    state: State<'_, AppState>,
    library_id: i64,
    dir_id: String,
) -> Result<Vec<TrackListView>, String> {
    LibraryTrackRepository::find_tracks_by_dir_id(&state.pool, library_id, &dir_id)
        .await
        .map_err(|e| format!("Failed to get tracks by dir: {}", e))
}

// ============================================================================
// EXPLORATEUR DE FICHIERS (restreint aux dossiers importés)
// ============================================================================

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "m4a", "aac", "wma", "opus", "aiff", "alac", "ape", "dsf", "dff"
];

#[tauri::command]
pub async fn list_directory(
    state: State<'_, AppState>,
    library_id: i64,
    path: String,
) -> Result<Vec<DirEntry>, String> {

    // Vérifier que le chemin demandé est bien dans un dossier importé de cette bibliothèque
    let dirs = LibraryDirRepository::find_all_by_library_id(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get dirs: {}", e))?;

    // Canonicalize le chemin demandé pour éviter les path traversal (../../)
    let canonical_path = std::fs::canonicalize(&path)
        .map_err(|e| format!("Invalid path: {}", e))?;

    let is_allowed = dirs.iter().any(|d| {
        if let Ok(canonical_dir) = std::fs::canonicalize(&d.path) {
            canonical_path.starts_with(&canonical_dir)
        } else {
            false
        }
    });

    if !is_allowed {
        return Err("Accès refusé : ce dossier n'est pas dans la bibliothèque".to_string());
    }

    let path = canonical_path.to_string_lossy().to_string();

    // Lire le contenu du dossier
    let read_dir = std::fs::read_dir(&path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut entries: Vec<DirEntry> = Vec::new();

    for entry in read_dir.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().to_string();

        // Ignorer les fichiers/dossiers cachés (commencent par .)
        if name.starts_with('.') { continue; }

        let entry_path = entry.path();
        let path_string = entry_path.to_string_lossy().to_string();

        if file_type.is_dir() {
            entries.push(DirEntry {
                name,
                path: path_string,
                is_dir: true,
                size: 0,
                extension: None,
            });
        } else if file_type.is_file() {
            let ext = entry_path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());

            // Ne montrer que les fichiers audio
            if let Some(ref ext_str) = ext {
                if AUDIO_EXTENSIONS.contains(&ext_str.as_str()) {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    entries.push(DirEntry {
                        name,
                        path: path_string,
                        is_dir: false,
                        size,
                        extension: ext,
                    });
                }
            }
        }
    }

    // Trier : dossiers d'abord, puis fichiers, alphabétiquement
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}

#[tauri::command]
pub fn get_file_tags(path: String) -> Result<FileTagsInfo, String> {
    use crate::core::audio_analyser::audio_analyser::AudioAnalyser;

    let file_buf = PathBuf::from(&path);
    let metadata = std::fs::metadata(&path).map_err(|e| format!("Failed to read metadata: {}", e))?;
    let audio_file = AudioAnalyser::analyse_audio_file(&file_buf).map_err(|e| e.to_string())?;

    let filename = file_buf.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
    let extension = file_buf.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();

    let cover = audio_file.tags.attached_images.first()
        .map(|img| img.image_src.clone())
        .filter(|src| !src.is_empty());

    Ok(FileTagsInfo {
        path,
        filename,
        extension,
        size: metadata.len(),
        title: audio_file.tags.title,
        artist: audio_file.tags.artist,
        album: audio_file.tags.album,
        album_artist: audio_file.tags.album_artist,
        year: audio_file.tags.year,
        genre: audio_file.tags.genre,
        track_number: audio_file.tags.track_number,
        disc_number: audio_file.tags.disc_number,
        duration: audio_file.duration,
        bitrate: audio_file.bitrate,
        sample_rate: audio_file.sample_rate,
        bits_per_sample: audio_file.bits_per_sample,
        channels: audio_file.channels,
        audio_format: format!("{:?}", audio_file.audio_format),
        cover,
    })
}

#[tauri::command]
pub async fn remove_library_dir(
    state: State<'_, AppState>,
    dir_id: String,
) -> Result<(), String> {
    LibraryDirRepository::delete_library_dir(&state.pool, &dir_id)
        .await
        .map_err(|e| format!("Failed to remove library dir: {}", e))
}

#[tauri::command]
pub async fn rescan_library_dir(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    library_id: i64,
    dir_id: String,
) -> Result<Vec<TrackListView>, String> {
    let dirs = LibraryDirRepository::find_all_by_library_id(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get dirs: {}", e))?;

    let dir = dirs.into_iter().find(|d| d.id == dir_id)
        .ok_or_else(|| "Directory not found".to_string())?;

    save_dir_to_library(app, &state.pool, library_id, dir.path).await
}

#[tauri::command]
pub fn save_thumbnail(
    app: tauri::AppHandle,
    image_data: Vec<u8>,
) -> Result<String, String> {
    
    // 1. Résoudre le chemin AppData/covers
    let covers_dir: PathBuf = app
        .path()
        .resolve("covers", BaseDirectory::AppData)
        .map_err(|e| e.to_string())?;

    thumbnail_saver(&covers_dir, &image_data, false)
}

#[tauri::command]
pub fn read_cover_as_base64(path: String) -> Result<String, String> {
    // Générer la miniature à la volée si elle n'existe pas
    let resolved = resolve_thumbnail(&path).unwrap_or(path);

    let data = std::fs::read(&resolved).map_err(|e| e.to_string())?;
    let mime = if data.starts_with(&[0xFF, 0xD8]) { "image/jpeg" }
               else if data.starts_with(b"\x89PNG") { "image/png" }
               else { "image/jpeg" };
    let encoded = general_purpose::STANDARD.encode(&data);
    Ok(format!("data:{};base64,{}", mime, encoded))
}

/// Vérifie/génère un thumbnail et retourne le chemin résolu
/// Utilisé par le frontend en mode asset pour s'assurer que le fichier existe
#[tauri::command]
pub fn resolve_cover_thumbnail(path: String) -> Result<String, String> {
    resolve_thumbnail(&path)
        .ok_or_else(|| format!("Impossible de générer la miniature pour: {}", path))
}

#[tauri::command]
pub async fn create_library(
    state: State<'_, AppState>, 
    payload: LibraryCreate
) -> Result<Library, String> {

    let library: Library = match LibraryRepository::insert_library(&state.pool, &payload).await {
        Ok(library) => library,
        Err(e) => return Err(format!("Failed to insert library : {}", e))
    };

    // La toute première bibliothèque d'un profil devient sa bibliothèque par
    // défaut : demander à l'utilisateur de désigner la seule qu'il possède
    // n'aurait aucun sens. Sans effet si le profil en a déjà une.
    let _ = LibraryRepository::ensure_default(&state.pool, library.profil_id).await;

    // Relecture : la promotion vient peut-être de changer `is_default`, et
    // l'interface s'appuie dessus pour afficher le repère.
    Ok(LibraryRepository::find_library_by_id(&state.pool, library.id)
        .await
        .unwrap_or(library))
}

#[tauri::command]
pub async fn remove_library(
    state: State<'_, AppState>,
    library_id: i64
) -> Result<(), String> {

    // Le profil est lu AVANT la suppression : après, la ligne n'existe plus et
    // on ne saurait plus à qui redonner un défaut.
    let profil_id = LibraryRepository::find_library_by_id(&state.pool, library_id)
        .await
        .map(|library| library.profil_id)
        .ok();

    LibraryRepository::remove_library(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to remove library: {}", e))?;

    // Supprimer la bibliothèque par défaut laisserait le profil sans point
    // d'entrée au démarrage : on en promeut une autre.
    if let Some(profil_id) = profil_id {
        let _ = LibraryRepository::ensure_default(&state.pool, profil_id).await;
    }

    Ok(())
}

/// Désigne la bibliothèque ouverte au démarrage, pour le profil concerné.
#[tauri::command]
pub async fn set_default_library(
    state: State<'_, AppState>,
    library_id: i64
) -> Result<(), String> {

    LibraryRepository::set_default(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to set default library: {}", e))
}

#[tauri::command]
pub async fn get_library(
    state: State<'_, AppState>, 
    library_id: i64
) -> Result<Library, String> {

    let library: Library = match LibraryRepository::find_library_by_id(&state.pool, library_id).await {
        Ok(library) => library,
        Err(e) => return Err(format!("Failed to get library : {}", e))
    };

    Ok(library)
}

#[tauri::command]
pub async fn get_libraries(
    state: State<'_, AppState>, 
    profil_id: i64
) -> Result<Vec<Library>, String> {

    let libraries: Vec<Library> = match LibraryRepository::find_libraries_by_profil_id(&state.pool, profil_id).await {
        Ok(libraries) => libraries,
        Err(e) => return Err(format!("Failed to get libraries : {}", e))
    };

    Ok(libraries)
}

#[tauri::command]
pub async fn create_library_cache(
    state: State<'_, AppState>, 
    payload: LibraryCacheCreate
) -> Result<LibraryCache, String> {

    let library_cache: LibraryCache = match LibraryCacheRepository::upsert_library_cache(&state.pool, payload).await {
        Ok(lc) => lc,
        Err(e) => return Err(format!("Failed to insert library : {}", e))
    };

    Ok(library_cache)
}

#[tauri::command]
pub async fn get_library_cache_id_by_path(
    state: State<'_, AppState>, 
    path: String
) -> Result<Option<i64>, String> {

    let library_cache_id: Option<i64> = match LibraryCacheRepository::get_library_cache_id_by_path(&state.pool, &path).await {
        Ok(lc_id) => lc_id,
        Err(e) => return Err(format!("Failed to get library cache ID : {}", e))
    };

    Ok(library_cache_id)
}

#[tauri::command]
pub async fn get_track(
    state: State<'_, AppState>,
    library_track_id: String
) -> Result<TrackDetailView, String> {

    let new_track: TrackDetailView = match LibraryTrackRepository::find_track_by_id(&state.pool, library_track_id).await {
        Ok(track) => track,
        Err(e) => return Err(format!("Failed to get track : {}", e))
    };

    Ok(new_track)
}

#[tauri::command]
pub async fn set_track_rating(
    state: State<'_, AppState>,
    track_id: String,
    rating: Option<f64>,
) -> Result<(), String> {
    // Une note va de 0,5 à 5,0 par pas d'un demi. Hors de ces bornes — zéro
    // compris — elle vaut « pas de note », donc NULL : c'est ce qui distingue
    // « jamais noté » de « noté zéro » dans les tris.
    //
    // On aligne sur le demi-cran le plus proche plutôt que de refuser une
    // valeur intermédiaire. Un arrondi de l'interface ne doit pas se solder par
    // une note perdue en silence.
    let normalized = rating.and_then(|r| {
        let crans = (r * 2.0).round();
        if (1.0..=10.0).contains(&crans) {
            Some(crans / 2.0)
        } else {
            None
        }
    });
    LibraryTrackRepository::update_rating(&state.pool, &track_id, normalized)
        .await
        .map_err(|e| format!("Failed to update rating: {}", e))
}


#[tauri::command]
pub async fn get_tracks(
    state: State<'_, AppState>,
    library_id: i64
) -> Result<Vec<TrackListView>, String> {

    LibraryTrackRepository::find_all_tracks_by_library_id(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get tracks: {}", e))
}

/// Enregistre qu'un morceau a été écouté.
///
/// # Pourquoi par le chemin
/// La file d'attente ne transporte pas l'identifiant d'une piste de la
/// bibliothèque : elle sert aussi bien un fichier ouvert au vol qu'un morceau
/// de la bibliothèque, et le chemin est la seule chose que les deux ont en
/// commun. Un fichier hors bibliothèque ne correspond à aucune ligne, la
/// commande ne fait alors rien — ce qui est le comportement voulu.
///
/// # Rendue muette en cas d'échec
/// Un compteur d'écoutes n'est pas une donnée dont dépend la lecture. Le seul
/// appelant est la boucle de position, et lui faire remonter une erreur
/// reviendrait à salir la console pendant tout un morceau.
#[tauri::command]
pub async fn mark_track_played(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let touchees = sqlx::query(
        "UPDATE library_tracks
            SET play_count = play_count + 1,
                last_played_at = CURRENT_TIMESTAMP,
                updated_at = CURRENT_TIMESTAMP
          WHERE file_id IN (SELECT id FROM library_files WHERE path = ?)",
    )
    .bind(&path)
    .execute(&state.pool)
    .await
    .map_err(|e| format!("Compteur d'écoutes : {e}"))?
    .rows_affected();

    // Tracé en information, et non en débogage.
    //
    // Le compteur ne se voit qu'au bout d'une minute d'écoute, et seulement
    // dans une playlist qui s'en sert : quand il ne marche pas, rien ne le dit.
    // Cette ligne est le seul moyen de distinguer « jamais déclenché » de
    // « déclenché sans rien trouver » — deux pannes qui n'ont pas le même
    // remède.
    if touchees == 0 {
        log::info!("♪ Écoute non comptée, chemin absent de la bibliothèque : {path}");
    } else {
        log::info!("♪ Écoute comptée : {path}");
    }

    Ok(())
}

#[derive(Serialize)]
pub struct PaginatedTracks {
    pub tracks: Vec<TrackListView>,
    pub total: i64,
}

/// Traduit une clé de tri en expression SQL, et en valeur à lier s'il en faut.
///
/// # Le nom du tag ne rejoint jamais le texte de la requête
/// Les clés `tag:…` viennent de l'interface, donc en dernier ressort d'un
/// fichier de l'utilisateur : un morceau peut porter un tag nommé
/// `x'; DROP TABLE library_tracks; --`. Les recopier dans le SQL ouvrirait une
/// injection par simple ajout d'un fichier dans la bibliothèque.
///
/// L'expression rendue ne contient donc jamais le nom : elle contient un `?`,
/// et le nom part comme valeur liée. Le moteur ne peut alors plus le
/// réinterpréter comme du code, quelle que soit sa forme.
///
/// # Le coût, mesuré
/// Trier sur un tag libre demande une sous-requête corrélée — `custom_tags`
/// est une liste de paires, pas un objet. Sur neuf mille pistes avec toutes
/// les jointures, la page revient en cinquante-six millisecondes. C'est ce
/// chiffre qui a décidé de trier en base plutôt qu'en mémoire : trier en
/// mémoire n'aurait rangé que les cent lignes déjà chargées, en donnant au
/// résultat l'apparence d'un tri complet.
fn resolve_sort(sort_by: Option<&str>) -> (String, Option<String>) {
    // Tag libre : la valeur se cherche dans la liste de paires.
    if let Some(nom) = sort_by.and_then(|s| s.strip_prefix("tag:custom:")) {
        return (
            "(SELECT json_extract(je.value, '$[1]') \
              FROM json_each(json_extract(lt.tags, '$.custom_tags')) je \
              WHERE json_extract(je.value, '$[0]') = ? LIMIT 1) COLLATE NOCASE"
                .to_string(),
            Some(nom.to_string()),
        );
    }

    // Champ nommé de la structure de tags.
    if let Some(champ) = sort_by.and_then(|s| s.strip_prefix("tag:")) {
        return (
            "json_extract(lt.tags, ?) COLLATE NOCASE".to_string(),
            Some(format!("$.{champ}")),
        );
    }

    // Colonnes de la bibliothèque. La liste est close : tout ce qui n'y figure
    // pas retombe sur l'ordre naturel plutôt que d'atteindre le SQL.
    let expr = match sort_by {
        Some("title") => "lt.title_normalized",
        Some("artist") => "a.name COLLATE NOCASE",
        Some("album") => "la.title COLLATE NOCASE",
        Some("album_artist") => "lc.album_artist COLLATE NOCASE",
        Some("year") => "lc.year",
        Some("genre") => "lc.genre COLLATE NOCASE",
        Some("duration") => "lt.duration",
        // `date` est le nom historique employé par la barre de filtres ;
        // `created_at` celui de la colonne. Les deux mènent au même endroit.
        Some("date") | Some("created_at") => "lt.created_at",
        Some("last_played_at") => "lt.last_played_at",
        Some("play_count") => "lt.play_count",
        // « index » est la clé de la colonne affichée, `track_number` celle du
        // champ. Les deux mènent au numéro de piste.
        Some("track_number") | Some("index") => "lt.track_number",
        Some("disc_number") => "lt.disc_number",
        Some("bitrate") => "lt.bitrate",
        Some("sample_rate") => "lt.sample_rate",
        Some("bits_per_sample") => "lc.bits_per_sample",
        Some("channels") => "lc.channels",
        Some("audio_format") => "lc.audio_format COLLATE NOCASE",
        Some("extension") => "lf.extension COLLATE NOCASE",
        Some("file_size") => "COALESCE(lc.file_size, lf.size)",
        Some("filename") => "lf.filename COLLATE NOCASE",
        Some("path") => "lf.path COLLATE NOCASE",
        // `IS NULL` d'abord : les pistes jamais notées se rangent en bas quel
        // que soit le sens, comme partout ailleurs dans l'application.
        Some("rating") => "lt.rating IS NULL, lt.rating",
        _ => "a.name COLLATE NOCASE, la.title COLLATE NOCASE, lt.disc_number, lt.track_number",
    };

    (expr.to_string(), None)
}

#[tauri::command]
pub async fn get_tracks_paginated(
    state: State<'_, AppState>,
    library_id: i64,
    offset: i64,
    limit: i64,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    filter: Option<String>,
    missing_cover: Option<bool>,
) -> Result<PaginatedTracks, String> {

    let (sort_col, sort_bind) = resolve_sort(sort_by.as_deref());

    let dir = match sort_dir.as_deref() {
        Some("desc") => "DESC",
        _ => "ASC",
    };

    let (tracks, total) = LibraryTrackRepository::find_tracks_paginated(
        &state.pool, library_id, offset, limit, &sort_col, sort_bind.as_deref(), dir,
        filter.as_deref(), missing_cover.unwrap_or(false),
    ).await.map_err(|e| format!("Failed to get tracks: {}", e))?;

    Ok(PaginatedTracks { tracks, total })
}

#[tauri::command]
pub async fn get_tracks_by_album(
    state: State<'_, AppState>,
    library_id: i64,
    library_album_id: String
) -> Result<Vec<TrackListView>, String> {
    LibraryTrackRepository::find_all_tracks_album_by_library_id(&state.pool, library_id, library_album_id)
        .await
        .map_err(|e| format!("Failed to get album tracks: {}", e))
}

#[tauri::command]
pub async fn get_album(
    state: State<'_, AppState>,
    library_album_id: String
) -> Result<AlbumDetailView, String> {
    LibraryAlbumRepository::find_album_by_id(&state.pool, library_album_id)
        .await
        .map_err(|e| format!("Failed to get album: {}", e))
}

#[tauri::command]
pub async fn get_albums(
    state: State<'_, AppState>,
    library_id: i64,
    missing_cover: Option<bool>
) -> Result<Vec<AlbumListView>, String> {
    LibraryAlbumRepository::find_all_albums_by_library_id(&state.pool, library_id, missing_cover)
        .await
        .map_err(|e| format!("Failed to get albums: {}", e))
}

#[tauri::command]
pub async fn get_artist(
    state: State<'_, AppState>,
    library_artist_id: String
) -> Result<ArtistDetailView, String> {
    LibraryArtistRepository::find_artist_by_id(&state.pool, library_artist_id)
        .await
        .map_err(|e| format!("Failed to get artist: {}", e))
}

#[tauri::command]
pub async fn get_tracks_by_artist(
    state: State<'_, AppState>,
    library_id: i64,
    artist_id: String,
) -> Result<Vec<TrackListView>, String> {
    LibraryTrackRepository::find_tracks_view_by_artist_id(&state.pool, library_id, &artist_id)
        .await
        .map_err(|e| format!("Failed to get artist tracks: {}", e))
}

#[tauri::command]
pub async fn get_tracks_by_artist_paginated(
    state: State<'_, AppState>,
    library_id: i64,
    artist_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<PaginatedTracks, String> {
    let limit = limit.unwrap_or(20);
    let offset = offset.unwrap_or(0);

    let (tracks, total) = tokio::try_join!(
        LibraryTrackRepository::find_tracks_view_by_artist_id_paginated(&state.pool, library_id, &artist_id, limit, offset),
        LibraryTrackRepository::count_tracks_by_artist_id(&state.pool, library_id, &artist_id),
    ).map_err(|e| format!("Failed to get artist tracks: {}", e))?;

    Ok(PaginatedTracks { tracks, total })
}

#[tauri::command]
pub async fn get_albums_by_artist(
    state: State<'_, AppState>,
    library_id: i64,
    artist_id: String,
) -> Result<Vec<AlbumListView>, String> {
    LibraryAlbumRepository::find_albums_by_artist_id(&state.pool, library_id, &artist_id)
        .await
        .map_err(|e| format!("Failed to get artist albums: {}", e))
}

#[tauri::command]
pub async fn get_artists(
    state: State<'_, AppState>,
    library_id: i64
) -> Result<Vec<ArtistListView>, String> {
    LibraryArtistRepository::find_all_artists_by_library_id(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get artists: {}", e))
}

#[tauri::command]
pub async fn get_similar_artists(
    state: State<'_, AppState>,
    library_id: i64,
    artist_id: String,
    limit: Option<i64>,
) -> Result<Vec<ArtistListView>, String> {
    let limit = limit.unwrap_or(10);
    LibraryArtistRepository::find_similar_artists(&state.pool, library_id, &artist_id, limit)
        .await
        .map_err(|e| format!("Failed to get similar artists: {}", e))
}

/// Vérifie si une URL Deezer est une image par défaut (pas de vraie photo)
/// Pattern : /artist//500x500 (double slash = hash vide)
fn is_deezer_default_image(url: &str) -> bool {
    url.contains("/artist//") || url.contains("/artist/d41d8cd98f00b204e9800998ecf8427e/")
}

/// Récupère l'image d'un artiste via Deezer API (lazy loading + cache DB)
/// - Si image_url est déjà en DB → retourne directement
/// - Si NULL → fetch Deezer, stocke en DB, retourne l'URL
/// - Si "" → déjà cherché, pas trouvé → retourne None
#[tauri::command]
pub async fn fetch_artist_image(
    state: State<'_, AppState>,
    artist_id: String,
    artist_name: String,
) -> Result<Option<String>, String> {

    // 1. Vérifier le cache DB
    let existing = ArtistRepository::get_image_url(&state.pool, &artist_id)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    match existing {
        Some(url) if !url.is_empty() => return Ok(Some(url)), // Déjà en cache
        Some(_) => return Ok(None), // "" = déjà cherché, pas trouvé
        None => {} // NULL = jamais cherché
    }

    // 2. Fetch Deezer API (timeout 3s)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let encoded_name = urlencoding::encode(&artist_name);
    let url = format!("https://api.deezer.com/search/artist?q={}&limit=1", encoded_name);

    let deezer_url = match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                // Prendre la plus grande image disponible (ignorer les images par défaut)
                json["data"][0]["picture_xl"]
                    .as_str()
                    .or_else(|| json["data"][0]["picture_big"].as_str())
                    .or_else(|| json["data"][0]["picture_medium"].as_str())
                    .filter(|url| !is_deezer_default_image(url))
                    .map(|s| s.to_string())
            } else {
                None
            }
        }
        Err(e) => {
            log::error!("Deezer API error for '{}': {}", artist_name, e);
            None
        }
    };

    log::info!("Deezer image pour '{}': {:?}", artist_name, deezer_url);

    // 3. Télécharger l'image en local si trouvée
    let local_path = if let Some(ref img_url) = deezer_url {
        match client.get(img_url).send().await {
            Ok(resp) => {
                if let Ok(bytes) = resp.bytes().await {
                    let mut artists_dir = dirs::data_dir().unwrap_or_default();
                    artists_dir.push("com.larevuegeek.rustmusic");
                    artists_dir.push("covers");
                    artists_dir.push("artists");

                    let filename = format!("artist_{}.jpg", artist_id.replace("-", ""));

                    match save_artist_image(&artists_dir, &filename, &bytes) {
                        Ok(full_path) => Some(full_path),
                        Err(e) => {
                            log::warn!("Sauvegarde image artiste échouée: {}", e);
                            Some(img_url.clone())
                        }
                    }
                } else {
                    Some(img_url.clone())
                }
            }
            Err(_) => Some(img_url.clone())
        }
    } else {
        None
    };

    // 4. Stocker en DB (chemin local ou "" si pas trouvé)
    let store_value = local_path.as_deref().unwrap_or("");
    ArtistRepository::update_image_url(&state.pool, &artist_id, store_value)
        .await
        .map_err(|e| format!("Failed to update artist image: {}", e))?;

    Ok(local_path)
}

/// Fetch toutes les images artistes manquantes via Deezer (batch)
/// Émet des events de progression au frontend
#[tauri::command]
pub async fn fetch_all_artist_images(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    force: Option<bool>,
) -> Result<u32, String> {

    // Si force=true, on reset toutes les URLs pour re-télécharger
    if force.unwrap_or(false) {
        ArtistRepository::reset_all_image_urls(&state.pool)
            .await
            .map_err(|e| format!("DB error reset: {}", e))?;
    }

    let artists = ArtistRepository::find_without_image(&state.pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    let total = artists.len();
    if total == 0 {
        return Ok(0);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP error: {}", e))?;

    let mut found: u32 = 0;

    for (i, artist) in artists.iter().enumerate() {
        // Progression
        if let Err(e) = app.emit("artist-image-progress", serde_json::json!({
            "current": i + 1,
            "total": total,
            "name": artist.name,
        })) {
            log::warn!("emit artist-image-progress failed: {}", e);
        }

        let encoded = urlencoding::encode(&artist.name);
        let url = format!("https://api.deezer.com/search/artist?q={}&limit=1", encoded);

        let deezer_url = match client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    json["data"][0]["picture_xl"]
                        .as_str()
                        .or_else(|| json["data"][0]["picture_big"].as_str())
                        .or_else(|| json["data"][0]["picture_medium"].as_str())
                        .filter(|url| !is_deezer_default_image(url))
                        .map(|s| s.to_string())
                } else { None }
            }
            Err(_) => None,
        };

        let store_value = if let Some(ref img_url) = deezer_url {
            match client.get(img_url).send().await {
                Ok(resp) => {
                    if let Ok(bytes) = resp.bytes().await {
                        let mut artists_dir = dirs::data_dir().unwrap_or_default();
                        artists_dir.push("com.larevuegeek.rustmusic");
                        artists_dir.push("covers");
                        artists_dir.push("artists");

                        let filename = format!("artist_{}.jpg", artist.id.replace("-", ""));

                        match save_artist_image(&artists_dir, &filename, &bytes) {
                            Ok(full_path) => {
                                found += 1;
                                full_path
                            }
                            Err(e) => {
                                log::warn!("Sauvegarde image artiste échouée: {}", e);
                                img_url.clone()
                            }
                        }
                    } else { img_url.clone() }
                }
                Err(_) => img_url.clone(),
            }
        } else {
            String::new()
        };

        if let Err(e) = ArtistRepository::update_image_url(&state.pool, &artist.id, &store_value).await {
            log::error!("Failed to update artist image for {}: {}", artist.name, e);
        }

        // Notifier le frontend que l'image est prête (fadeIn)
        if !store_value.is_empty() {
            let _ = app.emit("artist-image-ready", serde_json::json!({
                "artist_id": artist.id,
                "image_url": store_value,
            }));
        }

        // Délai entre chaque requête pour ne pas surcharger Deezer
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    if let Err(e) = app.emit("artist-image-complete", serde_json::json!({
        "found": found,
        "total": total,
    })) {
        log::warn!("emit artist-image-complete failed: {}", e);
    }

    Ok(found)
}

// ============================================================================
// CUSTOM COVER
// ============================================================================

/// Set a custom cover for an album from a local image file
#[tauri::command]
pub async fn set_album_cover(
    state: State<'_, AppState>,
    album_id: String,
    image_path: String,
) -> Result<String, String> {
    let src = std::path::PathBuf::from(&image_path);
    if !src.exists() {
        return Err("Image file not found".to_string());
    }

    let image_data = std::fs::read(&src)
        .map_err(|e| format!("Failed to read image: {}", e))?;

    let mut covers_dir = dirs::data_dir().unwrap_or_default();
    covers_dir.push("com.larevuegeek.rustmusic");
    covers_dir.push("covers");
    covers_dir.push("albums");

    let saved_path = crate::helper::library::thumbnail_helper::thumbnail_saver(&covers_dir, &image_data, false)
        .map_err(|e| format!("Failed to save cover: {}", e))?;

    LibraryAlbumRepository::update_cover_url_by_id(&state.pool, &album_id, &saved_path)
        .await
        .map_err(|e| format!("Failed to update DB: {}", e))?;

    Ok(saved_path)
}

// ============================================================================
// ALBUM COVERS (Deezer)
// ============================================================================

#[derive(Serialize)]
pub struct DeezerCoverResult {
    pub title: String,
    pub artist: String,
    pub cover_small: String,
    pub cover_xl: String,
}

/// Recherche de covers album sur Deezer (retourne plusieurs résultats)
#[tauri::command]
pub async fn search_deezer_covers(
    query: String,
    limit: Option<i64>,
) -> Result<Vec<DeezerCoverResult>, String> {
    let limit = limit.unwrap_or(12);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP error: {}", e))?;

    let encoded = urlencoding::encode(&query);
    let url = format!("https://api.deezer.com/search/album?q={}&limit={}", encoded, limit);

    let resp = client.get(&url).send().await
        .map_err(|e| format!("Deezer API error: {}", e))?;

    let json = resp.json::<serde_json::Value>().await
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let results = json["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let title = item["title"].as_str()?.to_string();
                    let artist = item["artist"]["name"].as_str().unwrap_or("").to_string();
                    let cover_small = item["cover_medium"].as_str()?.to_string();
                    let cover_xl = item["cover_xl"].as_str()
                        .or_else(|| item["cover_big"].as_str())?.to_string();
                    Some(DeezerCoverResult { title, artist, cover_small, cover_xl })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(results)
}

/// Télécharge une cover depuis une URL et l'applique à un album
#[tauri::command]
pub async fn apply_deezer_cover(
    state: State<'_, AppState>,
    album_id: String,
    cover_url: String,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP error: {}", e))?;

    let bytes = client.get(&cover_url).send().await
        .map_err(|e| format!("Download error: {}", e))?
        .bytes().await
        .map_err(|e| format!("Read error: {}", e))?;

    let mut covers_dir = dirs::data_dir().unwrap_or_default();
    covers_dir.push("com.larevuegeek.rustmusic");
    covers_dir.push("covers");
    covers_dir.push("albums");

    let saved_path = crate::helper::library::thumbnail_helper::thumbnail_saver(&covers_dir, &bytes.to_vec(), false)
        .map_err(|e| format!("Save error: {}", e))?;

    LibraryAlbumRepository::update_cover_url_by_id(&state.pool, &album_id, &saved_path)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    Ok(saved_path)
}

/// Fetch la cover d'un album via Deezer API
#[tauri::command]
pub async fn fetch_album_cover(
    state: State<'_, AppState>,
    album_id: String,
    album_title: String,
    artist_name: Option<String>,
) -> Result<Option<String>, String> {

    // Fetch Deezer (toujours, même si une cover existe déjà)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| format!("HTTP error: {}", e))?;

    let query = if let Some(ref artist) = artist_name {
        format!("{} {}", artist, album_title)
    } else {
        album_title.clone()
    };
    let encoded = urlencoding::encode(&query);
    let url = format!("https://api.deezer.com/search/album?q={}&limit=1", encoded);

    let deezer_url = match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                json["data"][0]["cover_xl"]
                    .as_str()
                    .or_else(|| json["data"][0]["cover_big"].as_str())
                    .or_else(|| json["data"][0]["cover_medium"].as_str())
                    .map(|s| s.to_string())
            } else { None }
        }
        Err(e) => {
            log::error!("Deezer album API error for '{}': {}", album_title, e);
            None
        }
    };

    // 3. Télécharger et sauvegarder
    let local_path = if let Some(ref img_url) = deezer_url {
        match client.get(img_url).send().await {
            Ok(resp) => {
                if let Ok(bytes) = resp.bytes().await {
                    let mut covers_dir = dirs::data_dir().unwrap_or_default();
                    covers_dir.push("com.larevuegeek.rustmusic");
                    covers_dir.push("covers");
                    covers_dir.push("albums");

                    match crate::helper::library::thumbnail_helper::thumbnail_saver(&covers_dir, &bytes.to_vec(), false) {
                        Ok(full_path) => Some(full_path),
                        Err(e) => {
                            log::warn!("Save album cover failed: {}", e);
                            Some(img_url.clone())
                        }
                    }
                } else { Some(img_url.clone()) }
            }
            Err(_) => Some(img_url.clone()),
        }
    } else { None };

    // 4. Update DB
    let store_value = local_path.as_deref().unwrap_or("");
    if !store_value.is_empty() {
        LibraryAlbumRepository::update_cover_url_by_id(&state.pool, &album_id, store_value)
            .await
            .map_err(|e| format!("Failed to update album cover: {}", e))?;
    }

    Ok(local_path)
}

/// Fetch toutes les covers albums manquantes via Deezer (batch)
#[tauri::command]
pub async fn fetch_all_album_covers(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<u32, String> {
    let albums = LibraryAlbumRepository::find_albums_without_cover(&state.pool, library_id)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    let total = albums.len();
    if total == 0 { return Ok(0); }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP error: {}", e))?;

    let mut found: u32 = 0;

    for (i, album) in albums.iter().enumerate() {
        let _ = app.emit("album-cover-progress", serde_json::json!({
            "current": i + 1,
            "total": total,
            "name": album.title,
        }));

        // On cherche par artiste_id → nom
        let artist: Option<String> =
            ArtistRepository::find_name_by_id(&state.pool, &album.artist_id)
                .await
                .ok()
                .flatten();

        let query = if let Some(ref a) = artist {
            format!("{} {}", a, album.title)
        } else {
            album.title.clone()
        };
        let encoded = urlencoding::encode(&query);
        let url = format!("https://api.deezer.com/search/album?q={}&limit=1", encoded);

        let deezer_url = match client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    json["data"][0]["cover_xl"]
                        .as_str()
                        .or_else(|| json["data"][0]["cover_big"].as_str())
                        .map(|s| s.to_string())
                } else { None }
            }
            Err(_) => None,
        };

        let store_value = if let Some(ref img_url) = deezer_url {
            match client.get(img_url).send().await {
                Ok(resp) => {
                    if let Ok(bytes) = resp.bytes().await {
                        let mut covers_dir = dirs::data_dir().unwrap_or_default();
                        covers_dir.push("com.larevuegeek.rustmusic");
                        covers_dir.push("covers");
                        covers_dir.push("albums");

                        match crate::helper::library::thumbnail_helper::thumbnail_saver(&covers_dir, &bytes.to_vec(), false) {
                            Ok(full_path) => {
                                found += 1;
                                full_path
                            }
                            Err(_) => img_url.clone(),
                        }
                    } else { img_url.clone() }
                }
                Err(_) => img_url.clone(),
            }
        } else { String::new() };

        if !store_value.is_empty() {
            let _ = LibraryAlbumRepository::update_cover_url_by_id(&state.pool, &album.id, &store_value).await;

            let _ = app.emit("album-cover-ready", serde_json::json!({
                "album_id": album.id,
                "cover_url": store_value,
            }));
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    let _ = app.emit("album-cover-complete", serde_json::json!({
        "found": found,
        "total": total,
    }));

    Ok(found)
}

// ============================================================================
// GENRES
// ============================================================================

/// Un tag proposable en colonne, avec le nombre de pistes qui le renseignent.
#[derive(Debug, Serialize)]
pub struct TagColumnCandidate {
    /// Clé stable : soit un champ nommé (`composer`), soit un tag libre
    /// préfixé (`custom:Label`). Le préfixe évite qu'un tag nommé « genre »
    /// écrit à la main dans un fichier n'entre en collision avec le champ.
    pub key: String,
    /// Nombre de pistes où ce tag porte une valeur non vide.
    pub filled: i64,
}

/// Recense les tags réellement présents dans une bibliothèque.
///
/// # Pourquoi compter plutôt que lister
/// La structure des tags compte une quarantaine de champs nommés, dont la
/// plupart ne sont jamais renseignés — sur une discothèque réelle, `conductor`,
/// `mood` ou `isrc` sont vides partout. Proposer la liste théorique noierait
/// les cinq tags utiles sous trente-cinq inutiles.
///
/// On rend donc ce qui existe, avec son effectif, et l'interface range les plus
/// répandus en tête.
///
/// # Le coût
/// Une lecture de la colonne `tags` de toute la bibliothèque, et autant
/// d'analyses JSON. Sur quatorze mille pistes, l'ordre de grandeur est la
/// fraction de seconde — acceptable pour une liste ouverte à la demande, et
/// c'est pourquoi elle n'est pas calculée au démarrage.
#[tauri::command]
pub async fn get_library_tag_keys(
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<Vec<TagColumnCandidate>, String> {
    let lignes: Vec<(Option<String>,)> =
        sqlx::query_as("SELECT tags FROM library_tracks WHERE library_id = ?")
            .bind(library_id)
            .fetch_all(&state.pool)
            .await
            .map_err(|e| format!("Failed to read tags: {e}"))?;

    let mut effectifs: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for (brut,) in lignes {
        let Some(json) = brut else { continue };
        let Ok(valeur) = serde_json::from_str::<serde_json::Value>(&json) else {
            continue;
        };
        let Some(objet) = valeur.as_object() else { continue };

        for (cle, val) in objet {
            match cle.as_str() {
                // Les images n'ont rien à faire dans une colonne de texte, et
                // les tags libres sont dépouillés juste en dessous.
                "attached_images" => continue,
                "custom_tags" => {
                    if let Some(paires) = val.as_array() {
                        for paire in paires {
                            let Some(paire) = paire.as_array() else { continue };
                            let (Some(k), Some(v)) = (paire.first(), paire.get(1)) else {
                                continue;
                            };
                            if let (Some(k), Some(v)) = (k.as_str(), v.as_str()) {
                                if !v.trim().is_empty() {
                                    *effectifs.entry(format!("custom:{k}")).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
                _ => {
                    if est_renseigne(val) {
                        *effectifs.entry(cle.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    let mut sortie: Vec<TagColumnCandidate> = effectifs
        .into_iter()
        .map(|(key, filled)| TagColumnCandidate { key, filled })
        .collect();

    // Les plus répandus d'abord ; à effectif égal, l'ordre alphabétique, pour
    // que deux appels successifs rendent la même liste.
    sortie.sort_by(|a, b| b.filled.cmp(&a.filled).then_with(|| a.key.cmp(&b.key)));

    Ok(sortie)
}

/// Un tag vaut d'être proposé s'il porte autre chose que du vide.
fn est_renseigne(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => false,
        serde_json::Value::String(s) => !s.trim().is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
        _ => true,
    }
}

#[tauri::command]
pub async fn get_tracks_by_genre(
    state: State<'_, AppState>,
    library_id: i64,
    genre: String,
) -> Result<Vec<TrackListView>, String> {
    LibraryTrackRepository::find_tracks_by_genre(&state.pool, library_id, &genre)
        .await
        .map_err(|e| format!("get_tracks_by_genre: {}", e))
}

#[tauri::command]
pub async fn get_genres(
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<Vec<GenreView>, String> {
    let rows = LibraryGenreRepository::find_all_genres(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get genres: {}", e))?;

    let mut genres = Vec::with_capacity(rows.len());
    for (name, total_albums, total_tracks) in rows {
        let covers = LibraryGenreRepository::find_genre_covers(&state.pool, &name, library_id).await;
        genres.push(GenreView { name, total_albums, total_tracks, covers });
    }

    Ok(genres)
}

// ============================================================================
// STATS
// ============================================================================

#[tauri::command]
pub async fn get_library_stats(
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<LibraryStats, String> {
    let (total_tracks, total_duration_sec, total_size_bytes, avg_bitrate, total_play_count) =
        LibraryStatsRepository::get_main_counters(&state.pool, library_id)
            .await
            .map_err(|e| format!("stats: {}", e))?;

    let total_albums = LibraryStatsRepository::count_albums(&state.pool, library_id).await;
    let total_artists = LibraryStatsRepository::count_artists(&state.pool, library_id).await;
    let total_genres = LibraryStatsRepository::count_genres(&state.pool, library_id).await;

    let formats = LibraryStatsRepository::get_format_stats(&state.pool, library_id).await;
    let quality_hires = LibraryStatsRepository::count_quality_hires(&state.pool, library_id).await;
    let quality_lossless = LibraryStatsRepository::count_quality_lossless(&state.pool, library_id).await;
    let quality_lossy = total_tracks - quality_hires - quality_lossless;

    let top_genres = LibraryStatsRepository::get_top_genres(&state.pool, library_id).await;
    let top_artists = LibraryStatsRepository::get_top_artists(&state.pool, library_id).await;
    let top_played_rows = LibraryStatsRepository::get_top_played(&state.pool, library_id).await;

    Ok(LibraryStats {
        total_tracks, total_albums, total_artists, total_genres,
        total_duration_sec, total_size_bytes, avg_bitrate, total_play_count,
        formats: formats.into_iter().map(|(name, count)| FormatStat { name, count }).collect(),
        top_genres: top_genres.into_iter().map(|(name, count)| GenreStat { name, count }).collect(),
        top_artists: top_artists.into_iter().map(|(name, count)| ArtistStat { name, count }).collect(),
        top_played: top_played_rows.into_iter().map(|(title, artist, play_count, thumbnail_path)| {
            TrackPlayStat { title, artist, play_count, thumbnail_path }
        }).collect(),
        quality_hires, quality_lossless, quality_lossy,
    })
}
#[cfg(test)]
mod tests_tri {
    use super::resolve_sort;

    #[test]
    fn une_colonne_connue_donne_son_expression_sans_liaison() {
        let (expr, bind) = resolve_sort(Some("duration"));
        assert_eq!(expr, "lt.duration");
        assert!(bind.is_none());
    }

    #[test]
    fn une_cle_inconnue_retombe_sur_l_ordre_naturel() {
        // Le point important n'est pas la valeur rendue mais qu'aucune clé
        // étrangère ne puisse atteindre le SQL : tout ce qui n'est pas prévu
        // donne la même expression close.
        let (attendu, _) = resolve_sort(None);
        for cle in ["", "lt.title; DROP TABLE library_tracks", "../../etc", "RANDOM()"] {
            let (expr, bind) = resolve_sort(Some(cle));
            assert_eq!(expr, attendu, "clé : {cle}");
            assert!(bind.is_none(), "clé : {cle}");
        }
    }

    #[test]
    fn un_champ_nomme_part_en_chemin_lie() {
        let (expr, bind) = resolve_sort(Some("tag:composer"));
        assert_eq!(expr, "json_extract(lt.tags, ?) COLLATE NOCASE");
        assert_eq!(bind.as_deref(), Some("$.composer"));
    }

    #[test]
    fn un_tag_libre_part_en_valeur_liee() {
        let (expr, bind) = resolve_sort(Some("tag:custom:Label"));
        assert!(expr.contains("json_each"), "expression : {expr}");
        assert_eq!(bind.as_deref(), Some("Label"));
    }

    #[test]
    fn le_nom_d_un_tag_ne_rejoint_jamais_le_texte_de_la_requete() {
        // Un fichier de la bibliothèque peut porter n'importe quel nom de tag,
        // y compris hostile. L'expression rendue ne doit jamais le contenir :
        // il ne circule que comme valeur liée, que le moteur ne réinterprète
        // pas comme du code.
        let mechants = [
            "x'; DROP TABLE library_tracks; --",
            "a\" OR 1=1 --",
            "'||(SELECT value FROM settings)||'",
        ];

        for nom in mechants {
            for cle in [format!("tag:custom:{nom}"), format!("tag:{nom}")] {
                let (expr, bind) = resolve_sort(Some(&cle));
                assert!(
                    !expr.contains(nom),
                    "le nom a fuité dans l'expression : {expr}"
                );
                assert!(bind.is_some(), "le nom doit partir en liaison : {cle}");
                assert!(
                    bind.as_deref().unwrap().contains(nom),
                    "le nom doit être porté par la liaison"
                );
            }
        }
    }
}
