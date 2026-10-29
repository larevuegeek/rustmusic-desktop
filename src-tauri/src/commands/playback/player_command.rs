use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use tauri::{AppHandle, Emitter, Manager};

use crate::core::audio_analyser::audio_analyser::AudioAnalyser;
use crate::core::audio_player::audio_player::AudioPlayer;
use crate::commands::playback::playback;
use crate::entity::audio::audio_file::AudioFile;
use crate::helper::files::reader::read_dir_deep;
use crate::service::podcast::podcast_service;
use crate::service::external_server::track_service;
use crate::state::AppState;

pub static AUDIO_PLAYER: OnceLock<Arc<Mutex<AudioPlayer>>> = OnceLock::new();
/// Dernière demande de lecture : une préparation d'épisode dépassée ne lance plus rien.
static PLAY_REQUEST: AtomicU64 = AtomicU64::new(0);

#[tauri::command]
pub async fn open_file(app: AppHandle, path: String) -> AudioFile {
    log::debug!("Fichier sélectionné : {}", path);

    let pool = &app.state::<AppState>().pool;
    // Morceau d'un serveur : sa fiche vient de la synchronisation.
    if let Some(file) = track_service::audio_file(pool, &path).await {
        return file;
    }

    let path_file: PathBuf = PathBuf::from(path);

    // Épisode de podcast : sa fiche vient de la base, le fichier n'est peut-être pas encore là.
    if let Some(file) = podcast_service::audio_file(pool, &path_file).await {
        return file;
    }

    let result_audio_file: Result<AudioFile, Box<dyn Error>> =
        AudioAnalyser::analyse_audio_file(&path_file);

    match result_audio_file {
        Ok(audio_file) => {
            log::debug!("✅ Analyse réussie : {}", audio_file.path);
            audio_file
        }
        Err(e) => {
            log::error!("❌ Erreur d'analyse : {}", e);
            AudioFile::default()
        }
    }
}

#[tauri::command]
pub async fn open_files(directory: String) -> Vec<AudioFile> {
    
    let mut files: Vec<PathBuf> = Vec::new();
    let mut audio_files: Vec<AudioFile> = Vec::new();

    let _ = read_dir_deep(&directory, &mut files);

    for file in files {

        let result_audio_file: Result<AudioFile, Box<dyn Error>> =
            AudioAnalyser::analyse_audio_file(&file);

        match result_audio_file {
            Ok(audio_file) => {
                log::debug!("✅ Analyse réussie : {}", audio_file.path);
                audio_files.push(audio_file)
            }
            Err(e) => {
                log::error!("❌ Erreur d'analyse : {}", e);
            }
        }

    }

    // Ordre de l'album : dossier, disque, piste. `read_dir` n'a pas d'ordre garanti (APFS).
    audio_files.sort_by(|a, b| {
        let key = |f: &AudioFile| {
            let parent = std::path::Path::new(&f.path).parent().map(|p| p.to_path_buf());
            (parent, f.tags.disc_number.unwrap_or(1), f.tags.track_number.unwrap_or(u16::MAX), f.path.clone())
        };
        key(a).cmp(&key(b))
    });

    audio_files
}

#[tauri::command]
pub async fn play_file(app_handle: AppHandle, path: String) {
    let path_file = PathBuf::from(path);
    log::debug!("File selected {:?}", path_file);
    let request = PLAY_REQUEST.fetch_add(1, Ordering::SeqCst) + 1;
    // Épisode : téléchargement lancé d'abord ; un lecteur réseau attend le fichier complet.
    let whole = playback::target() == playback::Target::Upnp;
    let pool = app_handle.state::<AppState>().pool.clone();
    if let Err(e) = podcast_service::prepare(&pool, &path_file, whole).await {
        log::warn!("🎙️  Épisode illisible : {e}");
        let _ = app_handle.emit("playback-preparing", false);
        let _ = app_handle.emit("podcast-error", e);
        return;
    }
    // Morceau d'un serveur : pris dans le cache ou téléchargé pendant l'écoute.
    crate::core::subsonic::stream::play_only(&path_file);
    let wait = if whole { track_service::Wait::Whole } else { track_service::Wait::Start };
    if let Err(e) = track_service::prepare(&pool, &path_file, wait).await {
        // Un autre morceau a été lancé entre-temps : son échec (souvent une annulation) ne compte plus.
        if PLAY_REQUEST.load(Ordering::SeqCst) != request {
            return;
        }
        log::warn!("☁️  Morceau distant illisible : {e}");
        let _ = app_handle.emit("playback-preparing", false);
        let _ = app_handle.emit("server-play-error", e);
        return;
    }
    if PLAY_REQUEST.load(Ordering::SeqCst) != request {
        return;
    }
    // Morceau d'un serveur : il apprend ce qui joue (« en cours » de ses autres clients).
    let state = app_handle.state::<AppState>();
    crate::commands::playlist::playlist_command::send_feedback(&state, path_file.to_string_lossy().into_owned(), crate::service::external_server::feedback_service::Feedback::NowPlaying);
    // Vitesse : celle de l'émission pour un épisode (en sortie partagée), normale sinon ou vers un lecteur réseau.
    let speed = if whole { None } else { podcast_service::speed_for(&pool, &path_file).await };
    crate::core::audio_player::output::set_shared_only(speed.is_some());
    crate::core::audio_player::time_stretch::set_speed(speed.unwrap_or(1.0));
    playback::play(app_handle, path_file).await;
}

/// Empêche la mise en veille automatique tant que la musique joue (l'écran peut s'éteindre).
#[tauri::command]
pub fn set_sleep_blocked(bloquer: bool) {
    crate::core::system_sleep::set_blocked(bloquer);
}

#[tauri::command]
pub async fn pause_play() {
    log::debug!("Demande de pause");
    playback::toggle_pause().await;
}

#[tauri::command]
pub async fn stop_play() {
    playback::stop().await;
}

#[tauri::command]
pub async fn get_progress() -> (f64, f64) {
    playback::progress()
}

#[tauri::command]
pub async fn seek_to(position: f64) {
    playback::seek(position).await;
}
