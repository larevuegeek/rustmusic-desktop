//! Tauri commands for the audio quality profile (settings UI).
//!
//! Persists the user's choice in the `settings` table under
//! `audio_quality_profile` (`"auto" | "high" | "medium" | "low"`), and keeps
//! the in-memory resolved profile up to date so player threads pick changes
//! up on the next track.

use serde::Serialize;
use tauri::State;

use crate::core::audio_player::replay_gain::{self, ReplayGainMode};
use crate::core::audio_quality::{
    self, AudioQualityProfile, AudioQualitySetting,
};
use crate::core::system_detect;
use crate::repository::settings::settings_repository::SettingsRepository;
use crate::state::AppState;

const KEY: &str = "audio_quality_profile";
const RG_MODE_KEY: &str = "replay_gain_mode";
const RG_PREAMP_KEY: &str = "replay_gain_preamp";
const GAPLESS_KEY: &str = "gapless";

/// Status payload returned to the frontend. Bundles everything the UI needs
/// in one round-trip.
#[derive(Debug, Serialize)]
pub struct AudioQualityStatus {
    /// User-chosen setting (auto / high / medium / low).
    pub setting: AudioQualitySetting,
    /// Concrete profile currently driving the pipeline.
    pub resolved: AudioQualityProfile,
    /// `Some("kvm" | "vmware" | ...)` when the host is virtualised; `None` on bare metal.
    pub virt_kind: Option<String>,
    /// Logical core count detected on the host.
    pub cpu_cores: usize,
}

fn detect_virt_kind() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        return crate::core::system_detect::detect_linux_virt();
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Read the persisted setting, resolve it, and apply it to the global
/// in-memory profile. Called once at app boot from `run()`.
pub async fn init_from_settings(state: &AppState) {
    let raw = SettingsRepository::get(&state.pool, KEY)
        .await
        .ok()
        .flatten();
    let setting = AudioQualitySetting::parse_or_auto(raw.as_deref());
    let resolved = setting.resolve();
    audio_quality::set_current_profile(resolved);
    log::info!(
        "🎚  Audio quality : setting={:?}, resolved={:?}",
        setting, resolved
    );

    // Replay Gain : les threads audio lisent des atomics, il faut donc pousser
    // les valeurs persistées AVANT la première lecture.
    let (mode, preamp) = read_replay_gain(state).await;
    replay_gain::set_mode(mode);
    replay_gain::set_preamp_db(preamp);
    log::info!(
        "🔊 Replay Gain : mode={:?}, préampli={:+.1} dB",
        mode, preamp
    );

    // Lecture sans blanc : activée par défaut (absence de clé = activé).
    let gapless = SettingsRepository::get(&state.pool, GAPLESS_KEY)
        .await
        .ok()
        .flatten()
        .map(|v| v != "false")
        .unwrap_or(true);
    crate::core::audio_player::preload::set_gapless_enabled(gapless);
}

/// Annonce la piste qui suivra, pour permettre son préchargement.
///
/// Appelée par le frontend chaque fois que « la suite » change : nouvelle
/// lecture, réordonnancement, shuffle, repeat, ajout ou retrait. `None`
/// signifie « rien après » (fin de file, minuterie de fin de morceau).
#[tauri::command]
pub fn set_next_track(path: Option<String>) {
    crate::core::audio_player::preload::announce_next(path.map(std::path::PathBuf::from));
}

/// Synchronise la préférence « lecture sans blanc » vers l'atomique lu par les
/// threads audio. La persistance est assurée par le store côté frontend
/// (clé `gapless`), comme pour la sortie exclusive.
#[tauri::command]
pub fn set_gapless(enabled: bool) {
    crate::core::audio_player::preload::set_gapless_enabled(enabled);
}

