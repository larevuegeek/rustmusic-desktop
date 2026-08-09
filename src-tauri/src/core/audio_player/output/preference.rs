//! Préférence runtime du backend audio.
//!
//! Stockée dans un atomic global pour être lue rapidement par
//! `play_file_thread` sans accès BDD (le thread audio n'a pas accès au pool
//! tokio). La valeur est synchronisée avec la BDD au boot
//! (`init_from_settings`) et à chaque toggle utilisateur via la Tauri
//! command `set_wasapi_exclusive_preference`.

use std::sync::atomic::{AtomicBool, Ordering};

use super::types::AudioBackend;

static WASAPI_EXCLUSIVE: AtomicBool = AtomicBool::new(false);

/// Préférence « DSD natif (DoP) ». Quand activée ET que WASAPI exclusive est
/// actif ET que le DAC accepte le format porteur, les fichiers DSD sont
/// envoyés en DoP (DSD over PCM) au lieu d'être convertis en PCM.
static DSD_DOP: AtomicBool = AtomicBool::new(false);

/// `true` si l'utilisateur a activé le DoP. La décision finale (DoP vs
/// DSD2PCM) dépend AUSSI de WASAPI exclusive actif + compat DAC, vérifiée
/// au moment de la lecture.
pub fn dop_enabled() -> bool {
    // Windows : DoP via WASAPI exclusive. Linux : DoP via ALSA hw exclusif.
    // macOS : DoP via CoreAudio + hog mode (seule voie DSD native du système).
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        return DSD_DOP.load(Ordering::Relaxed);
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        false
    }
}

/// Met à jour la préférence DoP (effet au prochain morceau DSD lancé).
pub fn set_dop_enabled(enabled: bool) {
    DSD_DOP.store(enabled, Ordering::Relaxed);
    log::info!("🎚️  DSD natif (DoP) preference : {}", if enabled { "ON" } else { "OFF" });
}

/// Backend préféré par l'utilisateur : le backend exclusif de la plateforme
/// quand la sortie exclusive est activée, `CpalShared` sinon.
///
/// Le réglage est unique et partagé par les trois OS (clé BDD historique
/// `wasapi_exclusive`, conservée pour ne pas perdre le choix des utilisateurs
/// Windows existants) ; c'est la plateforme qui décide du backend concret.
pub fn current_preference() -> AudioBackend {
    if WASAPI_EXCLUSIVE.load(Ordering::Relaxed) {
        if let Some(exclusive) = AudioBackend::platform_exclusive() {
            return exclusive;
        }
    }
    AudioBackend::CpalShared
}

/// Met à jour la préférence. Effet immédiat sur le PROCHAIN morceau lu
/// (les lectures en cours ne sont pas interrompues — l'utilisateur devra
/// arrêter / relancer pour voir le changement de backend).
pub fn set_wasapi_exclusive(enabled: bool) {
    WASAPI_EXCLUSIVE.store(enabled, Ordering::Relaxed);
    log::info!(
        "🎚️  Sortie exclusive (bit-perfect) : {}",
        if enabled { "ON" } else { "OFF" }
    );
}
