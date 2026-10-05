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
pub use wasapi_exclusive::fermer_moteur;
#[cfg(target_os = "linux")]
mod exclusive_alsa;
#[cfg(target_os = "macos")]
mod exclusive_coreaudio;
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub mod dop_engine;
#[cfg(target_os = "linux")]
pub mod dop_alsa;
#[cfg(target_os = "linux")]
pub mod device_reservation;
#[cfg(target_os = "linux")]
pub mod profil_pipewire;
#[cfg(target_os = "macos")]
pub mod dop_coreaudio;

pub mod echantillons;
pub mod peripherique;

use crate::core::audio_player::pipeline_info::Repli;

/// Linux / macOS : le DAC accepte-t-il le rate source en exclusif ? Le décodeur produit alors ce rate.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn exclusif_au_rate_source(device_name: &str, source_rate: u32, channels: u16) -> bool {
    #[cfg(target_os = "linux")]
    {
        matches!(current_preference(), AudioBackend::AlsaExclusive)
            && dop_alsa::resolve_hw_id(device_name)
                .is_some_and(|hw| exclusive_alsa::probe(&hw, source_rate, channels).is_ok())
    }
    #[cfg(target_os = "macos")]
    {
        let _ = channels;
        matches!(current_preference(), AudioBackend::CoreAudioExclusive)
            && exclusive_coreaudio::CoreAudioExclusiveOutput::probe(device_name, source_rate).is_some()
    }
}

/// DSD converti en PCM, en exclusif (Linux) : la carte derrière l'un de ces noms
/// accepte-t-elle `rate` ? `Ok` porte de quoi ouvrir [`open_dsd_exclusive`].
#[cfg(target_os = "linux")]
pub fn probe_dsd_exclusive(
    noms: &[&str],
    rate: u32,
    channels: u16,
) -> Result<(String, exclusive_alsa::AlsaNegotiation), Repli> {
    let hw_id = noms
        .iter()
        .find_map(|nom| dop_alsa::resolve_hw_id(nom))
        .ok_or(Repli::AppareilIntrouvable)?;
    let negociation = exclusive_alsa::probe(&hw_id, rate, channels)?;
    Ok((hw_id, negociation))
}

/// Sortie exclusive du DSD converti : le rendu des fichiers PCM, lu dans le ring
/// buffer du décodeur DSD (source « décodage en direct »).
#[cfg(target_os = "linux")]
#[allow(clippy::too_many_arguments)]
pub fn open_dsd_exclusive<C>(
    hw_id: String,
    device_name: String,
    negociation: exclusive_alsa::AlsaNegotiation,
    rate: u32,
    channels: u16,
    consumer: C,
    atomics: PlaybackAtomics,
    seek_flush: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<Box<dyn AudioOutput>, AudioOutputError>
where
    C: Consumer<Item = f32> + Send + 'static,
{
    use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize};
    use std::sync::{Arc, RwLock};
    let shared = SymphoniaSharedState {
        full_buffer_data: Arc::new(RwLock::new(Vec::new())),
        full_buffer_cursor: Arc::new(AtomicUsize::new(0)),
        is_full_buffer_ready: Arc::new(AtomicBool::new(false)),
        // 0 : le rendu lit le ring buffer.
        current_source: Arc::new(AtomicU8::new(0)),
        seek_flush,
        // Le décodeur DSD tient lui-même la position après un seek.
        pending_seek_frames: Arc::new(AtomicUsize::new(usize::MAX)),
        output_channels: channels,
    };
    let mut sortie = exclusive_alsa::AlsaExclusiveOutput::try_new(
        hw_id,
        device_name,
        negociation,
        rate,
        channels,
        consumer,
        atomics,
        shared,
    )?;
    sortie.start()?;
    Ok(Box::new(sortie))
}

pub use preference::{current_preference, dop_enabled, set_dop_enabled, set_wasapi_exclusive};
pub use traits::{AudioOutput, AudioOutputError};
pub use types::{AudioBackend, PlaybackAtomics, SymphoniaSharedState};

use ringbuf::traits::Consumer;