/// Lit les réglages Replay Gain persistés (valeurs par défaut si absents).
async fn read_replay_gain(state: &AppState) -> (ReplayGainMode, f32) {
    let raw_mode = SettingsRepository::get(&state.pool, RG_MODE_KEY)
        .await
        .ok()
        .flatten();
    let raw_preamp = SettingsRepository::get(&state.pool, RG_PREAMP_KEY)
        .await
        .ok()
        .flatten();
    (
        ReplayGainMode::parse_or_off(raw_mode.as_deref()),
        raw_preamp
            .and_then(|v| v.parse::<f32>().ok())
            .filter(|v| v.is_finite())
            .unwrap_or(0.0),
    )
}

/// Réglages Replay Gain exposés à l'UI.
#[derive(Debug, Serialize)]
pub struct ReplayGainSettings {
    /// `"off" | "track" | "album"`.
    pub mode: String,
    /// Pré-ampli en dB, borné à ±15.
    pub preamp_db: f32,
}

#[tauri::command]
pub async fn get_replay_gain_settings(
    state: State<'_, AppState>,
) -> Result<ReplayGainSettings, String> {
    let (mode, preamp) = read_replay_gain(&state).await;
    Ok(ReplayGainSettings {
        mode: mode.as_str().to_string(),
        preamp_db: preamp,
    })
}

/// Applique et persiste les réglages. Effet immédiat sur le **prochain**
/// morceau : le facteur du morceau en cours n'est pas recalculé, pour éviter
/// un saut de volume en pleine écoute.
#[tauri::command]
pub async fn set_replay_gain_settings(
    state: State<'_, AppState>,
    mode: String,
    preamp_db: f32,
) -> Result<ReplayGainSettings, String> {
    let parsed = ReplayGainMode::parse_or_off(Some(&mode));
    let preamp = if preamp_db.is_finite() {
        preamp_db.clamp(-15.0, 15.0)
    } else {
        0.0
    };

    SettingsRepository::set(&state.pool, RG_MODE_KEY, parsed.as_str())
        .await
        .map_err(|e| format!("save {RG_MODE_KEY}: {e}"))?;
    SettingsRepository::set(&state.pool, RG_PREAMP_KEY, &preamp.to_string())
        .await
        .map_err(|e| format!("save {RG_PREAMP_KEY}: {e}"))?;

    replay_gain::set_mode(parsed);
    replay_gain::set_preamp_db(preamp);
    log::info!(
        "🔊 Replay Gain updated : mode={:?}, préampli={:+.1} dB",
        parsed, preamp
    );

    Ok(ReplayGainSettings {
        mode: parsed.as_str().to_string(),
        preamp_db: preamp,
    })
}

#[tauri::command]
pub async fn get_audio_quality_status(
    state: State<'_, AppState>,
) -> Result<AudioQualityStatus, String> {
    let raw = SettingsRepository::get(&state.pool, KEY)
        .await
        .map_err(|e| format!("read {KEY}: {e}"))?;
    let setting = AudioQualitySetting::parse_or_auto(raw.as_deref());
    let resolved = setting.resolve();
    Ok(AudioQualityStatus {
        setting,
        resolved,
        virt_kind: detect_virt_kind(),
        cpu_cores: system_detect::logical_cpu_count(),
    })
}

#[tauri::command]
pub async fn set_audio_quality_setting(
    state: State<'_, AppState>,
    value: String,
) -> Result<AudioQualityStatus, String> {
    let setting = AudioQualitySetting::parse_or_auto(Some(&value));
    SettingsRepository::set(&state.pool, KEY, setting.as_str())
        .await
        .map_err(|e| format!("save {KEY}: {e}"))?;

    let resolved = setting.resolve();
    audio_quality::set_current_profile(resolved);
    log::info!(
        "🎚  Audio quality updated : setting={:?}, resolved={:?}",
        setting, resolved
    );

    Ok(AudioQualityStatus {
        setting,
        resolved,
        virt_kind: detect_virt_kind(),
        cpu_cores: system_detect::logical_cpu_count(),
    })
}
