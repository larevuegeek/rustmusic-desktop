//! Fiche d'un morceau distant pour le lecteur : tirée de la base, remplie à la synchronisation.

use std::path::Path;
use std::time::Duration;

use sqlx::SqlitePool;

use crate::core::podcast::download;
use crate::core::subsonic::stream;
use crate::repository::server::server_repository::ServerRepository;

use crate::entity::audio::audio_file::AudioFile;
use crate::entity::audio::audio_tags::{AttachedImage, AudioTags};
use crate::helper::library::thumbnail_helper::cover_data_url;
use crate::repository::library::library_cache_repository::LibraryCacheRepository;
use crate::repository::library::library_files_repository::LibraryFilesRepository;
use crate::service::external_server::sync_service::audio_format;

/// Début de flux attendu avant de lancer le décodeur.
const PREBUFFER: u64 = 256 * 1024;

pub fn is_remote(path: &str) -> bool {
    path.starts_with(crate::core::remote::SCHEME)
}

/// `remote://<serveur>/<album>/<morceau>` → (serveur, morceau).
pub(crate) fn parse(path: &str) -> Option<(i64, &str)> {
    let rest = path.strip_prefix(crate::core::remote::SCHEME)?;
    let server = rest.split('/').next()?.parse().ok()?;
    Some((server, rest.rsplit('/').next().filter(|s| !s.is_empty())?))
}

/// Ce que la préparation attend avant de rendre la main.
pub enum Wait {
    /// Rien : le téléchargement prend de l'avance (morceau suivant).
    Nothing,
    /// Le début du flux : la lecture commence pendant le téléchargement.
    Start,
    /// Le fichier entier : un lecteur réseau (UPnP) le reçoit d'un bloc.
    Whole,
}

/// Ce qu'il faut demander au serveur pour un morceau, et le nom de son fichier.
pub(crate) struct StreamRequest {
    pub server_id: i64,
    pub song_id: String,
    /// Transcodage demandé au serveur (format, débit en kbit/s) ; `None` : le fichier d'origine.
    pub transcode: Option<(String, i64)>,
    pub name: String,
    /// DSD d'origine : ses décodeurs lisent un fichier complet.
    pub dsd: bool,
    pub duration: Option<f64>,
    /// Taille estimée d'un flux MP3 transcodé (durée × débit), qui n'annonce pas la sienne.
    pub size_hint: Option<u64>,
}

/// Original par défaut ; transcodé si le serveur est réglé ainsi (fichier distinct).
pub(crate) async fn stream_request(pool: &SqlitePool, logical: &str) -> Result<StreamRequest, String> {
    let (server_id, song_id) = parse(logical).ok_or("chemin distant invalide")?;
    let view = ServerRepository::find_one(pool, server_id).await.map_err(|e| e.to_string())?.ok_or("serveur supprimé")?;
    if !view.enabled {
        return Err("serveur désactivé".into());
    }
    let extension = LibraryFilesRepository::find_extension_by_path(pool, logical).await.map_err(|e| e.to_string())?;
    let (duration, source_kbps) = LibraryCacheRepository::duration_and_bitrate(pool, logical).await.ok().flatten().unwrap_or((None, None));
    let stem = crate::core::tag_pattern::sanitize::component(song_id);
    // DLNA : jamais de transcodage, le fichier tel quel.
    let mut view = view;
    if view.kind == "dlna" {
        view.transcode_format = None;
    }
    let dsd = view.transcode_format.is_none() && matches!(extension.as_deref().map(str::to_ascii_lowercase).as_deref(), Some("dsf" | "dff"));
    let transcode = view.transcode_format.clone().map(|f| (f, view.transcode_bitrate.unwrap_or(192)));
    let name = match &transcode {
        Some((format, bitrate)) => format!("{stem}-{bitrate}.{format}"),
        None => format!("{stem}.{}", extension.as_deref().filter(|e| !e.is_empty()).unwrap_or("mp3").to_ascii_lowercase()),
    };
    // Le serveur ne monte jamais au-dessus du débit d'origine. MP3 seulement : l'Ogg (Opus) fixe
    // sa taille à l'analyse, il ne saute qu'une fois en cache.
    let size_hint = match (duration, view.transcode_bitrate.filter(|_| view.transcode_format.as_deref() == Some("mp3"))) {
        (Some(secs), Some(kbps)) => {
            let kbps = source_kbps.filter(|&s| s > 0).map_or(kbps, |s| s.min(kbps));
            Some((secs * kbps as f64 * 1000.0 / 8.0) as u64)
        }
        _ => None,
    };
    Ok(StreamRequest { server_id, song_id: song_id.to_string(), transcode, name, dsd, duration, size_hint })
}

