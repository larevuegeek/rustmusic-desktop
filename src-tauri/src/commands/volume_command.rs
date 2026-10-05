use cpal::traits::{DeviceTrait, HostTrait};

use crate::commands::player_command::AUDIO_PLAYER;
use crate::core::settings_manager::settings_manager::SettingsManager;

#[cfg(target_os = "linux")]
fn is_alsa_virtual_device(name: &str) -> bool {
    const VIRTUAL_PREFIXES: &[&str] = &[
        "surround", "front", "rear", "side", "center_lfe",
        "iec958", "spdif", "hdmi",
        "dmix", "dsnoop", "plughw", "hw:", "plug:",
        "usbstream", "samplerate", "speexrate",
        "upmix", "vdownmix", "null", "jack", "oss",
    ];

    let lower = name.to_lowercase();
    VIRTUAL_PREFIXES.iter().any(|p| lower.starts_with(p) || lower.contains(&format!(" {}", p)))
}

#[cfg(not(target_os = "linux"))]
fn is_alsa_virtual_device(_name: &str) -> bool {
    false
}

#[tauri::command]
pub async fn set_volume(volume: u8) {
    let vol = volume.min(100);

    if let Some(player_arc) = AUDIO_PLAYER.get() {
        if let Ok(player) = player_arc.lock() {
            player.set_volume(vol);
        }
    }

    // Persister
    if let Ok(mut config) = SettingsManager::load_config() {
        config.volume_initial = vol;
        let _ = SettingsManager::save_config(&config);
    }
}

#[tauri::command]
pub async fn get_volume() -> u8 {
    if let Some(player_arc) = AUDIO_PLAYER.get() {
        if let Ok(player) = player_arc.lock() {
            return player.get_volume();
        }
    }

    // Fallback: lire depuis la config
    if let Ok(config) = SettingsManager::load_config() {
        return config.volume_initial;
    }

    80
}

#[tauri::command]
pub async fn mute() {
    set_volume(0).await;
}

#[tauri::command]
pub async fn get_devices() -> Result<Vec<String>, String> {
    let host: cpal::Host = cpal::default_host();

    let output_devices = host.output_devices()
        .map_err(|e| format!("Failed to get output devices: {}", e))?;

    let mut devices_names: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Le serveur son (hôte PulseAudio) ne liste que de vraies sorties.
    #[cfg(target_os = "linux")]
    let hote_alsa = host.id() == cpal::HostId::Alsa;
    #[cfg(not(target_os = "linux"))]
    let hote_alsa = false;

    for device in output_devices {
        // CPAL 0.18 : `Device::name()` supprimé → on passe par `description()`.
        let desc = match device.description() {
            Ok(d) => d,
            Err(_) => continue,
        };
        let raw_name = desc.name().to_string();

        // Filtre ALSA : tester le nom brut ET le driver (le discriminant
        // virtuel hw:/plughw:/surround/iec958/… est dans le champ `driver`).
        if hote_alsa
            && (is_alsa_virtual_device(&raw_name)
                || desc.driver().map(is_alsa_virtual_device).unwrap_or(false))
        {
            continue;
        }

        let display_name = match (desc.manufacturer(), desc.driver()) {
            (Some(mfr), _) => format!("{} ({})", raw_name, mfr),
            (_, Some(drv)) => format!("{} ({})", raw_name, drv),
            _ => raw_name.clone(),
        };

        if seen.insert(display_name.clone()) {
            devices_names.push(display_name);
        }
    }

    // `device_default` vide veut dire « sortie du système » : on ne le remplit
    // pas avec la première carte venue, qui n'est souvent pas celle qu'on écoute.
    Ok(devices_names)
}

#[tauri::command]
pub async fn set_device(device_name: Option<String>) {
    if let Some(player_arc) = AUDIO_PLAYER.get() {
        if let Ok(player) = player_arc.lock() {
            player.set_device(device_name.clone());
            log::info!("Device sélectionné : {:?}", device_name);
        }
    }

    // Persister la sélection pour la restaurer au prochain démarrage
    // (indispensable au DoP : sinon le device revient à "défaut" et
    // `resolve_hw_id` échoue → plus de DSD natif silencieusement).
    if let Ok(mut config) = SettingsManager::load_config() {
        config.device_default = device_name;
        let _ = SettingsManager::save_config(&config);
    }
}
