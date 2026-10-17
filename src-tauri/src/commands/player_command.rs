use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use tauri::AppHandle;

use crate::core::audio_analyser::audio_analyser::AudioAnalyser;
use crate::core::audio_player::audio_player::AudioPlayer;
use crate::commands::playback;
use crate::entity::audio::audio_file::AudioFile;
use crate::helper::files::reader::read_dir_deep;

pub static AUDIO_PLAYER: OnceLock<Arc<Mutex<AudioPlayer>>> = OnceLock::new();

#[tauri::command]
pub async fn open_file(path: String) -> AudioFile {
    log::debug!("Fichier sélectionné : {}", path);

    let path_file: PathBuf = PathBuf::from(path);

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
