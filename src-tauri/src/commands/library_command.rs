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
use crate::helper::library::recuperation_images::{alleger, chercher, telecharger, POCHETTES, PORTRAITS};
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
use crate::repository::library::library_track_repository::{LibraryTrackRepository, TrackFilters};
use crate::mapper::library::artist::track_artist_view::TrackArtistView;
use crate::repository::library::library_track_artist_repository::LibraryTrackArtistRepository;
use crate::service::library::artist_link_repair::ArtistLinkReport;
use crate::service::library::library_service::{IMPORT, LibrarySaveContext, RescanProgress, create_context, lister_fichiers, save_dir_to_library, save_files_to_library, save_track_to_library};
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
    
    let Some(jeton) = IMPORT.demarrer() else { return Err("deja_en_cours".into()) };
    let tracks: Vec<TrackListView> = save_dir_to_library(app, &state.pool, library_id, directory, &jeton).await?;

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


/// Où trouver, en bibliothèque, le morceau joué depuis ce chemin.
///
/// Le lecteur ne connaît que le fichier : sans ce pont, son titre, son artiste
/// et son album ne mènent nulle part. Un chemin absent de la bibliothèque —
/// fichier ouvert à la volée — ne rend simplement rien.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct TrackLocation {
    pub path: String,
    pub library_id: i64,
    pub library_track_id: String,
    pub artist_id: Option<String>,
    pub library_album_id: Option<String>,
}

#[tauri::command]
pub async fn get_track_locations(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<TrackLocation>, String> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }

    // Les `?` sont générés, jamais les valeurs : elles restent liées.
    let trous = vec!["?"; paths.len()].join(",");
    let sql = format!(
        "SELECT f.path AS path,
                t.library_id AS library_id,
                t.id AS library_track_id,
                t.artist_id AS artist_id,
                t.library_album_id AS library_album_id
         FROM library_tracks t
         JOIN library_files f ON f.id = t.file_id
         WHERE f.path IN ({trous})"
    );

    let mut requete = sqlx::query_as::<_, TrackLocation>(&sql);
    for p in &paths {
        requete = requete.bind(p);
    }

    requete
        .fetch_all(&state.pool)
        .await
        .map_err(|e| format!("Failed to locate tracks: {e}"))
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

    let Some(jeton) = IMPORT.demarrer() else { return Err("deja_en_cours".into()) };

    let library_name = LibraryRepository::find_library_by_id(&state.pool, library_id)
        .await
        .map(|library| library.name)
        .unwrap_or_default();

    let _ = app.emit("rescan-start", serde_json::json!({
        "library_id": library_id,
        "library_name": library_name,
    }));

    // Tout est listé d'abord : la progression porte sur la bibliothèque entière.
    let mut lots: Vec<_> = Vec::with_capacity(dirs.len());
    for dir in dirs {
        let Some(files) = lister_fichiers(&app, library_id, &dir.path).await else { break };
        lots.push((dir, files));
    }

    let total: usize = lots.iter().map(|(_, files)| files.len()).sum();
    let mut progression = RescanProgress::new(library_id, total);
    progression.emettre(&app);

    let mut all_tracks: Vec<TrackListView> = Vec::new();

    // Re-scanner chaque dossier (les fichiers inchangés sont sautés via le cache)
    for (dir, files) in lots {
        if jeton.annule() {
            break;
        }
        match save_files_to_library(app.clone(), &state.pool, library_id, dir.path, files, Some(&mut progression), &jeton).await {
            Ok(tracks) => all_tracks.extend(tracks),
            Err(e) => log::error!("Erreur rescan dossier {}: {}", dir.name, e),
        }
    }

    let _ = app.emit("rescan-complete", serde_json::json!({
        "library_id": library_id,
        "library_name": library_name,
        "cancelled": jeton.annule(),
    }));

    Ok(all_tracks)
}