/// Lance (ou rejoint) le téléchargement du morceau vers `file`.
pub(crate) async fn start_download(
    pool: &SqlitePool,
    req: &StreamRequest,
    file: &Path,
    on_done: impl FnOnce(Result<std::path::PathBuf, String>) + Send + 'static,
) -> Result<std::sync::Arc<download::Download>, String> {
    if let Some(dl) = download::get(file) {
        return Ok(dl);
    }
    let server = ServerRepository::server(pool, req.server_id).await?;
    let url = server.stream_url(&req.song_id, req.transcode.as_ref().map(|(f, k)| (f.as_str(), *k))).await?;
    let dl = download::start(url.as_str(), file.to_path_buf(), req.duration, on_done);
    if let Some(bytes) = req.size_hint {
        dl.set_size_hint(bytes);
    }
    Ok(dl)
}

/// Prépare la lecture d'un morceau distant : gardé hors ligne, pris dans le cache, sinon téléchargé
/// pendant l'écoute.
pub async fn prepare(pool: &SqlitePool, path: &Path, wait: Wait) -> Result<(), String> {
    let Some(logical) = path.to_str().filter(|p| is_remote(p)) else { return Ok(()) };
    // Gardé hors ligne : jouable même serveur éteint.
    if let Some(file) = crate::service::external_server::offline_service::ready_file(pool, logical).await {
        stream::register(path, file);
        return Ok(());
    }
    let req = stream_request(pool, logical).await?;
    let wait = match wait {
        Wait::Start if req.dsd => Wait::Whole,
        other => other,
    };
    let file = stream::cache_dir().join(req.server_id.to_string()).join(&req.name);
    if file.exists() {
        stream::touch(&file);
        stream::register(path, file);
        return Ok(());
    }
    let dl = start_download(pool, &req, &file, |result| {
        if result.is_ok() {
            tauri::async_runtime::spawn_blocking(stream::prune);
        }
    })
    .await?;
    stream::register(path, file);
    let (min, timeout) = match wait {
        Wait::Nothing => return Ok(()),
        Wait::Start => (PREBUFFER, Duration::from_secs(30)),
        Wait::Whole => (u64::MAX, Duration::from_secs(600)),
    };
    tokio::task::spawn_blocking(move || dl.wait_for(min, timeout)).await.map_err(|e| e.to_string())?
}

/// Paroles d'un morceau distant, demandées à son serveur ; `None` s'il n'en a pas.
pub async fn server_lyrics(pool: &SqlitePool, path: &str, artist: &str, title: &str) -> Option<crate::core::subsonic::lyrics::ServerLyrics> {
    let (server_id, song_id) = parse(path)?;
    let view = ServerRepository::find_one(pool, server_id).await.ok()??;
    if !view.enabled {
        return None;
    }
    let structured = view.extensions.split(',').any(|e| e.trim() == "songLyrics");
    let server = ServerRepository::server(pool, server_id).await.ok()?;
    match server.lyrics(song_id, structured, artist, title).await {
        Ok(found) => found,
        Err(e) => {
            log::warn!("☁️  Paroles du serveur indisponibles ({song_id}) : {e}");
            None
        }
    }
}

