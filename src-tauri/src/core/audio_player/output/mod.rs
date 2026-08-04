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
    #[cfg(not(target_os = "windows"))]
    let _ = (source_sample_rate, source_channels);

    // ─── Path CPAL (default, cross-platform) ───
    let _ = desired_backend; // évite warning unused sur non-Windows
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
