use std::path::PathBuf;
use crate::helper::files::audio_formats::is_audio_extension;
use serde::Serialize;
use crate::app::State;
use base64::engine::general_purpose;
use base64::Engine;

use crate::entity::audio::file_tags_info::FileTagsInfo;
use crate::entity::library::genre_view::GenreView;
use crate::entity::library::library_stats::*;
use crate::entity::library::dir_entry::DirEntry;
use crate::entity::library::library::{Library, LibraryCreate};
use crate::entity::library::library_cache::{LibraryCache, LibraryCacheCreate};
use crate::entity::library::library_track::TrackLocation;
use crate::helper::library::thumbnail_helper::{thumbnail_saver, save_artist_image, resolve_thumbnail};
use crate::helper::library::image_fetch::{download, search_first, shrink_image, COVER_FETCH, PORTRAIT_FETCH};
use crate::repository::library::composer_repository::ComposerRepository;
use crate::service::library::composer;
use crate::query::album::album_detail_row::AlbumDetailRow;
use crate::query::album::album_list_row::AlbumListRow;
use crate::query::artist::artist_detail_row::ArtistDetailRow;
use crate::query::artist::artist_list_row::ArtistListRow;
use crate::query::track::track_detail_row::TrackDetailRow;
use crate::repository::artist::artist_repository::ArtistRepository;
use crate::repository::library::library_album_repository::LibraryAlbumRepository;
use crate::repository::library::library_artist_repository::LibraryArtistRepository;
use crate::repository::library::library_cache_repository::LibraryCacheRepository;
use crate::repository::library::library_dirs_repository::LibraryDirRepository;
use crate::repository::library::library_files_repository::LibraryFilesRepository;
use crate::repository::library::image_repository::ImageRepository;
use crate::repository::library::library_genre_repository::LibraryGenreRepository;
use crate::repository::library::library_repository::LibraryRepository;
use crate::repository::library::library_stats_repository::LibraryStatsRepository;
use crate::repository::library::library_track_repository::{LibraryTrackRepository, TrackFilters};
use crate::query::artist::track_artist_row::TrackArtistRow;
use crate::repository::library::library_track_artist_repository::LibraryTrackArtistRepository;
use crate::service::library::artist_link_repair::ArtistLinkReport;
use crate::service::library::library_service::{IMPORT, Listing, LibrarySaveContext, RescanProgress, create_context, list_files, save_dir_to_library, save_files_to_library, save_track_to_library};
use crate::{app::platform::app_state::AppState};
use crate::service::external_server::feedback_service::{self, Feedback};
use crate::query::track::track_list_row::TrackListRow;

pub async fn add_files(
    app: crate::app::AppHandle,
    state: State<'_, AppState>, 
    library_id: i64,
    files: Vec<String>
) -> Result<Vec<TrackListRow>, String> {
    
    let mut tracks: Vec<TrackListRow> = Vec::new();

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

pub async fn add_directory(
    app: crate::app::AppHandle,
    state: State<'_, AppState>, 
    library_id: i64,
    directory: String
) -> Result<Vec<TrackListRow>, String> {
    
    let Some(token) = IMPORT.start() else { return Err("deja_en_cours".into()) };
    let tracks: Vec<TrackListRow> = save_dir_to_library(app, &state.pool, library_id, directory, &token).await?;

    Ok(tracks)
}

/// Rejoue les liaisons piste ↔ artiste. Voir `artist_link_repair`.
/// Les artistes crédités sur une piste. Vide tant que les liaisons ne sont pas
/// posées — la fiche retombe alors sur le tag brut.
pub async fn get_track_artists(
    state: State<'_, AppState>,
    track_id: String,
) -> Result<Vec<TrackArtistRow>, String> {
    LibraryTrackArtistRepository::find_artists_of_track(&state.pool, &track_id)
        .await
        .map_err(|e| format!("Failed to get track artists: {}", e))
}


pub async fn get_track_locations(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<TrackLocation>, String> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }

    LibraryTrackRepository::find_locations(&state.pool, &paths)
        .await
        .map_err(|e| format!("Failed to locate tracks: {e}"))
}

pub async fn repair_artist_links(
    state: State<'_, AppState>,
    library_id: Option<i64>,
) -> Result<ArtistLinkReport, String> {
    crate::service::library::artist_link_repair::repair(&state.pool, library_id).await
}

/// Dossier de bibliothèque introuvable au scan, et le montage où il a été retrouvé.
#[derive(Clone, serde::Serialize)]
pub struct UnavailableDir {
    pub dir_id: String,
    pub path: String,
    pub remount: Option<crate::core::system::remount::Remount>,
}

/// Un dossier absent (partage réseau non monté, ou monté sous un autre nom) n'est
/// pas scanné : le lister donnait « 0 fichier » et remettait ses totaux à zéro.
async fn check_dir_available(
    pool: &sqlx::SqlitePool,
    dir: &crate::entity::library::library_dirs::LibraryDir,
) -> Option<UnavailableDir> {
    if std::path::Path::new(&dir.path).is_dir() {
        return None;
    }
    log::warn!("📁 Dossier inaccessible, non scanné : {}", dir.path);
    let _ = LibraryDirRepository::mark_unavailable(pool, &dir.id, "unavailable").await;
    Some(UnavailableDir {
        dir_id: dir.id.clone(),
        path: dir.path.clone(),
        remount: crate::core::system::remount::find_remount(std::path::Path::new(&dir.path)),
    })
}

fn emit_unavailable(app: &crate::app::AppHandle, library_id: i64, dirs: Vec<UnavailableDir>) {
    if !dirs.is_empty() {
        let _ = app.emit("library-dirs-unavailable", serde_json::json!({
            "library_id": library_id,
            "dirs": dirs,
        }));
    }
}

