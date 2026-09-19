use std::path::PathBuf;
use std::time::Instant;
use chrono::Utc;
use rayon::prelude::*;
use serde::Serialize;
use sqlx::{SqlitePool, SqliteConnection};
use tauri::{Manager, Emitter};
use tauri::path::BaseDirectory;

use crate::entity::artist::artist::{Artist, ArtistCreate};
use crate::entity::library::library_album::{LibraryAlbum, LibraryAlbumCreate};
use crate::entity::library::library_album_artist::{LibraryAlbumArtistCreate};
use crate::entity::library::library_artist::{LibraryArtist, LibraryArtistCreate};
use crate::entity::library::library_cache::{LibraryCache, LibraryCacheCreate};
use crate::entity::library::library_dirs::{LibraryDir, LibraryDirCreate};
use crate::entity::library::library_track::{LibraryTrack, LibraryTrackCreate};
use crate::entity::library::library_track_artist::LibraryTrackArtistCreate;
use crate::helper::files::reader::read_dir_deep;
use crate::helper::library::thumbnail_helper::{migrate_old_thumbnails, thumbnail_saver};
use crate::helper::string::string::{normalize_name, normalize_sort_name, split_artists};
use crate::mapper::library::track::mapper_track::to_track_list_view;
use crate::repository::artist::artist_repository::ArtistRepository;
use crate::repository::library::library_album_artist_repository::LibraryAlbumArtistRepository;
use crate::repository::library::library_album_repository::LibraryAlbumRepository;
use crate::repository::library::library_artist_repository::LibraryArtistRepository;
use crate::repository::library::library_cache_repository::LibraryCacheRepository;
use crate::repository::library::library_dirs_repository::LibraryDirRepository;
use crate::repository::library::library_files_repository::LibraryFilesRepository;
use crate::repository::library::library_track_artist_repository::LibraryTrackArtistRepository;
use crate::repository::library::library_track_repository::LibraryTrackRepository;
use crate::{core::audio_analyser::audio_analyser::AudioAnalyser, entity::audio::audio_file::AudioFile};
use crate::entity::audio::audio_tags::AudioTags;
use crate::entity::library::library_files::{LibraryFile, LibraryFileCreate};
use crate::mapper::library::track::track_list_item_view::TrackListView;

// ============================================================================
// RÉSULTAT D'ANALYSE D'UN FICHIER (phase CPU, sync, rayon-compatible)
// ============================================================================
// Cette struct transporte les données entre la phase d'analyse (parallèle)
// et la phase d'écriture DB (séquentielle dans la transaction).

pub struct FileAnalysisResult {
    pub path: String,
    pub filename: String,
    pub extension: String,
    pub size: i64,
    pub file_modified: Option<String>,
    pub audio_file: AudioFile,
    pub thumbnail_url: Option<String>,
}

// Analyse un fichier audio de manière synchrone (compatible rayon)
// Cette fonction fait le travail CPU : lire les métadonnées du FS,
// parser le fichier avec symphonia, extraire la cover
fn analyse_file_sync(file: &PathBuf, covers_dir: &PathBuf) -> Result<FileAnalysisResult, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        analyse_file_inner(file, covers_dir)
    }))
    .unwrap_or_else(|_| {
        let path = file.to_str().unwrap_or("?");
        log::error!("⛔ PANIC attrapé lors de l'analyse de : {}", path);
        Err(format!("Panic during analysis of {}", path))
    })
}

fn analyse_file_inner(file: &PathBuf, covers_dir: &PathBuf) -> Result<FileAnalysisResult, String> {
    let path = file.to_str().ok_or("Invalid UTF-8 path")?.to_string();
    let filename = file.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
    let extension = file.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();

    let metadata = std::fs::metadata(&path).map_err(|e| format!("metadata: {}", e))?;
    let size = metadata.len() as i64;
    let file_modified = metadata.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs().to_string());

    let audio_file = AudioAnalyser::analyse_audio_file(file).map_err(|e| e.to_string())?;

    let thumbnail_url = audio_file.tags.attached_images.first().and_then(|img| {
        if img.image_data.is_empty() { return None; }
        thumbnail_saver(covers_dir, &img.image_data, false).ok()
    });

    Ok(FileAnalysisResult { path, filename, extension, size, file_modified, audio_file, thumbnail_url })
}

