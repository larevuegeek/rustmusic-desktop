//! Commandes de la télécommande d'ampli : recherche, choix, envoi, réglage d'activation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::State;

use crate::core::amp_remote::discovery;
use crate::core::amp_remote::model::{Amp, Command};
use crate::core::amp_remote::session::{self, View};
use crate::repository::settings::settings_repository::SettingsRepository;
use crate::state::AppState;

/// Réglage « Télécommande » : désactivé par défaut, aucun ampli n'est contacté.
const ENABLED_KEY: &str = "amp_remote";
/// Ampli choisi, mémorisé tel quel (JSON) : retrouvé au lancement sans recherche.
const DEVICE_KEY: &str = "amp_remote_device";

static ENABLED: AtomicBool = AtomicBool::new(false);

fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Cherche les amplis pilotables (~2 s).
#[tauri::command]
pub async fn amp_remote_discover() -> Vec<Amp> {
    if !enabled() {
        return Vec::new();
    }
    let found = discovery::discover(Duration::from_secs(2)).await;
    log::info!("🎛️  {} ampli(s) pilotable(s) trouvé(s)", found.len());
    found
}

/// Choisit l'ampli à piloter (`None` : aucun) et le mémorise.
#[tauri::command]
pub async fn amp_remote_select(state: State<'_, AppState>, amp: Option<Amp>) -> Result<Option<View>, String> {
    if amp.is_some() && !enabled() {
        return Err("télécommande désactivée dans les réglages".into());
    }
    let json = amp.as_ref().map(|a| serde_json::to_string(a).unwrap_or_default()).unwrap_or_default();
    SettingsRepository::set(&state.pool, DEVICE_KEY, &json).await.map_err(|e| e.to_string())?;
    session::select(amp);
    Ok(session::view())
}

/// L'ampli piloté, ses capacités et son état.
#[tauri::command]
pub fn amp_remote_status() -> Option<View> {
    session::view()
}

/// Pochette de l'écran réseau (adresse `data:`), si l'ampli en a envoyé une.
#[tauri::command]
pub fn amp_remote_cover() -> Option<String> {
    session::cover()
}

#[tauri::command]
pub fn amp_remote_send(command: Command) -> Result<(), String> {
    session::send(command)
}

/// Active ou coupe la télécommande ; l'ampli mémorisé est gardé.
#[tauri::command]
pub async fn set_amp_remote_enabled(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    ENABLED.store(enabled, Ordering::Relaxed);
    if enabled {
        restore(&state).await;
    } else {
        session::select(None);
    }
    Ok(())
}

/// Au lancement : lit le réglage, puis reprend l'ampli mémorisé.
pub async fn init(state: &AppState) {
    // Ampli retrouvé à une autre adresse : la nouvelle est gardée pour le prochain lancement.
    let pool = state.pool.clone();
    session::on_moved(move |amp| {
        let (pool, json) = (pool.clone(), serde_json::to_string(amp).unwrap_or_default());
        tauri::async_runtime::spawn(async move {
            let _ = SettingsRepository::set(&pool, DEVICE_KEY, &json).await;
        });
    });
    let on = SettingsRepository::get(&state.pool, ENABLED_KEY).await.ok().flatten().as_deref() == Some("true");
    ENABLED.store(on, Ordering::Relaxed);
    if on {
        restore(state).await;
    }
}

async fn restore(state: &AppState) {
    let json = SettingsRepository::get(&state.pool, DEVICE_KEY).await.ok().flatten().unwrap_or_default();
    if let Ok(amp) = serde_json::from_str::<Amp>(&json) {
        session::select(Some(amp));
    }
}