/// Réécrit les chemins de `old_root` vers `new_root` (le même partage, monté sous un
/// autre nom). Rend le nombre de fichiers déplacés ; un rescan doit suivre.
pub async fn relocate_library_root(
    state: State<'_, AppState>,
    old_root: String,
    new_root: String,
) -> Result<u64, String> {
    if !std::path::Path::new(&new_root).is_dir() {
        return Err("nouveau_dossier_introuvable".into());
    }
    // Pas pendant un import : il écrirait des chemins que la réécriture ne verrait pas.
    let Some(_token) = IMPORT.start() else { return Err("deja_en_cours".into()) };
    let n = crate::service::library::relocate::relocate_prefix(&state.pool, &old_root, &new_root).await?;
    log::info!("📁 Bibliothèque déplacée : {old_root} → {new_root} ({n} fichiers)");
    Ok(n)
}

pub async fn rescan_library(
    app: crate::app::AppHandle,
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<Vec<TrackListRow>, String> {
    // Bibliothèque distante : on synchronise avec son serveur.
    if LibraryRepository::find_library_by_id(&state.pool, library_id).await.is_ok_and(|l| l.server_id.is_some()) {
        let Some(token) = IMPORT.start() else { return Err("deja_en_cours".into()) };
        crate::service::external_server::sync_service::sync_library(app, &state.pool, library_id, &token).await?;
        return Ok(Vec::new());
    }

    // Récupérer tous les dossiers actifs de la bibliothèque
    let dirs = LibraryDirRepository::find_active(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get library dirs: {}", e))?;

    if dirs.is_empty() {
        return Err("Aucun dossier enregistré dans cette bibliothèque".to_string());
    }

    let Some(token) = IMPORT.start() else { return Err("deja_en_cours".into()) };

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
    let mut inaccessible: Vec<UnavailableDir> = Vec::new();
    for dir in dirs {
        if let Some(absent) = check_dir_available(&state.pool, &dir).await {
            inaccessible.push(absent);
            continue;
        }
        let Some(listing) = list_files(&app, library_id, &dir.path).await else { break };
        lots.push((dir, listing));
    }

    // Avant le scan, sur toute la bibliothèque (un fichier peut changer de dossier) : renommé ou
    // déplacé, il garde sa piste au lieu d'en recréer une.
    if !token.is_cancelled() && !lots.is_empty() {
        let dirs: Vec<String> = lots.iter().map(|(dir, _)| dir.path.clone()).collect();
        let listed: Vec<PathBuf> = lots.iter().flat_map(|(_, l)| l.files.iter().cloned()).collect();
        let unreadable: Vec<PathBuf> = lots.iter().flat_map(|(_, l)| l.unreadable.iter().cloned()).collect();
        if let Err(e) = crate::service::library::missing_files::relink_renamed(&state.pool, library_id, &dirs, &listed, &unreadable).await {
            log::warn!("📁 Renommages non suivis : {e}");
        }
    }

    let total: usize = lots.iter().map(|(_, listing)| listing.files.len()).sum();
    let mut progression = RescanProgress::new(library_id, total);
    progression.emit_progress(&app);

    let mut all_tracks: Vec<TrackListRow> = Vec::new();

    // Re-scanner chaque dossier (les fichiers inchangés sont sautés via le cache)
    for (dir, Listing { files, unreadable }) in lots {
        if token.is_cancelled() {
            break;
        }
        let listed = files.clone();
        match save_files_to_library(app.clone(), &state.pool, library_id, dir.path.clone(), files, Some(&mut progression), &token).await {
            Ok(tracks) => all_tracks.extend(tracks),
            Err(e) => {
                log::error!("Erreur rescan dossier {}: {}", dir.name, e);
                continue;
            }
        }
        // Listage complet et scan mené au bout : ce qui manque au disque quitte la bibliothèque.
        if !token.is_cancelled() {
            if let Err(e) = crate::service::library::missing_files::remove_missing_files(&state.pool, library_id, &dir.path, &listed, &unreadable).await {
                log::warn!("📁 Fichiers disparus de {} non retirés : {e}", dir.path);
            }
        }
    }

    // Après le scan : la relocalisation proposée doit pouvoir relancer le sien.
    emit_unavailable(&app, library_id, inaccessible);

    let _ = app.emit("rescan-complete", serde_json::json!({
        "library_id": library_id,
        "library_name": library_name,
        "cancelled": token.is_cancelled(),
    }));

    Ok(all_tracks)
}

/// Date du dernier scan, rendue comme serde rend nos `DateTime<Utc>` : `2026-09-30T12:34:56Z`.
pub async fn get_library_last_scan(
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<Option<String>, String> {
    let last_scan = LibraryDirRepository::find_last_scan_at(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get last scan: {}", e))?;

    Ok(last_scan.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)))
}

pub async fn get_library_dirs(
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<Vec<crate::entity::library::library_dirs::LibraryDir>, String> {
    LibraryDirRepository::find_all_by_library_id(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get library dirs: {}", e))
}

pub async fn get_tracks_by_dir(
    state: State<'_, AppState>,
    library_id: i64,
    dir_id: String,
) -> Result<Vec<TrackListRow>, String> {
    LibraryTrackRepository::find_tracks_by_dir_id(&state.pool, library_id, &dir_id)
        .await
        .map_err(|e| format!("Failed to get tracks by dir: {}", e))
}

/// Pistes sous un sous-dossier quelconque (sélection d'un dossier dans la vue Dossiers).
pub async fn get_tracks_under_path(
    state: State<'_, AppState>,
    library_id: i64,
    path: String,
) -> Result<Vec<TrackListRow>, String> {
    LibraryTrackRepository::find_tracks_under_path(&state.pool, library_id, &path)
        .await
        .map_err(|e| format!("Failed to get tracks under path: {}", e))
}

// ============================================================================
// EXPLORATEUR DE FICHIERS (restreint aux dossiers importés)
// ============================================================================


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
    let mut depth = 0usize;
    for d in &dirs {
        let Ok(canonical_dir) = std::fs::canonicalize(&d.path) else { continue };
        if let Ok(rest) = canonical_path.strip_prefix(&canonical_dir) {
            let n = canonical_dir.as_os_str().len();
            if base.is_none() || n > depth {
                depth = n;
                base = Some(if rest.as_os_str().is_empty() { PathBuf::from(&d.path) } else { PathBuf::from(&d.path).join(rest) });
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
                preview: None,
            });
        } else if file_type.is_file() {
            let ext = entry_path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());

            // Ne montrer que les fichiers audio
            if let Some(ref ext_str) = ext {
                if is_audio_extension(ext_str) {
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
                        preview: None,
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
    let paths: Vec<String> = entries.iter().filter(|e| !e.is_dir).map(|e| e.path.clone()).collect();
    let mut records = LibraryTrackRepository::find_entries_by_paths(&state.pool, Some(library_id), &paths).await;
    for e in entries.iter_mut().filter(|e| !e.is_dir) {
        if let Some(f) = records.remove(&e.path) {
            e.title = f.title;
            e.artist = f.artist;
            e.duration = f.duration;
            e.thumbnail_path = f.thumbnail_path;
            e.track_id = f.track_id;
        }
    }

    // Sous-dossiers : pochettes et qualité de leur contenu, en une requête.
    if entries.iter().any(|e| e.is_dir) {
        let mut previews = crate::service::library::folder_preview::subfolder_previews(&state.pool, library_id, &base.to_string_lossy()).await;
        for e in entries.iter_mut().filter(|e| e.is_dir) {
            e.preview = previews.remove(&e.name);
        }
    }

    Ok(entries)
}

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

pub async fn remove_library_dir(
    state: State<'_, AppState>,
    dir_id: String,
) -> Result<(), String> {
    LibraryDirRepository::delete_library_dir(&state.pool, &dir_id)
        .await
        .map_err(|e| format!("Failed to remove library dir: {}", e))
}

pub async fn rescan_library_dir(
    app: crate::app::AppHandle,
    state: State<'_, AppState>,
    library_id: i64,
    dir_id: String,
) -> Result<Vec<TrackListRow>, String> {
    let dirs = LibraryDirRepository::find_all_by_library_id(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get dirs: {}", e))?;

    let dir = dirs.into_iter().find(|d| d.id == dir_id)
        .ok_or_else(|| "Directory not found".to_string())?;

    if let Some(absent) = check_dir_available(&state.pool, &dir).await {
        emit_unavailable(&app, library_id, vec![absent]);
        return Ok(Vec::new());
    }

    let Some(token) = IMPORT.start() else { return Err("deja_en_cours".into()) };
    save_dir_to_library(app, &state.pool, library_id, dir.path, &token).await
}

pub fn save_thumbnail(
    app: crate::app::AppHandle,
    image_data: Vec<u8>,
) -> Result<String, String> {
    
    // 1. Résoudre le chemin AppData/covers
    let covers_dir: PathBuf = crate::app::platform::paths::covers_dir();

    thumbnail_saver(&covers_dir, &image_data, false)
}

/// Ecrit la vignette de la pochette embarquee d'un fichier.
///
/// Remplace l'aller-retour par `save_thumbnail`, ou les octets faisaient
/// Rust -> JSON -> JS -> JSON -> Rust pour finir dans un fichier.
pub fn save_thumbnail_from_file(
    app: crate::app::AppHandle,
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

    let covers_dir: PathBuf = crate::app::platform::paths::covers_dir();

    thumbnail_saver(&covers_dir, &image.image_data, false).map(Some)
}

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
pub fn resolve_cover_thumbnail(path: String) -> Result<String, String> {
    resolve_thumbnail(&path)
        .ok_or_else(|| format!("Impossible de générer la miniature pour: {}", path))
}

/// Pochette pré-floutée pour les fonds, générée hors du fil principal.
pub async fn resolve_blurred_cover(path: String, saturation: f32, blur: f32) -> Result<String, String> {
    crate::app::platform::runtime::spawn_blocking(move || {
        crate::helper::library::thumbnail_helper::blurred_cover(&path, saturation, blur)
    })
    .await
    .map_err(|e| format!("pochette floue : {e}"))?
    .map(|path_part| path_part.to_string_lossy().into_owned())
}

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
    let _ = LibraryRepository::ensure_default(&state.pool, library.profile_id).await;

    // Relecture : la promotion vient peut-être de changer `is_default`, et
    // l'interface s'appuie dessus pour afficher le repère.
    Ok(LibraryRepository::find_library_by_id(&state.pool, library.id)
        .await
        .unwrap_or(library))
}

/// Rend les chemins des morceaux distants retirés, pour que l'interface les sorte de la file.
pub async fn remove_library(
    state: State<'_, AppState>,
    library_id: i64
) -> Result<Vec<String>, String> {

    // Le profil est lu AVANT la suppression : après, la ligne n'existe plus et
    // on ne saurait plus à qui redonner un défaut.
    let profile_id = LibraryRepository::find_library_by_id(&state.pool, library_id)
        .await
        .map(|library| library.profile_id)
        .ok();

    // Distante : ses chemins `remote://`, à effacer aussi du cache, de la file et des titres aimés.
    let remote_paths = LibraryFilesRepository::find_remote_paths_by_library(&state.pool, library_id).await.unwrap_or_default();

    LibraryRepository::remove_library(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to remove library: {}", e))?;

    if !remote_paths.is_empty() {
        let mut conn = state.pool.acquire().await.map_err(|e| e.to_string())?;
        let _ = crate::repository::server::server_repository::ServerRepository::purge_paths(&mut conn, &remote_paths).await;
    }

    // Supprimer la bibliothèque par défaut laisserait le profil sans point
    // d'entrée au démarrage : on en promeut une autre.
    if let Some(profile_id) = profile_id {
        let _ = LibraryRepository::ensure_default(&state.pool, profile_id).await;
    }

    Ok(remote_paths)
}

/// Désigne la bibliothèque ouverte au démarrage, pour le profil concerné.
pub async fn set_default_library(
    state: State<'_, AppState>,
    library_id: i64
) -> Result<(), String> {

    LibraryRepository::set_default(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to set default library: {}", e))
}

/// Renomme une bibliothèque ; un nom déjà pris dans le profil est refusé (UNIQUE).
pub async fn rename_library(
    state: State<'_, AppState>,
    library_id: i64,
    name: String,
) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Nom vide".to_string());
    }
    LibraryRepository::rename(&state.pool, library_id, name)
        .await
        .map_err(|e| format!("Failed to rename library: {}", e))
}

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

pub async fn get_libraries(
    state: State<'_, AppState>, 
    profile_id: i64
) -> Result<Vec<Library>, String> {

    let libraries: Vec<Library> = match LibraryRepository::find_libraries_by_profile_id(&state.pool, profile_id).await {
        Ok(libraries) => libraries,
        Err(e) => return Err(format!("Failed to get libraries : {}", e))
    };

    Ok(libraries)
}

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

pub async fn get_track(
    state: State<'_, AppState>,
    library_track_id: String
) -> Result<TrackDetailRow, String> {

    let new_track: TrackDetailRow = match LibraryTrackRepository::find_track_by_id(&state.pool, library_track_id).await {
        Ok(track) => track,
        Err(e) => return Err(format!("Failed to get track : {}", e))
    };

    Ok(new_track)
}

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
        let steps = (r * 2.0).round();
        if (1.0..=10.0).contains(&steps) {
            Some(steps / 2.0)
        } else {
            None
        }
    });
    LibraryTrackRepository::update_rating(&state.pool, &track_id, normalized)
        .await
        .map_err(|e| format!("Failed to update rating: {}", e))?;
    // Morceau d'un serveur : la note lui est renvoyée (entière, de 0 à 5).
    let path = LibraryTrackRepository::find_path_by_id(&state.pool, &track_id).await.ok().flatten();
    if let Some(path) = path {
        crate::commands::playlist::playlist_command::send_feedback(&state, path, Feedback::Rating(feedback_service::server_rating(normalized)));
    }
    Ok(())
}


pub async fn get_tracks(
    state: State<'_, AppState>,
    library_id: i64
) -> Result<Vec<TrackListRow>, String> {

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
pub async fn mark_track_played(
    state: State<'_, AppState>,
    path: String,
) -> Result<Option<crate::repository::library::library_track_repository::PlayStats>, String> {
    let touched = LibraryTrackRepository::mark_played(&state.pool, &path)
        .await
        .map_err(|e| format!("Compteur d'écoutes : {e}"))?;

    // Tracé en information, et non en débogage.
    //
    // Le compteur ne se voit qu'au bout d'une minute d'écoute, et seulement
    // dans une playlist qui s'en sert : quand il ne marche pas, rien ne le dit.
    // Cette ligne est le seul moyen de distinguer « jamais déclenché » de
    // « déclenché sans rien trouver » — deux pannes qui n'ont pas le même
    // remède.
    if touched == 0 {
        log::info!("♪ Écoute non comptée, chemin absent de la bibliothèque : {path}");
    } else {
        log::info!("♪ Écoute comptée : {path}");
    }
    // Morceau d'un serveur : l'écoute lui est aussi comptée (scrobble).
    let now = chrono::Utc::now().timestamp_millis();
    crate::commands::playlist::playlist_command::send_feedback(&state, path.clone(), Feedback::Scrobble(now));

    // Les nouvelles valeurs, pour que les listes ouvertes se mettent à jour sans rechargement.
    Ok(LibraryTrackRepository::find_play_stats(&state.pool, &path).await.ok().flatten())
}

#[derive(Serialize)]
pub struct PaginatedTracks {
    pub tracks: Vec<TrackListRow>,
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
    if let Some(name) = sort_by.and_then(|s| s.strip_prefix("tag:custom:")) {
        return (
            "(SELECT json_extract(je.value, '$[1]') \
              FROM json_each(json_extract(lt.tags, '$.custom_tags')) je \
              WHERE json_extract(je.value, '$[0]') = ? LIMIT 1) COLLATE NOCASE"
                .to_string(),
            Some(name.to_string()),
        );
    }

    // Champ nommé de la structure de tags.
    if let Some(field) = sort_by.and_then(|s| s.strip_prefix("tag:")) {
        return (
            "json_extract(lt.tags, ?) COLLATE NOCASE".to_string(),
            Some(format!("$.{field}")),
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
    let letters = LibraryTrackRepository::find_title_letter_offsets(
        &state.pool, library_id, sort_dir.as_deref() == Some("desc"), filter.as_deref(), missing_cover.unwrap_or(false), &extra,
    ).await.map_err(|e| format!("Failed to get letter offsets: {}", e))?;
    Ok(letters.into_iter().map(|(letter, offset)| LetterOffset { letter, offset }).collect())
}

pub async fn get_tracks_by_album(
    state: State<'_, AppState>,
    library_id: i64,
    library_album_id: String
) -> Result<Vec<TrackListRow>, String> {
    LibraryTrackRepository::find_all_tracks_album_by_library_id(&state.pool, library_id, library_album_id)
        .await
        .map_err(|e| format!("Failed to get album tracks: {}", e))
}

pub async fn get_album(
    state: State<'_, AppState>,
    library_album_id: String
) -> Result<AlbumDetailRow, String> {
    LibraryAlbumRepository::find_album_by_id(&state.pool, library_album_id)
        .await
        .map_err(|e| format!("Failed to get album: {}", e))
}

pub async fn get_albums(
    state: State<'_, AppState>,
    library_id: i64,
    missing_cover: Option<bool>
) -> Result<Vec<AlbumListRow>, String> {
    LibraryAlbumRepository::find_all_albums_by_library_id(&state.pool, library_id, missing_cover)
        .await
        .map_err(|e| format!("Failed to get albums: {}", e))
}

pub async fn get_artist(
    state: State<'_, AppState>,
    library_artist_id: String
) -> Result<ArtistDetailRow, String> {
    LibraryArtistRepository::find_artist_by_id(&state.pool, library_artist_id)
        .await
        .map_err(|e| format!("Failed to get artist: {}", e))
}

pub async fn get_tracks_by_artist(
    state: State<'_, AppState>,
    library_id: i64,
    artist_id: String,
) -> Result<Vec<TrackListRow>, String> {
    LibraryTrackRepository::find_tracks_view_by_artist_id(&state.pool, library_id, &artist_id)
        .await
        .map_err(|e| format!("Failed to get artist tracks: {}", e))
}

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

pub async fn get_albums_by_artist(
    state: State<'_, AppState>,
    library_id: i64,
    artist_id: String,
) -> Result<Vec<AlbumListRow>, String> {
    LibraryAlbumRepository::find_albums_by_artist_id(&state.pool, library_id, &artist_id, true)
        .await
        .map_err(|e| format!("Failed to get artist albums: {}", e))
}

pub async fn get_artists(
    state: State<'_, AppState>,
    library_id: i64
) -> Result<Vec<ArtistListRow>, String> {
    LibraryArtistRepository::find_all_artists_by_library_id(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get artists: {}", e))
}

pub async fn get_similar_artists(
    state: State<'_, AppState>,
    library_id: i64,
    artist_id: String,
    limit: Option<i64>,
) -> Result<Vec<ArtistListRow>, String> {
    let limit = limit.unwrap_or(10);
    LibraryArtistRepository::find_similar_artists(&state.pool, library_id, &artist_id, limit)
        .await
        .map_err(|e| format!("Failed to get similar artists: {}", e))
}

pub(crate) fn portraits_dir() -> PathBuf {
    let mut folder = crate::app::platform::paths::data_root().unwrap_or_default();
    folder.push("com.larevuegeek.rustmusic");
    folder.push("covers");
    folder.push("artists");
    folder
}

pub(crate) fn client_deezer(seconds: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(seconds))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))
}

const PORTRAIT_FIELDS: &[&str] = &["picture_xl", "picture_big", "picture_medium"];

/// Portrait Deezer d'un nom : `Ok(None)` introuvable, `Err` passager (réseau, quota, téléchargement).
pub(crate) async fn deezer_portrait(client: &reqwest::Client, name: &str) -> Result<Option<Vec<u8>>, String> {
    let url = format!("https://api.deezer.com/search/artist?q={}&limit=1", urlencoding::encode(name));
    let Some(image) = search_first(client, &url, PORTRAIT_FIELDS).await? else { return Ok(None) };
    download(client, &image).await.map(Some).ok_or_else(|| "téléchargement".to_string())
}

/// Ce qu'une passe de récupération rapporte à l'interface.
#[derive(Serialize)]
pub struct RecoverySummary {
    pub found: u32,
    /// Pannes passagères (réseau, quota) : ces éléments seront recherchés à la prochaine passe.
    pub errors: u32,
    pub cancelled: bool,
}

/// Portrait Deezer d'un artiste. En base : chemin = trouvé, "" = introuvable, NULL = à chercher.
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
    let image = match search_first(&client, &url, PORTRAIT_FIELDS).await {
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
    let Some(bytes) = download(&client, &image).await else { return Ok(None) };
    let name = format!("artist_{}.jpg", artist_id.replace('-', ""));
    let path_part = save_artist_image(&portraits_dir(), &name, &bytes)?;
    ArtistRepository::update_image_url(&state.pool, &artist_id, &path_part)
        .await
        .map_err(|e| format!("Failed to update artist image: {}", e))?;
    Ok(Some(path_part))
}

/// Portraits manquants, en lot, avec progression. `force` : relance aussi les artistes
/// restés sans résultat — les portraits déjà trouvés ne sont pas retéléchargés.
pub async fn fetch_all_artist_images(
    app: crate::app::AppHandle,
    state: State<'_, AppState>,
    force: Option<bool>,
) -> Result<RecoverySummary, String> {
    let Some(token) = PORTRAIT_FETCH.start() else { return Err("deja_en_cours".into()) };

    if force.unwrap_or(false) {
        ArtistRepository::reset_not_found_images(&state.pool)
            .await
            .map_err(|e| format!("DB error reset: {}", e))?;
        ComposerRepository::reset_not_found(&state.pool)
            .await
            .map_err(|e| format!("DB error reset: {}", e))?;
    }

    let artists = ArtistRepository::find_without_image(&state.pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;
    let composers = composer::without_photo(&state.pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;
    let total = artists.len() + composers.len();
    if total == 0 {
        return Ok(RecoverySummary { found: 0, errors: 0, cancelled: false });
    }

    let client = client_deezer(5)?;
    let folder = portraits_dir();
    let mut found: u32 = 0;
    let mut errors: u32 = 0;

    for (i, artist) in artists.iter().enumerate() {
        if token.is_cancelled() {
            break;
        }
        let _ = app.emit("artist-image-progress", serde_json::json!({ "current": i + 1, "total": total, "name": artist.name }));

        let url = format!("https://api.deezer.com/search/artist?q={}&limit=1", urlencoding::encode(&artist.name));
        match search_first(&client, &url, PORTRAIT_FIELDS).await {
            Ok(None) => {
                let _ = ArtistRepository::update_image_url(&state.pool, &artist.id, "").await;
            }
            Ok(Some(image)) => {
                let name = format!("artist_{}.jpg", artist.id.replace('-', ""));
                let saved = match download(&client, &image).await {
                    Some(bytes) => save_artist_image(&folder, &name, &bytes).ok(),
                    None => None,
                };
                match saved {
                    Some(path_part) => {
                        found += 1;
                        let _ = ArtistRepository::update_image_url(&state.pool, &artist.id, &path_part).await;
                        let _ = app.emit("artist-image-ready", serde_json::json!({ "artist_id": artist.id, "image_url": path_part }));
                    }
                    None => errors += 1,
                }
            }
            Err(e) => {
                // Laissé à NULL : il sera recherché à la prochaine passe.
                errors += 1;
                log::warn!("[portraits] '{}' : {}", artist.name, e);
                if e.contains("Quota") {
                    token.sleep(5000).await;
                }
            }
        }

        // Une respiration entre deux artistes : Deezer limite le débit.
        token.sleep(500).await;
    }

    for (j, name) in composers.iter().enumerate() {
        if token.is_cancelled() {
            break;
        }
        let _ = app.emit("artist-image-progress", serde_json::json!({ "current": artists.len() + j + 1, "total": total, "name": name }));
        match deezer_portrait(&client, name).await {
            Ok(None) => {
                let _ = composer::save_photo(&state.pool, name, "").await;
            }
            Ok(Some(bytes)) => match save_artist_image(&folder, &composer::photo_file(name), &bytes) {
                Ok(path_part) => {
                    found += 1;
                    let _ = composer::save_photo(&state.pool, name, &path_part).await;
                    let _ = app.emit("composer-image-ready", serde_json::json!({ "name": name, "image_url": path_part }));
                }
                Err(_) => errors += 1,
            },
            Err(e) => {
                errors += 1;
                log::warn!("[portraits] compositeur '{}' : {}", name, e);
                if e.contains("Quota") {
                    token.sleep(5000).await;
                }
            }
        }
        token.sleep(500).await;
    }

    let _ = app.emit("artist-image-complete", serde_json::json!({
        "found": found, "total": total, "errors": errors, "cancelled": token.is_cancelled(),
    }));
    Ok(RecoverySummary { found, errors, cancelled: token.is_cancelled() })
}

// ============================================================================
// CUSTOM COVER
// ============================================================================

/// Set a custom cover for an album from a local image file
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

    let mut covers_dir = crate::app::platform::paths::data_root().unwrap_or_default();
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

    let mut covers_dir = crate::app::platform::paths::data_root().unwrap_or_default();
    covers_dir.push("com.larevuegeek.rustmusic");
    covers_dir.push("covers");

    let saved_path = crate::helper::library::thumbnail_helper::thumbnail_saver(&covers_dir, &bytes.to_vec(), false)
        .map_err(|e| format!("Save error: {}", e))?;

    LibraryAlbumRepository::update_cover_url_by_id(&state.pool, &album_id, &saved_path)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    Ok(saved_path)
}

fn covers_root() -> PathBuf {
    let mut folder = crate::app::platform::paths::data_root().unwrap_or_default();
    folder.push("com.larevuegeek.rustmusic");
    folder.push("covers");
    folder
}

const COVER_FIELDS: &[&str] = &["cover_xl", "cover_big", "cover_medium"];

/// Pochette d'un album via Deezer.
pub async fn fetch_album_cover(
    state: State<'_, AppState>,
    album_id: String,
    album_title: String,
    artist_name: Option<String>,
) -> Result<Option<String>, String> {
    let client = client_deezer(3)?;
    let query = match artist_name {
        Some(ref artist_name) => format!("{} {}", artist_name, album_title),
        None => album_title.clone(),
    };
    let url = format!("https://api.deezer.com/search/album?q={}&limit=1", urlencoding::encode(&query));
    let image = match search_first(&client, &url, COVER_FIELDS).await {
        Ok(Some(image)) => image,
        Ok(None) => return Ok(None),
        Err(e) => {
            log::warn!("[pochettes] '{}' : {}", album_title, e);
            return Ok(None);
        }
    };
    let Some(bytes) = download(&client, &image).await else { return Ok(None) };
    let path_part = crate::helper::library::thumbnail_helper::thumbnail_saver(&covers_root(), &bytes, false)?;
    LibraryAlbumRepository::update_cover_url_by_id(&state.pool, &album_id, &path_part)
        .await
        .map_err(|e| format!("Failed to update album cover: {}", e))?;
    Ok(Some(path_part))
}

/// Pochettes manquantes d'une bibliothèque, en lot, avec progression ; annulable.
pub async fn fetch_all_album_covers(
    app: crate::app::AppHandle,
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<RecoverySummary, String> {
    let Some(token) = COVER_FETCH.start() else { return Err("deja_en_cours".into()) };

    let albums = LibraryAlbumRepository::find_albums_without_cover(&state.pool, library_id)
        .await
        .map_err(|e| format!("DB error: {}", e))?;
    let total = albums.len();
    if total == 0 {
        return Ok(RecoverySummary { found: 0, errors: 0, cancelled: false });
    }

    let client = client_deezer(5)?;
    let folder = covers_root();
    let mut found: u32 = 0;
    let mut errors: u32 = 0;

    for (i, album) in albums.iter().enumerate() {
        if token.is_cancelled() {
            break;
        }
        let _ = app.emit("album-cover-progress", serde_json::json!({ "current": i + 1, "total": total, "name": album.title }));

        let artist_name: Option<String> = ArtistRepository::find_name_by_id(&state.pool, &album.artist_id).await.ok().flatten();
        let query = match artist_name {
            Some(ref a) => format!("{} {}", a, album.title),
            None => album.title.clone(),
        };
        let url = format!("https://api.deezer.com/search/album?q={}&limit=1", urlencoding::encode(&query));
        match search_first(&client, &url, COVER_FIELDS).await {
            Ok(None) => {}
            Ok(Some(image)) => {
                let saved = match download(&client, &image).await {
                    Some(bytes) => crate::helper::library::thumbnail_helper::thumbnail_saver(&folder, &bytes, false).ok(),
                    None => None,
                };
                match saved {
                    Some(path_part) => {
                        found += 1;
                        let _ = LibraryAlbumRepository::update_cover_url_by_id(&state.pool, &album.id, &path_part).await;
                        let _ = app.emit("album-cover-ready", serde_json::json!({ "album_id": album.id, "cover_url": path_part }));
                    }
                    None => errors += 1,
                }
            }
            Err(e) => {
                errors += 1;
                log::warn!("[pochettes] '{}' : {}", album.title, e);
                if e.contains("Quota") {
                    token.sleep(5000).await;
                }
            }
        }

        token.sleep(500).await;
    }

    let _ = app.emit("album-cover-complete", serde_json::json!({
        "found": found, "total": total, "errors": errors, "cancelled": token.is_cancelled(),
    }));
    Ok(RecoverySummary { found, errors, cancelled: token.is_cancelled() })
}

/// Arrête une récupération en cours (barre d'état) ; vrai si une passe tournait.
pub fn cancel_task(task_id: String) -> bool {
    match task_id.as_str() {
        "album-covers" => COVER_FETCH.cancel(),
        "artist-images" => PORTRAIT_FETCH.cancel(),
        "import" | "rescan" => IMPORT.cancel(),
        _ => false,
    }
}

#[derive(Serialize)]
pub struct CleanupSummary {
    #[serde(rename = "supprimes")]
    pub deleted: u32,
    pub alleges: u32,
    #[serde(rename = "liberes")]
    pub freed: u64,
}

/// Supprime les images que rien ne cite et allège les trop lourdes ; épargne celles de moins d'une heure (scan en cours).
pub async fn clean_image_cache(state: State<'_, AppState>) -> Result<CleanupSummary, String> {
    let mut cites: std::collections::HashSet<String> = std::collections::HashSet::new();
    let references = ImageRepository::referenced_paths(&state.pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;
    for path_part in references {
        // Par nom de fichier : hachage md5 ou id d'artiste, uniques — et insensible
        // à un dossier de données déplacé.
        if let Some(name) = std::path::Path::new(&path_part.replace('\\', "/")).file_name() {
            cites.insert(name.to_string_lossy().to_lowercase());
        }
    }
    if cites.is_empty() {
        return Err("Aucune image référencée : nettoyage annulé par prudence".into());
    }

    let root = covers_root();
    tokio::task::spawn_blocking(move || sweep_images(&root, &cites))
        .await
        .map_err(|e| e.to_string())
}

fn sweep_images(root: &std::path::Path, cites: &std::collections::HashSet<String>) -> CleanupSummary {
    let mut summary = CleanupSummary { deleted: 0, alleges: 0, freed: 0 };
    let recent = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    for base in [root.join("albums"), root.join("albums").join("albums"), root.join("artists")] {
        for size in ["full", "1x", "2x"] {
            let Ok(entries) = std::fs::read_dir(base.join(size)) else { continue };
            for entry in entries.flatten() {
                let path_part = entry.path();
                let Ok(meta) = entry.metadata() else { continue };
                if !meta.is_file() || meta.modified().map(|m| m > recent).unwrap_or(true) {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if !cites.contains(&name) {
                    if std::fs::remove_file(&path_part).is_ok() {
                        summary.deleted += 1;
                        summary.freed += meta.len();
                    }
                } else if size == "full" {
                    let Ok(bytes) = std::fs::read(&path_part) else { continue };
                    if let Some(light) = shrink_image(&bytes) {
                        if std::fs::write(&path_part, &light).is_ok() {
                            summary.alleges += 1;
                            summary.freed += (bytes.len() - light.len()) as u64;
                        }
                    }
                }
            }
        }
    }
    summary
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
pub async fn get_library_tag_keys(
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<Vec<TagColumnCandidate>, String> {
    let lines = LibraryTrackRepository::find_tags_json(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to read tags: {e}"))?;

    let mut effective: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for raw in lines {
        let Some(json) = raw else { continue };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) else {
            continue;
        };
        let Some(object) = value.as_object() else { continue };

        for (key, val) in object {
            match key.as_str() {
                // Les images n'ont rien à faire dans une colonne de texte, et
                // les tags libres sont dépouillés juste en dessous.
                "attached_images" => continue,
                "custom_tags" => {
                    if let Some(pairs) = val.as_array() {
                        for pair in pairs {
                            let Some(pair) = pair.as_array() else { continue };
                            let (Some(k), Some(v)) = (pair.first(), pair.get(1)) else {
                                continue;
                            };
                            if let (Some(k), Some(v)) = (k.as_str(), v.as_str()) {
                                if !v.trim().is_empty() {
                                    *effective.entry(format!("custom:{k}")).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
                _ => {
                    if is_filled(val) {
                        *effective.entry(key.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    let mut output: Vec<TagColumnCandidate> = effective
        .into_iter()
        .map(|(key, filled)| TagColumnCandidate { key, filled })
        .collect();

    // Les plus répandus d'abord ; à effectif égal, l'ordre alphabétique, pour
    // que deux appels successifs rendent la même liste.
    output.sort_by(|a, b| b.filled.cmp(&a.filled).then_with(|| a.key.cmp(&b.key)));

    Ok(output)
}

/// Un tag vaut d'être proposé s'il porte autre chose que du vide.
fn is_filled(val: &serde_json::Value) -> bool {
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
pub async fn get_mix_tracks(
    state: State<'_, AppState>,
    library_id: i64,
    kind: String,
    genre: Option<String>,
    decade: Option<i64>,
    year: Option<i64>,
    limit: i64,
) -> Result<Vec<TrackListRow>, String> {
    LibraryTrackRepository::find_mix_tracks(&state.pool, library_id, &kind, genre.as_deref(), decade, year, limit.clamp(1, 200))
        .await
        .map_err(|e| format!("get_mix_tracks: {}", e))
}

pub async fn get_tracks_by_genre(
    state: State<'_, AppState>,
    library_id: i64,
    genre: String,
) -> Result<Vec<TrackListRow>, String> {
    LibraryTrackRepository::find_tracks_by_genre(&state.pool, library_id, &genre)
        .await
        .map_err(|e| format!("get_tracks_by_genre: {}", e))
}

/// Les titres d'une période (une décennie, ou une année quand `start == end`).
pub async fn get_tracks_by_years(
    state: State<'_, AppState>,
    library_id: i64,
    start: i64,
    end: i64,
) -> Result<Vec<TrackListRow>, String> {
    LibraryTrackRepository::find_tracks_by_years(&state.pool, library_id, start, end)
        .await
        .map_err(|e| format!("get_tracks_by_years: {}", e))
}

/// Un genre de la bibliothèque assez varié pour faire un mix.
#[derive(serde::Serialize)]
pub struct GenreMix {
    #[serde(rename = "nom")]
    pub name: String,
    #[serde(rename = "titres")]
    pub tracks_count: i64,
    #[serde(rename = "artistes")]
    pub artists: i64,
}

/// Les genres qui font un vrai mix (assez d'artistes et de titres), les plus fournis d'abord.
pub async fn get_mix_genres(state: State<'_, AppState>, library_id: i64) -> Result<Vec<GenreMix>, String> {
    use crate::core::smart_playlist::mix_variety::{MIN_ARTISTS, MIN_TRACKS};
    let lines = LibraryGenreRepository::find_mix_genres(&state.pool, library_id, MIN_TRACKS, MIN_ARTISTS)
        .await
        .map_err(|e| format!("get_mix_genres: {e}"))?;
    Ok(lines.into_iter().map(|(name, tracks_count, artists)| GenreMix { name, tracks_count, artists }).collect())
}

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
    let mut artists: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (genre, name) in LibraryGenreRepository::find_top_artists(&state.pool, library_id).await {
        artists.entry(genre).or_default().push(name);
    }

    let mut genres = Vec::with_capacity(rows.len());
    for (name, total_albums, total_tracks, last_played_at) in rows {
        // Clé en minuscules ASCII, comme le `LOWER` de SQLite.
        let k = name.to_ascii_lowercase();
        let covers = covers.remove(&k).unwrap_or_default();
        let top_artists = artists.remove(&k).unwrap_or_default();
        genres.push(GenreView { name, total_albums, total_tracks, covers, top_artists, last_played_at });
    }

    Ok(genres)
}

// ============================================================================
// STATS
// ============================================================================

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
mod tests_sort {
    use super::resolve_sort;

    #[test]
    fn a_known_column_gives_its_expression_without_binding() {
        let (expr, bind) = resolve_sort(Some("duration"));
        assert_eq!(expr, "lt.duration");
        assert!(bind.is_none());
    }

    #[test]
    fn an_unknown_key_falls_back_to_natural_order() {
        // Le point important n'est pas la valeur rendue mais qu'aucune clé
        // étrangère ne puisse atteindre le SQL : tout ce qui n'est pas prévu
        // donne la même expression close.
        let (expected, _) = resolve_sort(None);
        for key in ["", "lt.title; DROP TABLE library_tracks", "../../etc", "RANDOM()"] {
            let (expr, bind) = resolve_sort(Some(key));
            assert_eq!(expr, expected, "clé : {key}");
            assert!(bind.is_none(), "clé : {key}");
        }
    }

    #[test]
    fn a_named_field_goes_out_as_a_bound_path() {
        let (expr, bind) = resolve_sort(Some("tag:composer"));
        assert_eq!(expr, "json_extract(lt.tags, ?) COLLATE NOCASE");
        assert_eq!(bind.as_deref(), Some("$.composer"));
    }

    #[test]
    fn a_free_tag_goes_out_as_a_bound_value() {
        let (expr, bind) = resolve_sort(Some("tag:custom:Label"));
        assert!(expr.contains("json_each"), "expression : {expr}");
        assert_eq!(bind.as_deref(), Some("Label"));
    }

    #[test]
    fn a_tag_name_never_joins_the_query_text() {
        // Un fichier de la bibliothèque peut porter n'importe quel nom de tag,
        // y compris hostile. L'expression rendue ne doit jamais le contenir :
        // il ne circule que comme valeur liée, que le moteur ne réinterprète
        // pas comme du code.
        let bad = [
            "x'; DROP TABLE library_tracks; --",
            "a\" OR 1=1 --",
            "'||(SELECT value FROM settings)||'",
        ];

        for name in bad {
            for key in [format!("tag:custom:{name}"), format!("tag:{name}")] {
                let (expr, bind) = resolve_sort(Some(&key));
                assert!(
                    !expr.contains(name),
                    "le nom a fuité dans l'expression : {expr}"
                );
                assert!(bind.is_some(), "le nom doit partir en liaison : {key}");
                assert!(
                    bind.as_deref().unwrap().contains(name),
                    "le nom doit être porté par la liaison"
                );
            }
        }
    }
}