pub struct LibrarySaveContext {
    pub pool: SqlitePool,
    pub covers_dir: PathBuf,
}

pub fn create_context(
    app: tauri::AppHandle,
    pool_api: &SqlitePool
) -> LibrarySaveContext {
    let covers_dir: PathBuf = app
        .path()
        .resolve("covers", BaseDirectory::AppData)
        .unwrap_or_else(|_| PathBuf::from("covers"));

    LibrarySaveContext {
        pool: pool_api.clone(),
        covers_dir,
    }
}

// ============================================================================
// EVENTS DE PROGRESSION
// ============================================================================

#[derive(Clone, Serialize)]
pub struct ImportProgress {
    pub current: usize,
    pub total: usize,
    pub percent: usize,
    pub file_name: String,
}

#[derive(Clone, Serialize)]
pub struct ImportComplete {
    pub total: usize,
    pub duration_ms: u128,
    /// Number of files skipped because of an analysis or DB error.
    pub skipped: usize,
}

// ============================================================================
// IMPORT D'UN DOSSIER DANS LA BIBLIOTHÈQUE
// ============================================================================
//
// Fonctionnement :
// 1. On scanne le dossier récursivement pour lister tous les fichiers audio
// 2. On ouvre UNE SEULE transaction SQLite (BEGIN IMMEDIATE)
//    → Toutes les insertions se font en mémoire, pas de fsync entre chaque
//    → C'est ~10-50x plus rapide que des commits individuels
// 3. Pour chaque fichier, on émet un event Tauri "import-progress" au frontend
// 4. À la fin, on COMMIT (1 seul fsync disque) et on émet "import-complete"
//
// Pourquoi BEGIN IMMEDIATE ?
// - BEGIN simple = shared lock (d'autres peuvent lire)
// - BEGIN IMMEDIATE = reserved lock (on est sûr de pouvoir écrire)
// - Évite les erreurs "database is locked" si un autre thread lit en même temps
//
// ============================================================================

