//! Synchronisation d'une bibliothèque distante : le catalogue du serveur passe par le même
//! enregistrement que le scan d'un dossier. Seul ce qui a changé est réécrit.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, Instant};

use sqlx::SqlitePool;
use tauri::Emitter;

use crate::core::subsonic::catalog::RemoteSong;
use crate::entity::audio::audio_file::{AudioFile, AudioFormat};
use crate::entity::audio::audio_tags::AudioTags;
use crate::helper::library::thumbnail_helper::thumbnail_saver;
use crate::helper::task::TaskToken;
use crate::repository::library::library_files_repository::LibraryFilesRepository;
use crate::repository::library::library_repository::LibraryRepository;
use crate::repository::server::server_repository::ServerRepository;
use crate::service::external_server::{feedback_service, playlist_service};
use crate::service::library::library_service::{create_context, save_analysed_to_db, FileAnalysisResult, ImportComplete, ImportListing, ImportProgress, ImportStart};

/// Écritures validées par lots : une transaction par paquet de morceaux.
const BATCH: usize = 100;

/// Chemin d'un morceau distant : `remote://<serveur>/<album>/<morceau>` (l'album groupe comme un dossier).
pub fn song_path(server_id: i64, song: &RemoteSong) -> String {
    format!("{}{server_id}/{}/{}", crate::core::remote::SCHEME, song.album_id.as_deref().unwrap_or("_"), song.id)
}

pub(crate) fn audio_format(suffix: Option<&str>) -> AudioFormat {
    match suffix.unwrap_or("") {
        "mp3" => AudioFormat::MP3,
        "flac" => AudioFormat::FLAC,
        "ogg" | "oga" | "opus" => AudioFormat::OGG,
        "m4a" | "mp4" | "aac" => AudioFormat::AAC,
        "alac" => AudioFormat::ALAC,
        "wav" => AudioFormat::WAV,
        "dsf" => AudioFormat::DSF,
        "dff" => AudioFormat::DFF,
        _ => AudioFormat::Unknown,
    }
}

/// Le morceau sous la forme d'un fichier analysé, prêt pour l'enregistrement commun.
fn as_analysis(path: String, song: &RemoteSong, thumbnail: Option<String>) -> FileAnalysisResult {
    let mut tags = AudioTags::new();
    tags.title = Some(song.title.clone());
    tags.artist = song.artist.clone();
    tags.album = song.album.clone();
    tags.album_artist = song.album_artist.clone();
    tags.year = song.year.map(|y| y.to_string());
    tags.genre = song.genre.clone();
    tags.track_number = song.track;
    tags.disc_number = song.disc;
    tags.rating = song.user_rating;
    tags.duration = song.duration.map(|d| d as u32);
    let extension = song.suffix.clone().unwrap_or_default();
    // Nom affiché : celui du fichier sur le serveur, sinon le titre.
    let filename = song
        .server_path
        .as_deref()
        .and_then(|p| p.rsplit(['/', '\\']).next())
        .map(str::to_string)
        .unwrap_or_else(|| if extension.is_empty() { song.title.clone() } else { format!("{}.{extension}", song.title) });
    let audio_file = AudioFile {
        path: path.clone(),
        audio_format: audio_format(song.suffix.as_deref()),
        file_size: song.size.unwrap_or(0).max(0) as u64,
        duration: song.duration.unwrap_or(0.0) as f32,
        bits_per_sample: song.bit_depth.unwrap_or(0),
        bitrate: song.bit_rate.unwrap_or(0),
        sample_rate: song.sampling_rate.unwrap_or(0),
        channels: song.channels.unwrap_or(2) as usize,
        track_id: None,
        tags,
    };
    FileAnalysisResult {
        path,
        filename,
        extension,
        size: song.size.unwrap_or(0),
        // L'empreinte tient lieu de date : inchangée, le morceau n'est pas réécrit.
        file_modified: Some(song.fingerprint()),
        audio_file,
        thumbnail_url: thumbnail,
    }
}

