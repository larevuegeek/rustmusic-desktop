//! Aiguillage de la lecture : le lecteur de RustMusic (carte son ou AirPlay) ou un
//! lecteur réseau UPnP qui lit lui-même les fichiers. Chaque action traite les deux
//! cas par un `match` : une nouvelle cible ne compile pas tant qu'elle n'est pas gérée.

use std::path::PathBuf;

use tauri::{AppHandle, Emitter};

use crate::commands::player_command::AUDIO_PLAYER;
use crate::core::audio_player::audio_player::AudioPlayer;
use crate::core::network_player::airplay::sender as airplay;
use crate::core::network_player::session;
use crate::core::settings_manager::settings_manager::SettingsManager;

/// Qui joue.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    /// Le lecteur de RustMusic ; AirPlay n'en est qu'une sortie.
    Local,
    /// Un lecteur UPnP/DLNA choisi comme sortie.
    Upnp,
}

pub fn target() -> Target {
    if session::is_active() { Target::Upnp } else { Target::Local }
}

/// Le lecteur local, s'il est prêt.
fn local<R>(f: impl FnOnce(&AudioPlayer) -> R) -> Option<R> {
    let player = AUDIO_PLAYER.get()?.lock().unwrap_or_else(|e| e.into_inner());
    Some(f(&player))
}

/// Lance `path` ; sur le lecteur local, un morceau en cours bascule lecture/pause.
pub async fn play(app: AppHandle, path: PathBuf) {
    // Une seule source : un morceau ou un épisode coupe la radio.
    if !crate::core::radio::live::is_live_path(&path) {
        crate::core::radio::live::stop();
    }
    match target() {
        Target::Upnp => {
            // Une seule sortie joue : le lecteur local se tait.
            stop_local();
            if let Err(e) = session::play(app.clone(), path).await {
                log::error!("📡 Lecture réseau impossible : {e}");
                let _ = app.emit("network-player-error", e);
            }
        }
        Target::Local => {
            let done = local(|p| {
                if p.is_playing() {
                    if p.is_paused() { p.resume() } else { p.pause() }
                } else if let Err(e) = p.play_file(app, path) {
                    log::error!("Erreur de lecture: {}", e);
                }
            });
            if done.is_none() {
                log::error!("⚠️ AudioPlayer non initialisé !");
            }
        }
    }
}

pub async fn toggle_pause() {
    match target() {
        Target::Upnp => {
            if let Err(e) = session::toggle_pause().await {
                log::warn!("📡 Pause réseau : {e}");
            }
        }
        Target::Local => {
            local(|p| {
                if p.is_paused() { p.resume() } else { p.pause() }
                // Radio : la pause fait le différé.
                crate::core::radio::live::set_paused(p.is_paused());
            });
        }
    }
}

pub async fn stop() {
    // La radio d'abord : son lecteur attend peut-être des données, il doit lâcher tout de suite.
    crate::core::radio::live::stop();
    match target() {
        Target::Upnp => {
            if let Err(e) = session::stop().await {
                log::debug!("📡 Arrêt réseau : {e}");
            }
        }
        Target::Local => {
            local(|p| p.stop());
        }
    }
}

/// Arrête le lecteur local seulement (une sortie réseau prend le relais).
pub fn stop_local() {
    local(|p| {
        if p.is_playing() {
            p.stop();
        }
    });
}

/// Arrête ce qui joue, avant de changer de sortie.
pub async fn stop_all() {
    stop_local();
    match target() {
        Target::Upnp => {
            let _ = session::stop().await;
        }
        Target::Local => {}
    }
}

/// (position, durée) en secondes.
pub fn progress() -> (f64, f64) {
    match target() {
        Target::Upnp => session::progress(),
        Target::Local => local(|p| {
            let mut current = p.get_current_position();
            // AirPlay : l'appareil joue ~2 s après l'envoi, on affiche ce qu'on entend.
            if airplay::is_active() {
                current = (current - airplay::lag_secs()).max(0.0);
            }
            (current, p.get_total_duration())
        })
        .unwrap_or((0.0, 0.0)),
    }
}

pub async fn seek(position: f64) {
    match target() {
        Target::Upnp => {
            if let Err(e) = session::seek(position).await {
                log::warn!("📡 Seek réseau : {e}");
            }
        }
        Target::Local => {
            local(|p| p.seek_to(position));
        }
    }
}

/// Volume 0–100 ; celui du lecteur local est aussi mémorisé pour le prochain lancement.
pub async fn set_volume(volume: u8) {
    let volume = volume.min(100);
    match target() {
        // Lecteur réseau : son volume à lui ; celui du lecteur local reste tel quel.
        Target::Upnp => {
            if let Err(e) = session::set_volume(volume).await {
                log::warn!("📡 Volume réseau : {e}");
            }
        }
        Target::Local => {
            local(|p| p.set_volume(volume));
            if let Ok(mut config) = SettingsManager::load_config() {
                config.volume_initial = volume;
                let _ = SettingsManager::save_config(&config);
            }
        }
    }
}

/// Volume affiché ; sans réponse du lecteur réseau, celui du lecteur local.
pub async fn volume() -> u8 {
    let remote = match target() {
        Target::Upnp => session::volume().await,
        Target::Local => None,
    };
    remote
        .or_else(|| local(|p| p.get_volume()))
        .or_else(|| SettingsManager::load_config().ok().map(|c| c.volume_initial))
        .unwrap_or(80)
}

/// Piste qui suivra (`None` : rien après), pour l'enchaînement sans blanc.
pub fn set_next(path: Option<PathBuf>) {
    match target() {
        Target::Upnp => {
            tauri::async_runtime::spawn(session::set_next(path));
        }
        Target::Local => crate::core::audio_player::preload::announce_next(path),
    }
}