pub async fn save_dir_to_library(
    app: tauri::AppHandle,
    pool_api: &SqlitePool,
    library_id: i64,
    directory: String
) -> Result<Vec<TrackListView>, String> {

    let start: Instant = Instant::now();

    let mut files: Vec<PathBuf> = Vec::new();
    let mut tracks: Vec<TrackListView> = Vec::new();

    // declaration de mes repository
    let ctx: LibrarySaveContext = create_context(app.clone(), pool_api);

    // On ajoute le dossier (hors transaction, c'est 1 seule requête)
    let library_dir: LibraryDir = match LibraryDirRepository::find_by_path(pool_api, library_id, &directory).await {
        Ok(Some(library_dir)) => library_dir,
        Ok(None) => {
            LibraryDirRepository::insert_library_dir(pool_api, LibraryDirCreate {
                    library_id,
                    path: directory.clone(),
                    name: PathBuf::from(&directory)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown")
                        .to_string(),
                    is_recursive: true,
                    is_active: true,
                    watch_enabled: true,
                    include_patterns: None,
                    exclude_patterns: None,
                })
                .await
                .map_err(|e| e.to_string())?
        },
        Err(e) => return Err(format!("Failed to find directory: {}", e)),
    };

    // Migrations miniatures (covers → covers/albums/, artists → covers/artists/)
    migrate_old_thumbnails(&app, &ctx.covers_dir, pool_api, "covers").await?;
    migrate_old_thumbnails(&app, &ctx.covers_dir, pool_api, "artists").await?;

    // Scan récursif du dossier
    let _ = read_dir_deep(&directory, &mut files);

    let total: usize = files.len();
    log::info!("📁 {} fichiers trouvés dans {}", total, directory);

    // ─── OUVRIR LA TRANSACTION ───────────────────────────────────────
    let mut tx = pool_api.begin().await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // ─── IMPORT PAR BATCH DE 8 (analyse parallèle + DB séquentielle) ───
    //
    // Pour chaque batch de 8 fichiers :
    //   1. rayon analyse les 8 fichiers en parallèle sur tous les cores CPU
    //   2. On récupère les résultats (Vec<Result<FileAnalysisResult>>)
    //   3. On écrit chaque résultat en DB séquentiellement (transaction unique)
    //   4. On émet la progression fichier par fichier pendant la phase DB
    //
    // Pourquoi 8 ? C'est un bon compromis entre :
    //   - Utilisation CPU (assez de fichiers pour occuper tous les cores)
    //   - Mémoire (max 8 AudioFile + covers en RAM simultanément)
    //   - Latence de progression (pause max = temps d'analyse de 8 fichiers)

    const BATCH_SIZE: usize = 8;
    let mut processed: usize = 0;
    let mut skipped: Vec<(PathBuf, String)> = Vec::new();
    let covers_dir = ctx.covers_dir.clone();

    for chunk in files.chunks(BATCH_SIZE) {

        // ── Phase 1 : Analyse parallèle (CPU-bound, sync, rayon) ──
        // Double protection :
        // 1. catch_unwind dans analyse_file_sync (par fichier)
        // 2. catch_unwind autour du batch rayon entier (si un thread crash malgré tout)
        // Si le batch crash → on retente les fichiers un par un (séquentiel)
        let analyses: Vec<(PathBuf, Result<FileAnalysisResult, String>)> = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            chunk
                .par_iter()
                .map(|file| {
                    let result = analyse_file_sync(file, &covers_dir);
                    (file.clone(), result)
                })
                .collect::<Vec<_>>()
        })) {
            Ok(results) => results,
            Err(_) => {
                log::error!("⛔ PANIC batch rayon — fallback séquentiel pour {:?}",
                    chunk.iter().filter_map(|f| f.file_name().map(|n| n.to_string_lossy().to_string())).collect::<Vec<_>>());
                chunk.iter().map(|file| {
                    let result = analyse_file_sync(file, &covers_dir);
                    (file.clone(), result)
                }).collect()
            }
        };

        // ── Phase 2 : Écriture DB séquentielle (async, transaction) ──
        // On itère les résultats un par un et on écrit dans la transaction
        // La progression est émise après chaque fichier → UX fluide
        for (file, analysis_result) in analyses {
            processed += 1;

            let file_name = file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();

            // Émettre la progression
            let _ = app.emit("import-progress", ImportProgress {
                current: processed,
                total,
                percent: if total > 0 { (processed * 100) / total } else { 0 },
                file_name: file_name.clone(),
            });

            let analysis = match analysis_result {
                Ok(a) => a,
                Err(e) => {
                    log::warn!("⚠️ Analyse ignorée {} : {}", file_name, e);
                    skipped.push((file.clone(), e));
                    continue;
                }
            };

            // Écrire le résultat analysé en DB via save_analysed_to_db
            match save_analysed_to_db(
                &mut *tx, library_id, Some(library_dir.id.clone()), analysis
            ).await {
                Ok(track) => tracks.push(track),
                Err(e) => {
                    log::warn!("⚠️ DB ignoré {} : {}", file_name, e);
                    skipped.push((file.clone(), e));
                }
            }
        }
    }

    // ─── COMMIT ─────────────────────────────────────────────────────
    tx.commit().await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    let elapsed = start.elapsed();
    log::info!(
        "✅ Import terminé : {} fichiers en {:.2}s ({:.0} fichiers/sec)\
         {}",
        tracks.len(),
        elapsed.as_secs_f64(),
        tracks.len() as f64 / elapsed.as_secs_f64().max(0.001),
        if skipped.is_empty() {
            String::new()
        } else {
            format!(" — {} fichier(s) ignoré(s)", skipped.len())
        }
    );

    // Mettre à jour les stats du dossier importé
    let total_size: i64 = tracks.iter()
        .map(|t| t.size)
        .sum();
    let _ = LibraryDirRepository::update_scan_result(
        pool_api, &library_dir.id,
        tracks.len() as i64, total_size,
        "completed", None,
    ).await;


    //clear old directory


    // Émettre l'événement de fin au frontend
    let _ = app.emit("import-complete", ImportComplete {
        total: tracks.len(),
        duration_ms: elapsed.as_millis(),
        skipped: skipped.len(),
    });

    Ok(tracks)
}


// ============================================================================
// ÉCRITURE DB D'UN FICHIER ANALYSÉ (phase DB, async, séquentielle)
// ============================================================================
// Prend le résultat d'analyse (CPU) et écrit tout en DB dans la transaction.
// Gère aussi le skip si le fichier est déjà indexé et inchangé.