/// Étapes de la synchronisation, relayées à l'interface comme un import.
enum Step {
    Listing(usize),
    Progress { done: usize, total: usize, written: usize, name: String },
}

/// Synchronise la bibliothèque distante `library_id` ; s'arrête entre deux lots si `token` est annulé.
pub async fn sync_library(app: tauri::AppHandle, pool: &SqlitePool, library_id: i64, token: &TaskToken) -> Result<(), String> {
    let start = Instant::now();
    let library = LibraryRepository::find_library_by_id(pool, library_id).await.map_err(|e| e.to_string())?;
    let label = library.server_name.clone().unwrap_or_else(|| library.name.clone());
    let _ = app.emit("import-start", ImportStart { library_id, directory: label.clone() });
    let covers_dir = create_context(app.clone(), pool).covers_dir;
    let result = run(pool, &covers_dir, library_id, token, |step| match step {
        Step::Listing(found) => {
            let _ = app.emit("import-listing", ImportListing { library_id, directory: label.clone(), found });
        }
        Step::Progress { done, total, written, name } => {
            let _ = app.emit("import-progress", ImportProgress {
                library_id,
                directory: label.clone(),
                current: done,
                total,
                percent: (done * 100).checked_div(total).unwrap_or(100),
                file_name: name,
                added: written,
            });
        }
    })
    .await;
    let (total, cancelled) = result.as_ref().map_or((0, false), |r| (r.0, r.1));
    if result.is_ok() {
        crate::service::external_server::offline_service::kick(app.clone(), pool.clone());
    }
    let _ = app.emit("import-complete", ImportComplete {
        library_id,
        directory: label,
        total,
        cancelled,
        duration_ms: start.elapsed().as_millis(),
        skipped: 0,
    });
    result.map(|_| ())
}

