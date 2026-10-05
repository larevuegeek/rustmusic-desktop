use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use tauri::AppHandle;

use crate::core::audio_analyser::audio_analyser::AudioAnalyser;
use crate::core::audio_player::audio_player::AudioPlayer;
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
pub async fn play_file(
    app_handle: AppHandle,
    path: String
) {
    
    let path_file: PathBuf = PathBuf::from(path);

    log::debug!("File selected {:?}", path_file);

    if let Some(player_arc) = AUDIO_PLAYER.get() {
        let player: MutexGuard<'_, AudioPlayer> = player_arc.lock().unwrap_or_else(|e| e.into_inner());

        if player.is_playing() {
            if player.is_paused() {
                player.resume();
            } else {
                player.pause();
            }
        } else {
            if let Err(e) = player.play_file(app_handle, path_file) {
                log::error!("Erreur de lecture: {}", e);
            }
        }
    } else {
        log::error!("⚠️ AudioPlayer non initialisé !");
    }
}

/// Empêche la mise en veille automatique tant que la musique joue (l'écran peut s'éteindre).
#[tauri::command]
pub fn set_veille_bloquee(bloquer: bool) {
    crate::core::veille::bloquer(bloquer);
}

#[tauri::command]
pub async fn pause_play() {
    log::debug!("Demande de pause");

    if let Some(player_arc) = AUDIO_PLAYER.get() {
        let player: MutexGuard<'_, AudioPlayer> = player_arc.lock().unwrap_or_else(|e| e.into_inner());

        // Pas besoin de relâcher le lock, on ne bloque pas le stream ici
        if player.is_paused() {
            player.resume();
        } else {
            player.pause();
        }
    } else {
        log::error!("⚠️ AudioPlayer non initialisé !");
    }
}

#[tauri::command]
pub async fn stop_play() {
    if let Some(player_arc) = AUDIO_PLAYER.get() {
        let player: MutexGuard<'_, AudioPlayer> = player_arc.lock().unwrap_or_else(|e| e.into_inner());

        // Pas besoin de relâcher le lock, on ne bloque pas le stream ici
        player.stop();
    } else {
        log::error!("⚠️ AudioPlayer non initialisé !");
    }
}

#[tauri::command]
pub async fn get_progress() -> (f64, f64) {
    if let Some(player_arc) = AUDIO_PLAYER.get() {
        let player: MutexGuard<'_, AudioPlayer> = player_arc.lock().unwrap_or_else(|e| e.into_inner());

        let current: f64 = player.get_current_position();
        let total: f64 = player.get_total_duration();
        return (current, total);
    }

    (0.0, 0.0)
}

#[tauri::command]
pub async fn seek_to(position: f64) {
    if let Some(player_arc) = AUDIO_PLAYER.get() {
        let player = player_arc.lock().unwrap_or_else(|e| e.into_inner());
        // Demande au decode_thread de seek dans le fichier audio
        // Met aussi à jour le compteur de position pour le frontend
        player.seek_to(position);
    }
}