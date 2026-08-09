//! Module `output` — backends de sortie audio pour la voie Symphonia.
//!
//! Architecture :
//! - `types.rs` : types partagés (atomics, état partagé Symphonia, choix backend)
//! - `traits.rs` : trait `AudioOutput` + erreurs typées
//! - `cpal_symphonia.rs` : implémentation CPAL shared mode (default, cross-platform)
//! - `wasapi_exclusive.rs` : implémentation WASAPI exclusive (Windows, bit-perfect)
//!
//! L'entry point public est `create_symphonia_output()` qui :
//!   1. Tente le backend demandé en premier (WASAPI si activé + sur Windows)
//!   2. Si WASAPI échoue à négocier le format, fallback automatique sur CPAL
//!   3. Si CPAL échoue aussi, propagation de l'erreur
//!
//! Le caller (`audio_player.rs`) ne connaît que le trait `AudioOutput`, il
//! n'a aucune dépendance sur cpal ou wasapi en direct.

mod cpal_symphonia;
pub mod preference;
mod traits;
mod types;
#[cfg(target_os = "windows")]
mod wasapi_exclusive;
#[cfg(target_os = "linux")]
mod exclusive_alsa;
#[cfg(target_os = "macos")]
mod exclusive_coreaudio;
#[cfg(target_os = "windows")]
pub mod dop_engine;
#[cfg(target_os = "linux")]
pub mod dop_alsa;
#[cfg(target_os = "linux")]
pub mod device_reservation;
#[cfg(target_os = "macos")]
pub mod dop_coreaudio;

pub use preference::{current_preference, dop_enabled, set_dop_enabled, set_wasapi_exclusive};
pub use traits::{AudioOutput, AudioOutputError};
pub use types::{AudioBackend, PlaybackAtomics, SymphoniaSharedState};

use ringbuf::traits::Consumer;