/// Le cœur, sans l'appli : rend (morceaux sur le serveur, interrompu).
async fn run(pool: &SqlitePool, covers_dir: &Path, library_id: i64, token: &TaskToken, mut report: impl FnMut(Step)) -> Result<(usize, bool), String> {
    let start = Instant::now();
    let library = LibraryRepository::find_library_by_id(pool, library_id).await.map_err(|e| e.to_string())?;
    let (Some(server_id), Some(folder_id)) = (library.server_id, remote_folder(pool, library_id).await) else {
        return Err("bibliothèque locale".into());
    };
    let server = ServerRepository::server(pool, server_id).await?;
    // Ce qui attendait (favoris, notes, écoutes faits hors connexion) part d'abord : le serveur fera foi ensuite.
    feedback_service::flush(pool, server_id, &server).await?;
    let songs = server.all_songs(&folder_id, |found| report(Step::Listing(found))).await?;
    let total = songs.len();
    log::info!("☁️  {} : {total} morceaux sur le serveur", library.name);

    let indexes = LibraryFilesRepository::indexed_by_path(pool, library_id).await.unwrap_or_default();
    // Une pochette par album : Navidrome en donne une par morceau, identique.
    let mut covers: HashMap<String, Option<String>> = HashMap::new();
    let mut seen: HashSet<String> = HashSet::with_capacity(total);
    let (mut done, mut written, mut cancelled) = (0usize, 0usize, false);
    let mut last_emit: Option<Instant> = None;

    for chunk in songs.chunks(BATCH) {
        if token.is_cancelled() {
            cancelled = true;
            break;
        }
        // Pochettes du lot d'abord (réseau), écriture ensuite (transaction courte).
        let mut pending = Vec::new();
        for song in chunk {
            let path = song_path(server_id, song);
            seen.insert(path.clone());
            let fingerprint = song.fingerprint();
            if indexes.get(&path).is_some_and(|(_, m)| m.as_deref() == Some(fingerprint.as_str())) {
                continue;
            }
            let key = song.album_id.clone().or_else(|| song.cover_art.clone()).unwrap_or_default();
            if !covers.contains_key(&key) {
                let thumb = match song.cover_art.as_deref() {
                    Some(id) => fetch_cover(&server, id, covers_dir).await,
                    None => None,
                };
                covers.insert(key.clone(), thumb);
            }
            pending.push(as_analysis(path, song, covers.get(&key).cloned().flatten()));
        }
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.map_err(|e| e.to_string())?;
        for analysis in pending {
            let name = analysis.filename.clone();
            match save_analysed_to_db(&mut tx, library_id, None, analysis).await {
                Ok(_) => written += 1,
                Err(e) => log::warn!("☁️  Morceau ignoré ({name}) : {e}"),
            }
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        done += chunk.len();
        if last_emit.is_none_or(|t| t.elapsed() >= Duration::from_millis(100)) || done == total {
            last_emit = Some(Instant::now());
            report(Step::Progress { done, total, written, name: chunk.last().map(|s| s.title.clone()).unwrap_or_default() });
        }
    }

    // Disparus du serveur (synchronisation complète seulement) : retirés, avec les albums vidés.
    let removed = if cancelled { 0 } else { remove_missing(pool, library_id, &seen).await.unwrap_or(0) };
    // Compilations éclatées par le serveur : réunies comme en local.
    if !cancelled {
        if let Err(e) = super::album_regroup::regroup(pool, library_id).await {
            log::warn!("💿 Albums distants non réunis : {e}");
        }
    }
    if !cancelled {
        match feedback_service::apply_server_state(pool, server_id, &songs, server.has_ratings()).await {
            Ok((likes, ratings)) if likes + ratings > 0 => log::info!("☁️  Depuis le serveur : {likes} favori(s), {ratings} note(s)"),
            Ok(_) => {}
            Err(e) => log::warn!("☁️  Favoris et notes du serveur non appliqués : {e}"),
        }
        match playlist_service::apply_server_playlists(pool, server_id, &server).await {
            Ok(n) => log::info!("☁️  {n} playlist(s) du serveur à jour"),
            Err(e) => log::warn!("☁️  Playlists du serveur non synchronisées : {e}"),
        }
        let _ = ServerRepository::set_synced(pool, server_id).await;
    }
    log::info!("☁️  {} synchronisée en {:.1} s : {written} écrits, {removed} retirés", library.name, start.elapsed().as_secs_f64());
    Ok((total, cancelled))
}

async fn remote_folder(pool: &SqlitePool, library_id: i64) -> Option<String> {
    sqlx::query_scalar("SELECT remote_folder_id FROM library WHERE id = ?").bind(library_id).fetch_optional(pool).await.ok().flatten().flatten()
}

async fn fetch_cover(server: &crate::core::remote::Backend, id: &str, covers_dir: &Path) -> Option<String> {
    let bytes = server.cover_art(id, 600).await?;
    let dir = covers_dir.to_path_buf();
    tokio::task::spawn_blocking(move || thumbnail_saver(&dir, &bytes, false).ok()).await.ok().flatten()
}

async fn remove_missing(pool: &SqlitePool, library_id: i64, seen: &HashSet<String>) -> Result<u64, sqlx::Error> {
    let known: Vec<(String, String)> = sqlx::query_as("SELECT id, path FROM library_files WHERE library_id = ? AND path LIKE ? || '%'")
        .bind(library_id)
        .bind(crate::core::remote::SCHEME)
        .fetch_all(pool)
        .await?;
    let gone: Vec<(String, String)> = known.into_iter().filter(|(_, p)| !seen.contains(p)).collect();
    let mut tx = pool.begin().await?;
    for (id, _) in &gone {
        sqlx::query("DELETE FROM library_files WHERE id = ?").bind(id).execute(&mut *tx).await?;
    }
    let paths: Vec<String> = gone.iter().map(|(_, p)| p.clone()).collect();
    crate::repository::server::server_repository::purge_paths(&mut tx, &paths).await?;
    if !gone.is_empty() {
        sqlx::query(
            "DELETE FROM library_albums WHERE library_id = ?1
               AND id NOT IN (SELECT library_album_id FROM library_tracks WHERE library_id = ?1 AND library_album_id IS NOT NULL)",
        )
        .bind(library_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(gone.len() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_group_songs_by_album() {
        let song = RemoteSong { id: "s1".into(), album_id: Some("al9".into()), ..Default::default() };
        let path = song_path(3, &song);
        assert_eq!(path, "remote://3/al9/s1");
        // Même album, même « dossier » : le regroupement d'albums du scan s'applique.
        assert_eq!(crate::service::library::album_folder::album_root(&path), "remote://3/al9");
    }

    #[test]
    fn a_song_becomes_an_analysed_file() {
        let song = RemoteSong {
            id: "s1".into(), title: "Get Lucky".into(), artist: Some("Daft Punk".into()), suffix: Some("flac".into()),
            sampling_rate: Some(88200), bit_depth: Some(24), size: Some(10), server_path: Some("DP/RAM/08 Get Lucky.flac".into()),
            ..Default::default()
        };
        let a = as_analysis("remote://1/_/s1".into(), &song, None);
        assert_eq!(a.filename, "08 Get Lucky.flac");
        assert!(matches!(a.audio_file.audio_format, AudioFormat::FLAC));
        assert_eq!((a.audio_file.sample_rate, a.audio_file.bits_per_sample), (88200, 24));
        assert_eq!(a.file_modified, Some(song.fingerprint()));
    }

    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_demo_sync_into_memory() {
        use crate::core::subsonic::client::{Auth, Server, ServerInfo};
        use crate::entity::library::library::LibraryCreate;
        use sqlx::sqlite::SqlitePoolOptions;
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        let (server, info) = Server::connect("demo.navidrome.org", Auth::from_password("demo", "demo")).await.unwrap();
        let folder = server.music_folders().await.unwrap().remove(0);
        let sid = ServerRepository::upsert(&pool, 1, "Démo", "demo", &crate::core::remote::Backend::Subsonic(server.clone()), &info).await.unwrap();
        let lib = LibraryRepository::insert_library(&pool, &LibraryCreate { profil_id: 1, name: "Démo".into(), description: None }).await.unwrap();
        ServerRepository::attach_library(&pool, lib.id, sid, &folder.id).await.unwrap();
        let _ = ServerInfo::default();

        let covers = std::env::temp_dir().join("rustmusic-test-covers");
        static TASK: crate::helper::task::Task = crate::helper::task::Task::new();
        let token = TASK.start().unwrap();
        let mut writes = 0;
        let (total, cancelled) = run(&pool, &covers, lib.id, &token, |s| if let Step::Progress { written, .. } = s { writes = written }).await.unwrap();
        let lib2 = LibraryRepository::find_library_by_id(&pool, lib.id).await.unwrap();
        println!("{total} morceaux, {writes} écrits ; bibliothèque : {} titres, {} albums, {} artistes", lib2.total_tracks, lib2.total_albums, lib2.total_artists);
        assert!(!cancelled && total > 0);
        assert_eq!(lib2.total_tracks as usize, total);
        // Rien n'a changé : la resynchronisation n'écrit rien.
        let mut rewrites = usize::MAX;
        run(&pool, &covers, lib.id, &token, |s| if let Step::Progress { written, .. } = s { rewrites = written }).await.unwrap();
        assert_eq!(rewrites, 0);
    }

    // Essai réel Jellyfin : connexion, enregistrement, synchro, puis un morceau téléchargé en entier.
    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_jellyfin_demo_sync_and_stream() {
        use crate::core::remote::{Backend, Kind};
        use crate::entity::library::library::LibraryCreate;
        use crate::service::external_server::track_service::{prepare, Wait};
        use sqlx::sqlite::SqlitePoolOptions;
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        let (server, info) = Backend::connect(Kind::Jellyfin, "demo.jellyfin.org/stable", "demo", Some(""), None).await.unwrap();
        let folder = server.music_folders().await.unwrap().remove(0);
        let sid = ServerRepository::upsert(&pool, 1, "Démo", "demo", &server, &info).await.unwrap();
        assert!(matches!(ServerRepository::server(&pool, sid).await.unwrap(), Backend::Jellyfin(_)));
        let lib = LibraryRepository::insert_library(&pool, &LibraryCreate { profil_id: 1, name: "Démo".into(), description: None }).await.unwrap();
        ServerRepository::attach_library(&pool, lib.id, sid, &folder.id).await.unwrap();

        let covers = std::env::temp_dir().join("rustmusic-test-covers");
        static TASK: crate::helper::task::Task = crate::helper::task::Task::new();
        let token = TASK.start().unwrap();
        let (total, _) = run(&pool, &covers, lib.id, &token, |_| {}).await.unwrap();
        assert!(total > 0);
        let path: String = sqlx::query_scalar("SELECT path FROM library_files WHERE library_id = ? LIMIT 1").bind(lib.id).fetch_one(&pool).await.unwrap();
        prepare(&pool, Path::new(&path), Wait::Whole).await.unwrap();
        let (_, ext) = crate::core::subsonic::stream::open(Path::new(&path)).unwrap().unwrap();
        println!("{total} morceaux ; {path} prêt ({ext:?})");
        // Transcodé par le serveur : un autre morceau, en MP3 192.
        ServerRepository::set_transcode(&pool, sid, Some(("mp3", 192))).await.unwrap();
        let other: String = sqlx::query_scalar("SELECT path FROM library_files WHERE library_id = ? AND path <> ? LIMIT 1").bind(lib.id).bind(&path).fetch_one(&pool).await.unwrap();
        prepare(&pool, Path::new(&other), Wait::Whole).await.unwrap();
        let (_, ext) = crate::core::subsonic::stream::open(Path::new(&other)).unwrap().unwrap();
        assert_eq!(ext.as_deref(), Some("mp3"));
    }

    // Essai réel DLNA : le MiniDLNA d'essai (`J:\docker\minidlna`), premier dossier de « Browse Folders ».
    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_minidlna_sync_and_stream() {
        use crate::core::remote::{Backend, Kind};
        use crate::entity::library::library::LibraryCreate;
        use crate::service::external_server::track_service::{prepare, Wait};
        use sqlx::sqlite::SqlitePoolOptions;
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        let (server, info) = Backend::connect(Kind::Dlna, "localhost:8201", "", None, None).await.unwrap();
        let Backend::Dlna(dlna) = &server else { unreachable!() };
        let folder = dlna.folders("64").await.unwrap().remove(0);
        let sid = ServerRepository::upsert(&pool, 1, "MiniDLNA", "", &server, &info).await.unwrap();
        // Transcodage demandé : ignoré pour un serveur DLNA.
        ServerRepository::set_transcode(&pool, sid, Some(("mp3", 192))).await.unwrap();
        let lib = LibraryRepository::insert_library(&pool, &LibraryCreate { profil_id: 1, name: folder.name.clone(), description: None }).await.unwrap();
        ServerRepository::attach_library(&pool, lib.id, sid, &folder.id).await.unwrap();

        let covers = std::env::temp_dir().join("rustmusic-test-covers");
        static TASK: crate::helper::task::Task = crate::helper::task::Task::new();
        let token = TASK.start().unwrap();
        let t0 = std::time::Instant::now();
        let (total, _) = run(&pool, &covers, lib.id, &token, |_| {}).await.unwrap();
        let lib2 = LibraryRepository::find_library_by_id(&pool, lib.id).await.unwrap();
        println!("{} : {total} morceaux en {:?} ; {} albums, {} artistes", folder.name, t0.elapsed(), lib2.total_albums, lib2.total_artists);
        assert!(total > 0);
        let path: String = sqlx::query_scalar("SELECT path FROM library_files WHERE library_id = ? AND lower(extension) <> 'dsf' LIMIT 1").bind(lib.id).fetch_one(&pool).await.unwrap();
        prepare(&pool, Path::new(&path), Wait::Start).await.unwrap();
        let (_, ext) = crate::core::subsonic::stream::open(Path::new(&path)).unwrap().unwrap();
        println!("{path} prêt ({ext:?})");
        assert_ne!(ext.as_deref(), Some("mp3"), "fichier d'origine, pas de transcodage");
    }
}