async fn save_analysed_to_db(
    conn: &mut SqliteConnection,
    library_id: i64,
    library_dir_id: Option<String>,
    analysis: FileAnalysisResult,
) -> Result<TrackListView, String> {
    let now = Utc::now();

    // ─── Upsert le fichier en DB ───
    let library_file: LibraryFile = LibraryFilesRepository::insert_library_file(&mut *conn, LibraryFileCreate {
        library_id, library_dir_id, cache_id: None,
        path: analysis.path.clone(), filename: analysis.filename.clone(),
        extension: analysis.extension.clone(), size: analysis.size,
        file_hash: None, modified_at: analysis.file_modified.clone(),
        status: "pending".to_string(), is_available: true, error_message: None,
        created_at: now.clone(), updated_at: now, last_verified_at: None,
    }).await.map_err(|e| format!("Failed to insert file: {}", e))?;

    // ─── Skip si fichier inchangé ───
    if library_file.status == "indexed" && library_file.modified_at == analysis.file_modified {
        if let Ok(Some(existing_view)) = LibraryTrackRepository::find_track_view_by_file_id(
            &mut *conn, &library_file.id
        ).await {
            return Ok(existing_view);
        }
    }

    // ─── Cache ───
    let audio_format_string = format!("{:?}", analysis.audio_file.audio_format);
    let data = LibraryCacheCreate {
        path: analysis.audio_file.path.clone(),
        title: analysis.audio_file.tags.title.clone(), artist: analysis.audio_file.tags.artist.clone(),
        album: analysis.audio_file.tags.album.clone(), album_artist: analysis.audio_file.tags.album_artist.clone(),
        year: analysis.audio_file.tags.year.clone(), genre: analysis.audio_file.tags.genre.clone(),
        track_number: analysis.audio_file.tags.track_number.map(|n| n as i32),
        disc_number: analysis.audio_file.tags.disc_number.map(|n| n as i32),
        duration: Some(analysis.audio_file.duration as f64), bitrate: Some(analysis.audio_file.bitrate as i32),
        bits_per_sample: Some(analysis.audio_file.bits_per_sample as i32),
        sample_rate: Some(analysis.audio_file.sample_rate as i32), channels: Some(analysis.audio_file.channels as i32),
        audio_format: Some(audio_format_string), mime_type: None,
        file_size: Some(analysis.audio_file.file_size as i64), extra_tags: None,
        thumbnail_path: analysis.thumbnail_url.clone(),
    };

    let library_cache = LibraryCacheRepository::upsert_library_cache(&mut *conn, data)
        .await.map_err(|e| format!("Failed to upsert cache: {}", e))?;

    let audio_tags = &analysis.audio_file.tags;

    // ─── Artistes (split multi-artistes) ───
    let raw_artist = audio_tags.artist.as_deref().unwrap_or("Unknown Artist");
    let artist_names = split_artists(raw_artist);

    // Le premier artiste est le "principal" (affiché sur la track)
    let mut all_artist_ids: Vec<String> = Vec::new();
    let mut first_artist: Option<Artist> = None;
    let mut first_library_artist: Option<LibraryArtist> = None;

    for (i, artist_name) in artist_names.iter().enumerate() {
        let artist = ArtistRepository::insert_artist(&mut *conn, ArtistCreate {
            name: artist_name.clone(),
            name_normalized: normalize_name(artist_name),
            sort_name: normalize_sort_name(artist_name),
        }).await.map_err(|e| format!("Failed to insert artist: {}", e))?;

        let lib_artist = LibraryArtistRepository::insert_library_artist(&mut *conn, LibraryArtistCreate {
            library_id, artist_id: artist.id.clone(),
        }).await.map_err(|e| format!("Failed to insert library_artist: {}", e))?;

        all_artist_ids.push(artist.id.clone());
        if i == 0 {
            first_artist = Some(artist);
            first_library_artist = Some(lib_artist);
        }
    }

    let artist = first_artist.ok_or("Aucun artiste créé")?;
    let artist_id: Option<String> = Some(artist.id.clone());
    let mut main_artist_id: Option<String> = artist_id.clone();
    let library_artist: Option<LibraryArtist> = first_library_artist;

    // Album artist (si différent du premier track artist)
    let mut artist_album_id: Option<String> = None;
    if let Some(album_artist_raw) = analysis.audio_file.tags.album_artist.as_deref() {
        let album_artist_names = split_artists(album_artist_raw);
        for album_artist_name in &album_artist_names {
            let album_artist_normalized = normalize_name(album_artist_name);
            if album_artist_normalized != normalize_name(&artist_names[0]) {
                let album_artist = ArtistRepository::insert_artist(&mut *conn, ArtistCreate {
                    name: album_artist_name.clone(),
                    name_normalized: album_artist_normalized,
                    sort_name: normalize_sort_name(album_artist_name),
                }).await.map_err(|e| format!("Failed to insert artist (album): {}", e))?;
                let _ = LibraryArtistRepository::insert_library_artist(&mut *conn, LibraryArtistCreate {
                    library_id, artist_id: album_artist.id.clone(),
                }).await;
                // Le premier album_artist différent devient le main pour l'album
                if artist_album_id.is_none() {
                    artist_album_id = Some(album_artist.id.clone());
                    main_artist_id = artist_album_id.clone();
                }
            }
        }
    }

    // ─── Album ───
    let year: Option<i32> = audio_tags.year.as_deref().and_then(|s| s.parse::<i32>().ok());
    let genre: Option<String> = audio_tags.genre.clone();
    let album_title = audio_tags.album.as_deref().unwrap_or("Unknown").to_string();
    let album_title_normalized = normalize_name(&album_title);
    let mut library_album_id: Option<String> = None;
    let mut library_album: Option<LibraryAlbum> = None;

    if let Some(main_artist_id_ok) = main_artist_id.clone() {
        let album = LibraryAlbumRepository::insert_library_album(&mut *conn, LibraryAlbumCreate {
            library_id, artist_id: main_artist_id_ok,
            title: album_title.clone(), title_normalized: album_title_normalized,
            year, genre, cover_url: analysis.thumbnail_url.clone(), album_type: Some("album".to_string()),
        }).await.map_err(|e| format!("Failed to insert album: {}", e))?;
        library_album_id = Some(album.id.clone());
        library_album = Some(album);
    }

    // ─── Track ───
    let title = audio_tags.title.as_deref().unwrap_or(&library_file.filename).to_string();
    let disc_number = audio_tags.disc_number.unwrap_or(1) as i32;
    let track_number = audio_tags.track_number.map(|n| n as i32);
    let title_normalized = title.to_lowercase();

    let library_track = LibraryTrackRepository::insert_library_track(&mut *conn, LibraryTrackCreate {
        library_id: library_file.library_id, file_id: library_file.id.clone(),
        cache_id: Some(library_cache.id.clone()), artist_id: main_artist_id.clone(),
        library_album_id: library_album_id.clone(), title, title_normalized,
        track_number, disc_number,
        tags: {
            // Stocker les tags SANS les images (trop lourd en base64)
            let mut tags_light = audio_tags.clone();
            tags_light.attached_images.clear();
            serde_json::to_string(&tags_light).ok()
        },
        duration: Some(analysis.audio_file.duration as f64), bitrate: Some(analysis.audio_file.bitrate as i32),
        sample_rate: Some(analysis.audio_file.sample_rate as i32),
        rating: audio_tags.rating,
    }).await.map_err(|e| format!("Failed to insert track: {}", e))?;

    // ─── Liaison artistes ↔ track (tous les artistes splittés) ───
    for aid in &all_artist_ids {
        if let Err(e) = LibraryTrackArtistRepository::insert_library_track_artist(&mut *conn, LibraryTrackArtistCreate {
            library_id, artist_id: aid.clone(), library_track_id: library_track.id.clone(),
        }).await {
            // Cette liaison porte les artistes secondaires — les « feat. », les
            // duos. Sans elle ils ne sont crédités nulle part. L'échec était
            // avalé : la table pouvait rester vide de bout en bout sans qu'une
            // seule ligne de journal ne le signale.
            log::warn!("⚠️ Liaison artiste ↔ piste refusée ({} / {}) : {}", aid, library_track.id, e);
        }
    }

    // Album artist(s) → liaison track + album
    if let Some(ref album_art_id) = artist_album_id {
        // Lier l'album artist au track si pas déjà fait
        if !all_artist_ids.contains(album_art_id) {
            let _ = LibraryTrackArtistRepository::insert_library_track_artist(&mut *conn, LibraryTrackArtistCreate {
                library_id, artist_id: album_art_id.clone(), library_track_id: library_track.id.clone(),
            }).await;
        }
        // Lier l'album artist à l'album
        if let Some(ref lib_album_id) = library_album_id {
            let _ = LibraryAlbumArtistRepository::insert_library_album_artist(&mut *conn, LibraryAlbumArtistCreate {
                library_id, artist_id: album_art_id.clone(), library_album_id: lib_album_id.clone(),
            }).await;
        }
    }

    // ─── Marquer comme indexé ───
    let _ = LibraryFilesRepository::mark_as_indexed(
        &mut *conn, &library_file.id, analysis.file_modified.as_deref(),
    ).await;

    Ok(to_track_list_view(&library_track, &library_file, &library_cache, &artist, library_artist.as_ref(), library_album.as_ref()))
}