/// Date du dernier scan, rendue comme serde rend nos `DateTime<Utc>` : `2026-09-30T12:34:56Z`.
#[tauri::command]
pub async fn get_library_last_scan(
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<Option<String>, String> {
    let last_scan = LibraryDirRepository::find_last_scan_at(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get last scan: {}", e))?;

    Ok(last_scan.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)))
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

/// Pistes sous un sous-dossier quelconque (sélection d'un dossier dans la vue Dossiers).
#[tauri::command]
pub async fn get_tracks_under_path(
    state: State<'_, AppState>,
    library_id: i64,
    path: String,
) -> Result<Vec<TrackListView>, String> {
    LibraryTrackRepository::find_tracks_under_path(&state.pool, library_id, &path)
        .await
        .map_err(|e| format!("Failed to get tracks under path: {}", e))
}

// ============================================================================
// EXPLORATEUR DE FICHIERS (restreint aux dossiers importés)
// ============================================================================

const AUDIO_EXTENSIONS: &[&str] = &[
    // Ce que le lecteur sait décoder (ALAC est dans le .m4a ; WMA et APE ne se lisent pas).
    "mp3", "flac", "wav", "ogg", "m4a", "aac", "opus", "aiff", "aif", "dsf", "dff"
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

    // Les chemins rendus gardent la forme du dossier importé : canonicalize change
    // un lecteur réseau (S:\) en \\?\UNC\…, que la bibliothèque ne reconnaît pas.
    let mut base: Option<PathBuf> = None;
    let mut profondeur = 0usize;
    for d in &dirs {
        let Ok(canonical_dir) = std::fs::canonicalize(&d.path) else { continue };
        if let Ok(reste) = canonical_path.strip_prefix(&canonical_dir) {
            let n = canonical_dir.as_os_str().len();
            if base.is_none() || n > profondeur {
                profondeur = n;
                base = Some(if reste.as_os_str().is_empty() { PathBuf::from(&d.path) } else { PathBuf::from(&d.path).join(reste) });
            }
        }
    }

    let Some(base) = base else {
        return Err("Accès refusé : ce dossier n'est pas dans la bibliothèque".to_string());
    };

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
        let path_string = base.join(&name).to_string_lossy().to_string();

        if file_type.is_dir() {
            entries.push(DirEntry {
                name,
                path: path_string,
                is_dir: true,
                size: 0,
                extension: None,
                title: None,
                artist: None,
                duration: None,
                thumbnail_path: None,
                track_id: None,
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
                        title: None,
                        artist: None,
                        duration: None,
                        thumbnail_path: None,
                        track_id: None,
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

    // Titre, artiste, durée et pochette connus de la bibliothèque : la file d'attente en hérite.
    let chemins: Vec<String> = entries.iter().filter(|e| !e.is_dir).map(|e| e.path.clone()).collect();
    let mut fiches = LibraryTrackRepository::find_fiches_by_paths(&state.pool, Some(library_id), &chemins).await;
    for e in entries.iter_mut().filter(|e| !e.is_dir) {
        if let Some(f) = fiches.remove(&e.path) {
            e.title = f.title;
            e.artist = f.artist;
            e.duration = f.duration;
            e.thumbnail_path = f.thumbnail_path;
            e.track_id = f.track_id;
        }
    }

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

    let Some(jeton) = IMPORT.demarrer() else { return Err("deja_en_cours".into()) };
    save_dir_to_library(app, &state.pool, library_id, dir.path, &jeton).await
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

/// Ecrit la vignette de la pochette embarquee d'un fichier.
///
/// Remplace l'aller-retour par `save_thumbnail`, ou les octets faisaient
/// Rust -> JSON -> JS -> JSON -> Rust pour finir dans un fichier.
#[tauri::command]
pub fn save_thumbnail_from_file(
    app: tauri::AppHandle,
    path: String,
) -> Result<Option<String>, String> {
    let audio = crate::core::audio_analyser::audio_analyser::AudioAnalyser::analyse_audio_file(&PathBuf::from(&path))
        .map_err(|e| format!("Analyse de {path} : {e}"))?;

    let Some(image) = audio.tags.attached_images.first() else {
        return Ok(None);
    };
    if image.image_data.is_empty() {
        return Ok(None);
    }

    let covers_dir: PathBuf = app
        .path()
        .resolve("covers", BaseDirectory::AppData)
        .map_err(|e| e.to_string())?;

    thumbnail_saver(&covers_dir, &image.image_data, false).map(Some)
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
        // Colonne « Qualité » : la profondeur d'abord, la fréquence départage.
        Some("quality") => "(COALESCE(lc.bits_per_sample, 0) * 1000000 + COALESCE(lt.sample_rate, lc.sample_rate, 0))",
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
    quality: Option<String>,
    favorites: Option<bool>,
    genre: Option<String>,
) -> Result<PaginatedTracks, String> {

    let (sort_col, sort_bind) = resolve_sort(sort_by.as_deref());
    let extra = TrackFilters { quality: quality.as_deref(), favorites: favorites.unwrap_or(false), genre: genre.as_deref() };

    let dir = match sort_dir.as_deref() {
        Some("desc") => "DESC",
        _ => "ASC",
    };

    let (tracks, total) = LibraryTrackRepository::find_tracks_paginated(
        &state.pool, library_id, offset, limit, &sort_col, sort_bind.as_deref(), dir,
        filter.as_deref(), missing_cover.unwrap_or(false), &extra,
    ).await.map_err(|e| format!("Failed to get tracks: {}", e))?;

    Ok(PaginatedTracks { tracks, total })
}

/// Position de la première piste de chaque lettre, pour la navigation A–Z de l'onglet Morceaux.
#[derive(Serialize)]
pub struct LetterOffset {
    pub letter: String,
    pub offset: i64,
}

#[tauri::command]
pub async fn get_track_letter_offsets(
    state: State<'_, AppState>,
    library_id: i64,
    sort_dir: Option<String>,
    filter: Option<String>,
    missing_cover: Option<bool>,
    quality: Option<String>,
    favorites: Option<bool>,
    genre: Option<String>,
) -> Result<Vec<LetterOffset>, String> {
    let extra = TrackFilters { quality: quality.as_deref(), favorites: favorites.unwrap_or(false), genre: genre.as_deref() };
    let lettres = LibraryTrackRepository::find_title_letter_offsets(
        &state.pool, library_id, sort_dir.as_deref() == Some("desc"), filter.as_deref(), missing_cover.unwrap_or(false), &extra,
    ).await.map_err(|e| format!("Failed to get letter offsets: {}", e))?;
    Ok(lettres.into_iter().map(|(letter, offset)| LetterOffset { letter, offset }).collect())
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
    LibraryAlbumRepository::find_albums_by_artist_id(&state.pool, library_id, &artist_id, true)
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

fn dossier_portraits() -> PathBuf {
    let mut dossier = dirs::data_dir().unwrap_or_default();
    dossier.push("com.larevuegeek.rustmusic");
    dossier.push("covers");
    dossier.push("artists");
    dossier
}

fn client_deezer(secondes: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(secondes))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))
}

const CHAMPS_PORTRAIT: &[&str] = &["picture_xl", "picture_big", "picture_medium"];

/// Ce qu'une passe de récupération rapporte à l'interface.
#[derive(Serialize)]
pub struct BilanRecuperation {
    pub found: u32,
    /// Pannes passagères (réseau, quota) : ces éléments seront recherchés à la prochaine passe.
    pub errors: u32,
    pub cancelled: bool,
}

/// Portrait Deezer d'un artiste. En base : chemin = trouvé, "" = introuvable, NULL = à chercher.
#[tauri::command]
pub async fn fetch_artist_image(
    state: State<'_, AppState>,
    artist_id: String,
    artist_name: String,
) -> Result<Option<String>, String> {
    match ArtistRepository::get_image_url(&state.pool, &artist_id)
        .await
        .map_err(|e| format!("DB error: {}", e))?
    {
        Some(url) if !url.is_empty() => return Ok(Some(url)),
        Some(_) => return Ok(None),
        None => {}
    }

    let client = client_deezer(3)?;
    let url = format!("https://api.deezer.com/search/artist?q={}&limit=1", urlencoding::encode(&artist_name));
    let image = match chercher(&client, &url, CHAMPS_PORTRAIT).await {
        Ok(Some(image)) => image,
        Ok(None) => {
            ArtistRepository::update_image_url(&state.pool, &artist_id, "")
                .await
                .map_err(|e| format!("Failed to update artist image: {}", e))?;
            return Ok(None);
        }
        Err(e) => {
            log::warn!("[portraits] '{}' : {}", artist_name, e);
            return Ok(None);
        }
    };

    // Téléchargement raté : rien d'enregistré, la prochaine visite réessaiera.
    let Some(octets) = telecharger(&client, &image).await else { return Ok(None) };
    let nom = format!("artist_{}.jpg", artist_id.replace('-', ""));
    let chemin = save_artist_image(&dossier_portraits(), &nom, &octets)?;
    ArtistRepository::update_image_url(&state.pool, &artist_id, &chemin)
        .await
        .map_err(|e| format!("Failed to update artist image: {}", e))?;
    Ok(Some(chemin))
}

/// Portraits manquants, en lot, avec progression. `force` : relance aussi les artistes
/// restés sans résultat — les portraits déjà trouvés ne sont pas retéléchargés.
#[tauri::command]
pub async fn fetch_all_artist_images(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    force: Option<bool>,
) -> Result<BilanRecuperation, String> {
    let Some(jeton) = PORTRAITS.demarrer() else { return Err("deja_en_cours".into()) };

    if force.unwrap_or(false) {
        ArtistRepository::reset_not_found_images(&state.pool)
            .await
            .map_err(|e| format!("DB error reset: {}", e))?;
    }

    let artists = ArtistRepository::find_without_image(&state.pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;
    let total = artists.len();
    if total == 0 {
        return Ok(BilanRecuperation { found: 0, errors: 0, cancelled: false });
    }

    let client = client_deezer(5)?;
    let dossier = dossier_portraits();
    let mut found: u32 = 0;
    let mut erreurs: u32 = 0;

    for (i, artist) in artists.iter().enumerate() {
        if jeton.annule() {
            break;
        }
        let _ = app.emit("artist-image-progress", serde_json::json!({ "current": i + 1, "total": total, "name": artist.name }));

        let url = format!("https://api.deezer.com/search/artist?q={}&limit=1", urlencoding::encode(&artist.name));
        match chercher(&client, &url, CHAMPS_PORTRAIT).await {
            Ok(None) => {
                let _ = ArtistRepository::update_image_url(&state.pool, &artist.id, "").await;
            }
            Ok(Some(image)) => {
                let nom = format!("artist_{}.jpg", artist.id.replace('-', ""));
                let enregistre = match telecharger(&client, &image).await {
                    Some(octets) => save_artist_image(&dossier, &nom, &octets).ok(),
                    None => None,
                };
                match enregistre {
                    Some(chemin) => {
                        found += 1;
                        let _ = ArtistRepository::update_image_url(&state.pool, &artist.id, &chemin).await;
                        let _ = app.emit("artist-image-ready", serde_json::json!({ "artist_id": artist.id, "image_url": chemin }));
                    }
                    None => erreurs += 1,
                }
            }
            Err(e) => {
                // Laissé à NULL : il sera recherché à la prochaine passe.
                erreurs += 1;
                log::warn!("[portraits] '{}' : {}", artist.name, e);
                if e.contains("Quota") {
                    jeton.patienter(5000).await;
                }
            }
        }

        // Une respiration entre deux artistes : Deezer limite le débit.
        jeton.patienter(500).await;
    }

    let _ = app.emit("artist-image-complete", serde_json::json!({
        "found": found, "total": total, "errors": erreurs, "cancelled": jeton.annule(),
    }));
    Ok(BilanRecuperation { found, errors: erreurs, cancelled: jeton.annule() })
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

    let saved_path = crate::helper::library::thumbnail_helper::thumbnail_saver(&covers_dir, &bytes.to_vec(), false)
        .map_err(|e| format!("Save error: {}", e))?;

    LibraryAlbumRepository::update_cover_url_by_id(&state.pool, &album_id, &saved_path)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    Ok(saved_path)
}

fn dossier_covers() -> PathBuf {
    let mut dossier = dirs::data_dir().unwrap_or_default();
    dossier.push("com.larevuegeek.rustmusic");
    dossier.push("covers");
    dossier
}

const CHAMPS_POCHETTE: &[&str] = &["cover_xl", "cover_big", "cover_medium"];

/// Pochette d'un album via Deezer.
#[tauri::command]
pub async fn fetch_album_cover(
    state: State<'_, AppState>,
    album_id: String,
    album_title: String,
    artist_name: Option<String>,
) -> Result<Option<String>, String> {
    let client = client_deezer(3)?;
    let requete = match artist_name {
        Some(ref artiste) => format!("{} {}", artiste, album_title),
        None => album_title.clone(),
    };
    let url = format!("https://api.deezer.com/search/album?q={}&limit=1", urlencoding::encode(&requete));
    let image = match chercher(&client, &url, CHAMPS_POCHETTE).await {
        Ok(Some(image)) => image,
        Ok(None) => return Ok(None),
        Err(e) => {
            log::warn!("[pochettes] '{}' : {}", album_title, e);
            return Ok(None);
        }
    };
    let Some(octets) = telecharger(&client, &image).await else { return Ok(None) };
    let chemin = crate::helper::library::thumbnail_helper::thumbnail_saver(&dossier_covers(), &octets, false)?;
    LibraryAlbumRepository::update_cover_url_by_id(&state.pool, &album_id, &chemin)
        .await
        .map_err(|e| format!("Failed to update album cover: {}", e))?;
    Ok(Some(chemin))
}

/// Pochettes manquantes d'une bibliothèque, en lot, avec progression ; annulable.
#[tauri::command]
pub async fn fetch_all_album_covers(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<BilanRecuperation, String> {
    let Some(jeton) = POCHETTES.demarrer() else { return Err("deja_en_cours".into()) };

    let albums = LibraryAlbumRepository::find_albums_without_cover(&state.pool, library_id)
        .await
        .map_err(|e| format!("DB error: {}", e))?;
    let total = albums.len();
    if total == 0 {
        return Ok(BilanRecuperation { found: 0, errors: 0, cancelled: false });
    }

    let client = client_deezer(5)?;
    let dossier = dossier_covers();
    let mut found: u32 = 0;
    let mut erreurs: u32 = 0;

    for (i, album) in albums.iter().enumerate() {
        if jeton.annule() {
            break;
        }
        let _ = app.emit("album-cover-progress", serde_json::json!({ "current": i + 1, "total": total, "name": album.title }));

        let artiste: Option<String> = ArtistRepository::find_name_by_id(&state.pool, &album.artist_id).await.ok().flatten();
        let requete = match artiste {
            Some(ref a) => format!("{} {}", a, album.title),
            None => album.title.clone(),
        };
        let url = format!("https://api.deezer.com/search/album?q={}&limit=1", urlencoding::encode(&requete));
        match chercher(&client, &url, CHAMPS_POCHETTE).await {
            Ok(None) => {}
            Ok(Some(image)) => {
                let enregistre = match telecharger(&client, &image).await {
                    Some(octets) => crate::helper::library::thumbnail_helper::thumbnail_saver(&dossier, &octets, false).ok(),
                    None => None,
                };
                match enregistre {
                    Some(chemin) => {
                        found += 1;
                        let _ = LibraryAlbumRepository::update_cover_url_by_id(&state.pool, &album.id, &chemin).await;
                        let _ = app.emit("album-cover-ready", serde_json::json!({ "album_id": album.id, "cover_url": chemin }));
                    }
                    None => erreurs += 1,
                }
            }
            Err(e) => {
                erreurs += 1;
                log::warn!("[pochettes] '{}' : {}", album.title, e);
                if e.contains("Quota") {
                    jeton.patienter(5000).await;
                }
            }
        }

        jeton.patienter(500).await;
    }

    let _ = app.emit("album-cover-complete", serde_json::json!({
        "found": found, "total": total, "errors": erreurs, "cancelled": jeton.annule(),
    }));
    Ok(BilanRecuperation { found, errors: erreurs, cancelled: jeton.annule() })
}

/// Arrête une récupération en cours (barre d'état) ; vrai si une passe tournait.
#[tauri::command]
pub fn cancel_task(task_id: String) -> bool {
    match task_id.as_str() {
        "album-covers" => POCHETTES.annuler(),
        "artist-images" => PORTRAITS.annuler(),
        "import" | "rescan" => IMPORT.annuler(),
        _ => false,
    }
}

#[derive(Serialize)]
pub struct BilanNettoyage {
    pub supprimes: u32,
    pub alleges: u32,
    pub liberes: u64,
}

/// Supprime les images que rien ne cite et allège les trop lourdes ; épargne celles de moins d'une heure (scan en cours).
#[tauri::command]
pub async fn clean_image_cache(state: State<'_, AppState>) -> Result<BilanNettoyage, String> {
    let mut cites: std::collections::HashSet<String> = std::collections::HashSet::new();
    for sql in [
        "SELECT cover_url FROM library_albums",
        "SELECT thumbnail_path FROM library_cache",
        "SELECT image_url FROM artists",
        "SELECT cover FROM queue_tracks",
        "SELECT cover FROM playlists",
    ] {
        let lignes: Vec<Option<String>> = sqlx::query_scalar(sql)
            .fetch_all(&state.pool)
            .await
            .map_err(|e| format!("DB error: {}", e))?;
        for chemin in lignes.into_iter().flatten() {
            // Par nom de fichier : hachage md5 ou id d'artiste, uniques — et insensible
            // à un dossier de données déplacé.
            if let Some(nom) = std::path::Path::new(&chemin.replace('\\', "/")).file_name() {
                cites.insert(nom.to_string_lossy().to_lowercase());
            }
        }
    }
    if cites.is_empty() {
        return Err("Aucune image référencée : nettoyage annulé par prudence".into());
    }

    let racine = dossier_covers();
    tokio::task::spawn_blocking(move || balayer_images(&racine, &cites))
        .await
        .map_err(|e| e.to_string())
}

fn balayer_images(racine: &std::path::Path, cites: &std::collections::HashSet<String>) -> BilanNettoyage {
    let mut bilan = BilanNettoyage { supprimes: 0, alleges: 0, liberes: 0 };
    let recent = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    for base in [racine.join("albums"), racine.join("albums").join("albums"), racine.join("artists")] {
        for taille in ["full", "1x", "2x"] {
            let Ok(entrees) = std::fs::read_dir(base.join(taille)) else { continue };
            for entree in entrees.flatten() {
                let chemin = entree.path();
                let Ok(meta) = entree.metadata() else { continue };
                if !meta.is_file() || meta.modified().map(|m| m > recent).unwrap_or(true) {
                    continue;
                }
                let nom = entree.file_name().to_string_lossy().to_lowercase();
                if !cites.contains(&nom) {
                    if std::fs::remove_file(&chemin).is_ok() {
                        bilan.supprimes += 1;
                        bilan.liberes += meta.len();
                    }
                } else if taille == "full" {
                    let Ok(octets) = std::fs::read(&chemin) else { continue };
                    if let Some(leger) = alleger(&octets) {
                        if std::fs::write(&chemin, &leger).is_ok() {
                            bilan.alleges += 1;
                            bilan.liberes += (octets.len() - leger.len()) as u64;
                        }
                    }
                }
            }
        }
    }
    bilan
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

/// Les mixes de l'accueil, tirés au hasard : `oublies`, `hires`, `hasard`, ou
/// `genre` avec le genre voulu.
#[tauri::command]
pub async fn get_mix_tracks(
    state: State<'_, AppState>,
    library_id: i64,
    kind: String,
    genre: Option<String>,
    decennie: Option<i64>,
    annee: Option<i64>,
    limit: i64,
) -> Result<Vec<TrackListView>, String> {
    LibraryTrackRepository::find_mix_tracks(&state.pool, library_id, &kind, genre.as_deref(), decennie, annee, limit.clamp(1, 200))
        .await
        .map_err(|e| format!("get_mix_tracks: {}", e))
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

/// Les titres d'une période (une décennie, ou une année quand `debut == fin`).
#[tauri::command]
pub async fn get_tracks_by_years(
    state: State<'_, AppState>,
    library_id: i64,
    debut: i64,
    fin: i64,
) -> Result<Vec<TrackListView>, String> {
    LibraryTrackRepository::find_tracks_by_years(&state.pool, library_id, debut, fin)
        .await
        .map_err(|e| format!("get_tracks_by_years: {}", e))
}

/// Un genre de la bibliothèque assez varié pour faire un mix.
#[derive(serde::Serialize)]
pub struct GenreMix {
    pub nom: String,
    pub titres: i64,
    pub artistes: i64,
}

/// Les genres qui font un vrai mix (assez d'artistes et de titres), les plus fournis d'abord.
#[tauri::command]
pub async fn get_mix_genres(state: State<'_, AppState>, library_id: i64) -> Result<Vec<GenreMix>, String> {
    use crate::core::variete::{MIN_ARTISTES, MIN_TITRES};
    let lignes = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT lc.genre, COUNT(*) n, COUNT(DISTINCT lt.artist_id) FROM library_tracks lt
         JOIN library_cache lc ON lc.id = lt.cache_id
         WHERE lt.library_id = ? AND lc.genre IS NOT NULL AND lc.genre != ''
         GROUP BY lc.genre HAVING n >= ? AND COUNT(DISTINCT lt.artist_id) >= ? ORDER BY n DESC",
    )
    .bind(library_id)
    .bind(MIN_TITRES)
    .bind(MIN_ARTISTES)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| format!("get_mix_genres: {e}"))?;
    Ok(lignes.into_iter().map(|(nom, titres, artistes)| GenreMix { nom, titres, artistes }).collect())
}

#[tauri::command]
pub async fn get_genres(
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<Vec<GenreView>, String> {
    let rows = LibraryGenreRepository::find_all_genres(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get genres: {}", e))?;

    // Pochettes et artistes phares en deux requêtes pour tous les genres, plutôt qu'une par genre.
    let mut covers: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (genre, cover) in LibraryGenreRepository::find_all_covers(&state.pool, library_id).await {
        covers.entry(genre).or_default().push(cover);
    }
    let mut artistes: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (genre, nom) in LibraryGenreRepository::find_top_artists(&state.pool, library_id).await {
        artistes.entry(genre).or_default().push(nom);
    }

    let mut genres = Vec::with_capacity(rows.len());
    for (name, total_albums, total_tracks, last_played_at) in rows {
        let covers = covers.remove(&name).unwrap_or_default();
        let top_artists = artistes.remove(&name).unwrap_or_default();
        genres.push(GenreView { name, total_albums, total_tracks, covers, top_artists, last_played_at });
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