/// Construit le backend audio pour la voie Symphonia.
///
/// **Paramètres CPAL** : `cpal_device`, `cpal_config`, `cpal_device_name` —
/// nécessaires pour le fallback (et le path par défaut).
///
/// **Paramètres WASAPI** : `source_sample_rate`, `source_channels` —
/// utilisés pour la négociation de format. Le device WASAPI est toujours le
/// device par défaut Windows (le sélecteur de device CPAL n'est pas réutilisé
/// car le mapping CPAL→WASAPI device n'est pas trivial).
///
/// **Comportement** :
/// - `desired_backend = CpalShared` (toujours valable) → utilise CPAL direct
/// - `desired_backend = WasapiExclusive` + non-Minimal profile → tente WASAPI,
///   fallback CPAL si négociation échoue.
/// - Sur les OS non-Windows, le toggle WASAPI est ignoré et on utilise CPAL.
pub fn create_symphonia_output<C>(
    desired_backend: AudioBackend,
    source_sample_rate: u32,
    source_channels: u16,
    cpal_device: cpal::Device,
    cpal_config: cpal::StreamConfig,
    cpal_device_name: String,
    atomics: PlaybackAtomics,
    shared: SymphoniaSharedState,
    consumer: C,
) -> Result<Box<dyn AudioOutput>, AudioOutputError>
where
    C: Consumer<Item = f32> + Send + 'static,
{
    // ─── Tentative WASAPI exclusive (Windows uniquement) ───
    #[cfg(target_os = "windows")]
    if matches!(desired_backend, AudioBackend::WasapiExclusive) {
        match wasapi_exclusive::WasapiExclusiveOutput::try_new(
            source_sample_rate,
            source_channels,
            Some(cpal_device_name.clone()),
            atomics.clone(),
            shared.clone(),
            consumer,
        ) {
            Ok(output) => {
                log::info!(
                    "🎚️  Audio backend : WASAPI exclusive ({}) — {} Hz / {} ch (bit-perfect)",
                    output.device_name(),
                    output.output_sample_rate(),
                    output.output_channels()
                );
                return Ok(Box::new(output));
            }
            Err(build_err) => {
                log::warn!(
                    "🎚️  WASAPI exclusive indisponible ({}), fallback CPAL shared mode",
                    build_err.error
                );
                // Le consumer a été rendu, on peut le passer à CPAL.
                return cpal_symphonia::CpalSymphoniaOutput::try_new(
                    cpal_device,
                    cpal_config,
                    cpal_device_name,
                    build_err.consumer,
                    atomics,
                    shared,
                )
                .map(|o| {
                    log::info!("🎚️  Audio backend : CPAL shared ({})", o.device_name());
                    Box::new(o) as Box<dyn AudioOutput>
                });
            }
        }
    }
    // ─── Tentative ALSA hw: exclusive (Linux uniquement) ───
    // La faisabilité est vérifiée AVANT de consommer le `consumer` : tant que
    // `probe` n'a pas répondu, le repli CPAL reste possible sans acrobatie.
    #[cfg(target_os = "linux")]
    if matches!(desired_backend, AudioBackend::AlsaExclusive) {
        if let Some(hw_id) = dop_alsa::resolve_hw_id(&cpal_device_name) {
            if let Some(negotiation) =
                exclusive_alsa::probe(&hw_id, source_sample_rate, source_channels)
            {
                match exclusive_alsa::AlsaExclusiveOutput::try_new(
                    hw_id,
                    cpal_device_name.clone(),
                    negotiation,
                    source_sample_rate,
                    source_channels,
                    consumer,
                    atomics.clone(),
                    shared.clone(),
                ) {
                    Ok(output) => {
                        log::info!(
                            "🎚️  Audio backend : ALSA exclusive ({}) — {} Hz / {} ch (bit-perfect)",
                            output.device_name(),
                            output.output_sample_rate(),
                            output.output_channels()
                        );
                        return Ok(Box::new(output));
                    }
                    // Le consumer est perdu avec le thread : impossible de
                    // retomber sur CPAL. Cas extrême (échec de spawn).
                    Err(e) => return Err(e),
                }
            }
        } else {
            log::info!(
                "🎚️  ALSA exclusive : aucune carte hw: ne correspond à « {cpal_device_name} » \
                 → CPAL partagé"
            );
        }
    }

    // ─── Tentative CoreAudio exclusive (macOS uniquement) ───
    #[cfg(target_os = "macos")]
    if matches!(desired_backend, AudioBackend::CoreAudioExclusive) {
        if let Some(device_id) =
            exclusive_coreaudio::CoreAudioExclusiveOutput::probe(&cpal_device_name, source_sample_rate)
        {
            match exclusive_coreaudio::CoreAudioExclusiveOutput::try_new(
                cpal_device.clone(),
                device_id,
                cpal_device_name.clone(),
                source_sample_rate,
                source_channels,
                consumer,
                atomics.clone(),
                shared.clone(),
            ) {
                Ok(output) => {
                    log::info!(
                        "🎚️  Audio backend : CoreAudio exclusive ({}) — {} Hz / {} ch (bit-perfect)",
                        output.device_name(),
                        output.output_sample_rate(),
                        output.output_channels()
                    );
                    return Ok(Box::new(output));
                }
                // `CpalSymphoniaOutput::try_new` a déjà consommé le consumer :
                // si lui échoue, un fallback CPAL échouerait pareil.
                Err(e) => return Err(e),
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    let _ = (source_sample_rate, source_channels);

    // ─── Path CPAL (default, cross-platform) ───
    let _ = desired_backend; // évite warning unused quand aucun backend exclusif
    cpal_symphonia::CpalSymphoniaOutput::try_new(
        cpal_device,
        cpal_config,
        cpal_device_name,
        consumer,
        atomics,
        shared,
    )
    .map(|o| {
        log::info!("🎚️  Audio backend : CPAL shared ({})", o.device_name());
        Box::new(o) as Box<dyn AudioOutput>
    })
}