// Version pool — pour l'import de fichiers individuels (hors transaction)
pub async fn save_track_to_library(
    ctx: &LibrarySaveContext,
    library_id: i64,
    library_dir_id: Option<String>,
    file: String
) -> Result<TrackListView, String> {
    let mut conn = ctx.pool.acquire().await.map_err(|e| format!("Failed to acquire connection: {}", e))?;
    save_track_to_library_tx(&mut *conn, &ctx.covers_dir, library_id, library_dir_id, file).await
}

// Version transactionnelle — reçoit &mut SqliteConnection de la transaction
// Chaque appel utilise &mut *conn (reborrow) pour ne pas consommer la connexion
pub async fn save_track_to_library_tx(
    conn: &mut SqliteConnection,
    covers_dir: &PathBuf,
    library_id: i64,
    library_dir_id: Option<String>,
    file: String
) -> Result<TrackListView, String> {

        let file_buf: PathBuf = PathBuf::from(&file);

        // ─── ÉTAPE 1 : Lire les métadonnées du système de fichiers ───
        // std::fs::metadata() est très rapide (~0.01ms) car il lit juste l'inode
        // On récupère : taille du fichier + date de dernière modification
        let path_buf_file: PathBuf = PathBuf::from(&file);
        let filename: String = path_buf_file.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let extension: String = path_buf_file.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
        let metadata = std::fs::metadata(&file).map_err(|e| format!("Failed to read file metadata: {}", e))?;
        let size: i64 = metadata.len() as i64;

        // Convertir SystemTime en timestamp string pour stockage en DB
        // UNIX_EPOCH = 1er janvier 1970, on calcule les secondes écoulées depuis
        let file_modified: Option<String> = metadata.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs().to_string());

        let now = Utc::now();

        // ─── ÉTAPE 2 : Upsert le fichier en DB ──────────────────────
        // ON CONFLICT(library_id, path) → si le fichier existe déjà, on retourne la ligne existante
        // On passe le modified_at du disque pour les nouveaux fichiers
        let library_file: LibraryFile = LibraryFilesRepository::insert_library_file(&mut *conn, LibraryFileCreate {
            library_id, library_dir_id, cache_id: None,
            path: file.clone(), filename, extension, size,
            file_hash: None, modified_at: file_modified.clone(),
            status: "pending".to_string(), is_available: true, error_message: None,
            created_at: now.clone(), updated_at: now, last_verified_at: None,
        }).await.map_err(|e| format!("Failed to insert file: {}", e))?;

        // ─── ÉTAPE 3 : Skip si fichier inchangé ─────────────────────
        // Comparer la date de modification du fichier sur disque vs en base
        // Si identiques ET le fichier a déjà été indexé (status != "pending")
        // → Le fichier n'a pas changé depuis le dernier import
        // → On récupère le TrackListView existant depuis la DB (1 requête SQL rapide)
        // → On économise l'analyse audio complète (symphonia ~5-50ms par fichier)
        if library_file.status == "indexed"
            && library_file.modified_at == file_modified
        {
            // Le fichier est déjà importé et n'a pas changé → skip l'analyse
            if let Ok(Some(existing_view)) = LibraryTrackRepository::find_track_view_by_file_id(
                &mut *conn, &library_file.id
            ).await {
                return Ok(existing_view);
            }
            // Si on ne trouve pas le track (données incohérentes), on continue l'import normal
        }

        // ─── ÉTAPE 4 : Analyse audio (seulement si fichier nouveau ou modifié) ───
        // C'est l'opération la plus coûteuse : symphonia ouvre le fichier, lit les headers,
        // parse les tags ID3/Vorbis/FLAC, extrait les images embarquées, calcule la durée
        let audio_file: AudioFile = AudioAnalyser::analyse_audio_file(&file_buf).map_err(|e| e.to_string())?;

        let thumbnail_url: Option<String> = audio_file.tags.attached_images.first().and_then(|img| {
            if img.image_data.is_empty() { return None; }
            thumbnail_saver(covers_dir, &img.image_data, false).ok()
        });

        // --- Cache ---
        let audio_format_string: String = format!("{:?}", audio_file.audio_format);
        let data: LibraryCacheCreate = LibraryCacheCreate {
            path: audio_file.path.clone(),
            title: audio_file.tags.title.clone(), artist: audio_file.tags.artist.clone(),
            album: audio_file.tags.album.clone(), album_artist: audio_file.tags.album_artist.clone(),
            year: audio_file.tags.year.clone(), genre: audio_file.tags.genre.clone(),
            track_number: audio_file.tags.track_number.map(|n| n as i32),
            disc_number: audio_file.tags.disc_number.map(|n| n as i32),
            duration: Some(audio_file.duration as f64), bitrate: Some(audio_file.bitrate as i32),
            bits_per_sample: Some(audio_file.bits_per_sample as i32),
            sample_rate: Some(audio_file.sample_rate as i32), channels: Some(audio_file.channels as i32),
            audio_format: Some(audio_format_string), mime_type: None,
            file_size: Some(audio_file.file_size as i64), extra_tags: None,
            thumbnail_path: thumbnail_url.clone(),
        };

        let library_cache: LibraryCache = LibraryCacheRepository::upsert_library_cache(&mut *conn, data)
            .await.map_err(|e| format!("Failed to upsert cache: {}", e))?;

        let audio_tags: &AudioTags = &audio_file.tags;

        // ─── Artistes (split multi-artistes) ───
        let raw_artist = audio_tags.artist.as_deref().unwrap_or("Unknown Artist");
        let artist_names = split_artists(raw_artist);

        let mut all_artist_ids: Vec<String> = Vec::new();
        let mut first_artist: Option<Artist> = None;
        let mut first_library_artist: Option<LibraryArtist> = None;

        for (i, artist_name) in artist_names.iter().enumerate() {
            let artist = ArtistRepository::insert_artist(&mut *conn, ArtistCreate {
                name: artist_name.clone(),
                name_normalized: normalize_name(artist_name),
                sort_name: normalize_sort_name(artist_name),
            }).await.map_err(|e| format!("Failed to insert artist: {}", e))?;

            let lib_artist = LibraryArtistRepository::insert_library_artist(&mut *conn, LibraryArtistCreate {
                library_id, artist_id: artist.id.clone(),
            }).await.map_err(|e| format!("Failed to insert library_artist: {}", e))?;

            all_artist_ids.push(artist.id.clone());
            if i == 0 {
                first_artist = Some(artist);
                first_library_artist = Some(lib_artist);
            }
        }

        let artist = first_artist.ok_or("Aucun artiste créé")?;
        let artist_id: Option<String> = Some(artist.id.clone());
        let mut main_artist_id: Option<String> = artist_id.clone();
        let library_artist: Option<LibraryArtist> = first_library_artist;

        let mut artist_album_id: Option<String> = None;
        if let Some(album_artist_raw) = audio_file.tags.album_artist.as_deref() {
            let album_artist_names = split_artists(album_artist_raw);
            for album_artist_name in &album_artist_names {
                let album_artist_normalized = normalize_name(album_artist_name);
                if album_artist_normalized != normalize_name(&artist_names[0]) {
                    let album_artist = ArtistRepository::insert_artist(&mut *conn, ArtistCreate {
                        name: album_artist_name.clone(),
                        name_normalized: album_artist_normalized,
                        sort_name: normalize_sort_name(album_artist_name),
                    }).await.map_err(|e| format!("Failed to insert artist (album): {}", e))?;
                    let _ = LibraryArtistRepository::insert_library_artist(&mut *conn, LibraryArtistCreate {
                        library_id, artist_id: album_artist.id.clone(),
                    }).await;
                    if artist_album_id.is_none() {
                        artist_album_id = Some(album_artist.id.clone());
                        main_artist_id = artist_album_id.clone();
                    }
                }
            }
        }

        // --- Album ---
        let year: Option<i32> = audio_tags.year.as_deref().and_then(|s| s.parse::<i32>().ok());
        let genre: Option<String> = audio_tags.genre.clone();
        let album_title: String = audio_tags.album.as_deref().unwrap_or("Unknown").to_string();
        let album_title_normalized = normalize_name(&album_title);
        let mut library_album_id: Option<String> = None;
        let mut library_album: Option<LibraryAlbum> = None;

        if let Some(main_artist_id_ok) = main_artist_id.clone() {
            let album: LibraryAlbum = LibraryAlbumRepository::insert_library_album(&mut *conn, LibraryAlbumCreate {
                library_id, artist_id: main_artist_id_ok,
                title: album_title.clone(), title_normalized: album_title_normalized,
                year, genre, cover_url: thumbnail_url.clone(), album_type: Some("album".to_string()),
            }).await.map_err(|e| format!("Failed to insert album: {}", e))?;
            library_album_id = Some(album.id.clone());
            library_album = Some(album);
        }

        // --- Track (upsert — 1 requête au lieu de SELECT + INSERT) ---
        let title = audio_tags.title.as_deref().unwrap_or(&library_file.filename).to_string();
        let disc_number = audio_tags.disc_number.unwrap_or(1) as i32;
        let track_number = audio_tags.track_number.map(|n| n as i32);
        let title_normalized = title.to_lowercase();

        let library_track: LibraryTrack = LibraryTrackRepository::insert_library_track(&mut *conn, LibraryTrackCreate {
            library_id: library_file.library_id, file_id: library_file.id.clone(),
            cache_id: Some(library_cache.id.clone()), artist_id: main_artist_id.clone(),
            library_album_id: library_album_id.clone(), title, title_normalized,
            track_number, disc_number,
            tags: {
                let mut tags_light = audio_tags.clone();
                tags_light.attached_images.clear();
                serde_json::to_string(&tags_light).ok()
            },
            duration: Some(audio_file.duration as f64), bitrate: Some(audio_file.bitrate as i32),
            sample_rate: Some(audio_file.sample_rate as i32),
            rating: audio_tags.rating,
        }).await.map_err(|e| format!("Failed to insert track: {}", e))?;

        // ─── Liaison artistes ↔ track (tous les artistes splittés) ───
        for aid in &all_artist_ids {
            if let Err(e) = LibraryTrackArtistRepository::insert_library_track_artist(&mut *conn, LibraryTrackArtistCreate {
                library_id, artist_id: aid.clone(), library_track_id: library_track.id.clone(),
            }).await {
                log::warn!("⚠️ Liaison artiste ↔ piste refusée ({} / {}) : {}", aid, library_track.id, e);
            }
        }

        if let Some(ref album_art_id) = artist_album_id {
            if !all_artist_ids.contains(album_art_id) {
                let _ = LibraryTrackArtistRepository::insert_library_track_artist(&mut *conn, LibraryTrackArtistCreate {
                    library_id, artist_id: album_art_id.clone(), library_track_id: library_track.id.clone(),
                }).await;
            }
            if let Some(ref lib_album_id) = library_album_id {
                let _ = LibraryAlbumArtistRepository::insert_library_album_artist(&mut *conn, LibraryAlbumArtistCreate {
                    library_id, artist_id: album_art_id.clone(), library_album_id: lib_album_id.clone(),
                }).await;
            }
        }

        // ─── ÉTAPE FINALE : Marquer le fichier comme "indexed" ────────
        // On met à jour le status et le modified_at pour que le prochain scan
        // puisse détecter que ce fichier a déjà été traité et n'a pas changé
        let _ = LibraryFilesRepository::mark_as_indexed(
            &mut *conn,
            &library_file.id,
            file_modified.as_deref(),
        ).await;

        Ok(to_track_list_view(&library_track, &library_file, &library_cache, &artist, library_artist.as_ref(), library_album.as_ref()))
}