/// `None` pour un chemin local ou un morceau inconnu de la base.
pub async fn audio_file(pool: &SqlitePool, path: &str) -> Option<AudioFile> {
    if !is_remote(path) {
        return None;
    }
    let cache = LibraryCacheRepository::find_by_path(pool, path).await.ok()??;
    let extension = LibraryFilesRepository::find_extension_by_path(pool, path).await.ok().flatten();

    let mut tags = AudioTags::new();
    tags.title = cache.title;
    tags.artist = cache.artist;
    tags.album = cache.album;
    tags.album_artist = cache.album_artist;
    tags.year = cache.year;
    tags.genre = cache.genre;
    tags.track_number = cache.track_number.and_then(|n| u16::try_from(n).ok());
    tags.disc_number = cache.disc_number.and_then(|n| u16::try_from(n).ok());
    tags.duration = cache.duration.map(|d| d as u32);
    if let Some(src) = cache.thumbnail_path.as_deref().and_then(|c| cover_data_url(Path::new(c))) {
        tags.attached_images.push(AttachedImage::cover(src));
    }

    let positive = |n: Option<i32>| n.unwrap_or(0).max(0) as u32;
    Some(AudioFile {
        path: path.to_string(),
        audio_format: audio_format(extension.as_deref()),
        file_size: cache.file_size.unwrap_or(0).max(0) as u64,
        duration: cache.duration.unwrap_or(0.0) as f32,
        bits_per_sample: positive(cache.bits_per_sample),
        bitrate: positive(cache.bitrate),
        sample_rate: positive(cache.sample_rate),
        channels: positive(cache.channels) as usize,
        track_id: None,
        tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn parses_remote_paths() {
        assert_eq!(parse("remote://3/al9/s1"), Some((3, "s1")));
        assert_eq!(parse("remote://x/al/s1"), None);
        assert_eq!(parse(r"C:\Musique\a.mp3"), None);
    }

    #[tokio::test]
    async fn remote_track_comes_from_the_database() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        let path = "remote://1/al/s1";
        sqlx::query("INSERT INTO library_cache (path, title, artist, duration, bitrate, sample_rate, channels) VALUES (?, 'endurance', 'Invention_', 158.0, 320, 44100, 2)")
            .bind(path)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO profil (id, name) VALUES (9, 'T')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library (id, profil_id, name) VALUES (5, 9, 'L')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library_files (library_id, path, filename, extension) VALUES (5, ?, 'endurance.mp3', 'mp3')").bind(path).execute(&pool).await.unwrap();

        let file = audio_file(&pool, path).await.unwrap();
        assert_eq!(file.tags.title.as_deref(), Some("endurance"));
        assert_eq!(file.tags.artist.as_deref(), Some("Invention_"));
        assert!(matches!(file.audio_format, crate::entity::audio::audio_file::AudioFormat::MP3));
        assert_eq!((file.duration as u32, file.bitrate, file.sample_rate), (158, 320, 44100));
        assert!(audio_file(&pool, r"C:\Musique\a.mp3").await.is_none());
    }

    // Essai réel : un morceau de la démo Navidrome, préparé puis décodé comme par le moteur (saut compris).
    #[test]
    #[ignore = "réseau"]
    fn real_demo_song_plays_while_downloading() {
        use crate::core::subsonic::catalog::parse_song;
        use crate::core::subsonic::client::{Auth, Server};
        use crate::service::external_server::sync_service::song_path;
        use symphonia::core::codecs::audio::AudioDecoderOptions;
        use symphonia::core::formats::probe::Hint;
        use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo, TrackType};
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::units::Time;

        let rt = tokio::runtime::Runtime::new().unwrap();
        let t0 = std::time::Instant::now();
        let (path, duration) = rt.block_on(async {
            let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
            sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
            sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
            let (server, info) = Server::connect("demo.navidrome.org", Auth::from_password("demo", "demo")).await.unwrap();
            let sid = ServerRepository::upsert(&pool, 1, "Démo", "demo", &crate::core::remote::Backend::Subsonic(server.clone()), &info).await.unwrap();
            // `TRANSCODE=mp3-192` : même essai, transcodé par le serveur.
            if let Some(v) = std::env::var("TRANSCODE").ok().filter(|v| !v.is_empty()) {
                let (format, bitrate) = v.split_once('-').unwrap();
                ServerRepository::set_transcode(&pool, sid, Some((format, bitrate.parse().unwrap()))).await.unwrap();
            }
            let random = server.call("getRandomSongs", &[("size", "1")]).await.unwrap();
            let song = parse_song(&random["randomSongs"]["song"][0]).unwrap();
            let path = song_path(sid, &song);
            sqlx::query("INSERT INTO library (id, profil_id, name) VALUES (5, 1, 'L')").execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_files (library_id, path, filename, extension) VALUES (5, ?, 'x', ?)")
                .bind(&path)
                .bind(song.suffix.clone())
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO library_cache (path, duration, bitrate) VALUES (?, ?, ?)")
                .bind(&path)
                .bind(song.duration)
                .bind(song.bit_rate)
                .execute(&pool)
                .await
                .unwrap();
            prepare(&pool, Path::new(&path), Wait::Start).await.unwrap();
            println!("{} — {:?} ({:?}), prêt en {:?}", song.title, song.suffix, song.duration, t0.elapsed());
            (path, song.duration.unwrap_or(60.0))
        });

        let (source, ext) = stream::open(Path::new(&path)).unwrap().unwrap();
        let mut hint = Hint::new();
        if let Some(ext) = ext {
            hint.with_extension(&ext);
        }
        let mss = MediaSourceStream::new(source, Default::default());
        let mut format = symphonia::default::get_probe().probe(&hint, mss, FormatOptions::default(), MetadataOptions::default()).unwrap();
        let track = format.default_track(TrackType::Audio).unwrap().clone();
        let params = track.codec_params.as_ref().unwrap().audio().unwrap().clone();
        let mut decoder = crate::core::audio_player::codecs::codec_registry().make_audio_decoder(&params, &AudioDecoderOptions::default()).unwrap();
        let rate = params.sample_rate.unwrap() as usize;
        let mut frames = 0;
        while frames < rate * 3 {
            let packet = format.next_packet().unwrap().unwrap();
            frames += decoder.decode(&packet).unwrap().frames();
        }
        println!("3 s décodées à {:?}", t0.elapsed());
        let target = duration * 0.7;
        format.seek(SeekMode::Coarse, SeekTo::Time { time: Time::try_from_secs_f64(target).unwrap(), track_id: Some(track.id) }).unwrap();
        decoder.reset();
        let packet = format.next_packet().unwrap().unwrap();
        decoder.decode(&packet).unwrap();
        println!("saut à {target:.0} s décodé à {:?}", t0.elapsed());
    }
}
