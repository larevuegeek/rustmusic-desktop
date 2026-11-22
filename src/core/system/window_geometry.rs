//! Taille, position et état agrandi de la fenêtre principale, écrits au fil des
//! changements : l'application finit souvent sans sortie propre (icône de la zone de
//! notification, plantage). Le mini-lecteur ne les écrase jamais.
//!
//! La couche d'interface signale chaque géométrie par `report` ; l'écriture est
//! différée de 800 ms. Au démarrage, `load` rend la dernière géométrie connue.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Géométrie de la fenêtre en unités logiques de l'écran.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
    /// Plein écran (absent des fichiers écrits avant la 0.5.0-beta.6).
    #[serde(default)]
    pub fullscreen: bool,
}

static COMPACT: AtomicBool = AtomicBool::new(false);
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// Dernière géométrie signalée par la fenêtre.
static CURRENT: Mutex<Option<Geometry>> = Mutex::new(None);
/// Géométrie normale au moment de passer en mode compact, rétablie à la sortie.
static NORMAL_SNAPSHOT: Mutex<Option<Geometry>> = Mutex::new(None);

fn state_file() -> PathBuf {
    crate::app::platform::paths::app_config_dir().join("window-state.json")
}

/// La géométrie enregistrée, s'il y en a une.
pub fn load() -> Option<Geometry> {
    let text = std::fs::read_to_string(state_file()).ok()?;
    serde_json::from_str(&text).ok()
}

/// La fenêtre a bougé ou changé de taille : mémorisée, écrite après un court répit.
pub fn report(geometry: Geometry) {
    *CURRENT.lock().unwrap_or_else(|e| e.into_inner()) = Some(geometry);
    schedule_save();
}

fn write(geometry: Geometry) {
    let file = state_file();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match serde_json::to_string(&geometry) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&file, json) {
                log::warn!("État de la fenêtre non sauvegardé ({}) : {e}", file.display());
            }
        }
        Err(e) => log::warn!("État de la fenêtre non sérialisé : {e}"),
    }
}

/// Écrit tout de suite la dernière géométrie connue (fermeture de la fenêtre).
pub fn save_now() {
    if COMPACT.load(Ordering::Acquire) {
        return;
    }
    if let Some(geometry) = *CURRENT.lock().unwrap_or_else(|e| e.into_inner()) {
        write(geometry);
    }
}

/// Écrit une fois la fenêtre immobile depuis un moment.
pub fn schedule_save() {
    if COMPACT.load(Ordering::Acquire) {
        return;
    }
    let generation = GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    crate::app::platform::runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(800)).await;
        if GENERATION.load(Ordering::Acquire) == generation {
            save_now();
        }
    });
}

/// Mini ou micro-lecteur : la géométrie normale n'est plus réécrite.
pub fn set_compact(active: bool) {
    if active {
        save_now();
        *NORMAL_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()) =
            *CURRENT.lock().unwrap_or_else(|e| e.into_inner());
        COMPACT.store(true, Ordering::Release);
    } else {
        COMPACT.store(false, Ordering::Release);
        schedule_save();
    }
}

/// À la sortie : en mode compact, la géométrie normale est remise en place.
pub fn on_exit() {
    if !COMPACT.load(Ordering::Acquire) {
        save_now();
        return;
    }
    let snapshot = NORMAL_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(geometry) = snapshot {
        write(geometry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_geometry_round_trips_through_json() {
        let g = Geometry { x: 10, y: 20, width: 1280, height: 900, maximized: false, fullscreen: true };
        let json = serde_json::to_string(&g).unwrap();
        assert_eq!(serde_json::from_str::<Geometry>(&json).unwrap(), g);
    }

    #[test]
    fn an_older_file_without_fullscreen_still_loads() {
        let g: Geometry = serde_json::from_str(r#"{"x":1,"y":2,"width":1280,"height":900,"maximized":true}"#).unwrap();
        assert!(g.maximized && !g.fullscreen);
    }
}