/// Construit le backend audio pour la voie Symphonia.
///
/// **Paramètres CPAL** : `cpal_device`, `cpal_config`, `cpal_device_name` —
/// nécessaires pour le fallback (et le path par défaut).
///
/// **`nom_carte`** (Linux) : la sortie choisie, qui désigne la carte de la
/// sortie exclusive. Elle peut différer de `cpal_device_name` quand sa sortie
/// PipeWire a disparu le temps d'une lecture exclusive précédente.
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
    nom_carte: String,
    atomics: PlaybackAtomics,
    shared: SymphoniaSharedState,
    consumer: C,
) -> Result<(Box<dyn AudioOutput>, Option<Repli>), AudioOutputError>
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
                return Ok((Box::new(output), None));
            }
            Err(build_err) => {
                log::warn!(
                    "🎚️  WASAPI exclusive indisponible ({}), fallback CPAL shared mode",
                    build_err.error
                );
                let repli = if build_err.error.to_string().contains(crate::core::audio_player::audio_output_wasapi::AUCUN_FORMAT) {
                    Repli::FormatRefuse
                } else {
                    Repli::Indisponible
                };
                // Le consumer a été rendu, on peut le passer à CPAL ; le DAC doit être libre.
                wasapi_exclusive::fermer_moteur();
                return cpal_symphonia::CpalSymphoniaOutput::try_new(
                    cpal_device,
                    cpal_config,
                    cpal_device_name,
                    build_err.consumer,
                    atomics,
                    shared,
                    echantillons::MARGE_PARTAGEE,
                )
                .map(|o| {
                    log::info!("🎚️  Audio backend : CPAL shared ({})", o.device_name());
                    (Box::new(o) as Box<dyn AudioOutput>, Some(repli))
                });
            }
        }
    }
    #[allow(unused_mut)]
    let mut repli = None;
    // Le DAC s'ouvre au rate que produit le décodeur (celui négocié en amont) : jamais d'écart de vitesse.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let rate_decodeur = cpal_config.sample_rate;
    // ─── Tentative ALSA hw: exclusive (Linux uniquement) ───
    // La faisabilité est vérifiée AVANT de consommer le `consumer` : tant que
    // `probe` n'a pas répondu, le repli CPAL reste possible sans acrobatie.
    #[cfg(target_os = "linux")]
    if matches!(desired_backend, AudioBackend::AlsaExclusive) {
        let negociation = match dop_alsa::resolve_hw_id(&nom_carte) {
            Some(hw_id) => exclusive_alsa::probe(&hw_id, rate_decodeur, source_channels)
                .map(|n| (hw_id, n)),
            None => {
                log::info!(
                    "🎚️  ALSA exclusive : aucune carte hw: ne correspond à « {nom_carte} » \
                     → CPAL partagé"
                );
                Err(Repli::AppareilIntrouvable)
            }
        };
        match negociation {
            Err(motif) => repli = Some(motif),
            Ok((hw_id, negotiation)) => {
                match exclusive_alsa::AlsaExclusiveOutput::try_new(
                    hw_id,
                    nom_carte.clone(),
                    negotiation,
                    rate_decodeur,
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
                        return Ok((Box::new(output), None));
                    }
                    // Le consumer est perdu avec le thread : impossible de
                    // retomber sur CPAL. Cas extrême (échec de spawn).
                    Err(e) => return Err(e),
                }
            }
        }
    }

    // ─── Tentative CoreAudio exclusive (macOS uniquement) ───
    #[cfg(target_os = "macos")]
    if matches!(desired_backend, AudioBackend::CoreAudioExclusive) {
        if let Some(device_id) =
            exclusive_coreaudio::CoreAudioExclusiveOutput::probe(&cpal_device_name, rate_decodeur)
        {
            match exclusive_coreaudio::CoreAudioExclusiveOutput::try_new(
                cpal_device.clone(),
                device_id,
                cpal_device_name.clone(),
                rate_decodeur,
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
                    return Ok((Box::new(output), None));
                }
                // `CpalSymphoniaOutput::try_new` a déjà consommé le consumer :
                // si lui échoue, un fallback CPAL échouerait pareil.
                Err(e) => return Err(e),
            }
        }
        repli = Some(Repli::Indisponible);
    }

    #[cfg(not(target_os = "windows"))]
    let _ = source_sample_rate;
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    let _ = source_channels;

    // ─── Path CPAL (default, cross-platform) ───
    #[cfg(target_os = "windows")]
    wasapi_exclusive::fermer_moteur();
    // Carte choisie encore hors service (délai de grâce d'une lecture exclusive,
    // ou sonde exclusive refusée à l'instant) : elle revient à PipeWire, et le
    // flux partagé l'ouvre au lieu de la sortie système.
    #[cfg(target_os = "linux")]
    let (cpal_device, cpal_device_name) = match peripherique::reclaim_if_held(&nom_carte) {
        Some(device) => {
            use cpal::traits::DeviceTrait;
            let nom = device.description().map(|d| d.name().to_string()).unwrap_or(nom_carte);
            (device, nom)
        }
        None => (cpal_device, cpal_device_name),
    };
    #[cfg(not(target_os = "linux"))]
    let _ = nom_carte;
    let _ = desired_backend; // évite warning unused quand aucun backend exclusif
    cpal_symphonia::CpalSymphoniaOutput::try_new(
        cpal_device,
        cpal_config,
        cpal_device_name,
        consumer,
        atomics,
        shared,
        echantillons::MARGE_PARTAGEE,
    )
    .map(|o| {
        log::info!("🎚️  Audio backend : CPAL shared ({})", o.device_name());
        (Box::new(o) as Box<dyn AudioOutput>, repli)
    })
}
