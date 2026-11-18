//! Construction de la sortie audio : AirPlay, ou le backend demandé avec repli sur CPAL.

use super::*;
use crate::core::audio_player::pipeline_info::Fallback;
use ringbuf::traits::Consumer;

/// Sortie AirPlay : le décodeur produit déjà du 44,1 kHz stéréo.
pub fn create_airplay_output<C>(
    device: crate::core::network_player::airplay::device::AirplayDevice,
    atomics: PlaybackAtomics,
    shared: SymphoniaSharedState,
    consumer: C,
) -> Box<dyn AudioOutput>
where
    C: Consumer<Item = f32> + Send + 'static,
{
    // La carte son n'est plus utilisée : sa sortie partagée est rendue.
    shared_engine::close();
    log::info!("🎚️  Audio backend : AirPlay ({})", device.name);
    Box::new(airplay::AirplayOutput::new(device, consumer, atomics, shared))
}

/// Construit le backend audio pour la voie Symphonia.
///
/// **Paramètres CPAL** : `cpal_device`, `cpal_config`, `cpal_device_name` —
/// nécessaires pour le fallback (et le path par défaut).
///
/// **`card_name`** (Linux) : la sortie choisie, qui désigne la carte de la
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
    card_name: String,
    atomics: PlaybackAtomics,
    shared: SymphoniaSharedState,
    consumer: C,
) -> Result<(Box<dyn AudioOutput>, Option<Fallback>), AudioOutputError>
where
    C: Consumer<Item = f32> + Send + 'static,
{
    // ─── Tentative WASAPI exclusive (Windows uniquement) ───
    #[cfg(target_os = "windows")]
    if matches!(desired_backend, AudioBackend::WasapiExclusive) {
        // Le flux partagé tiendrait le DAC : l'exclusif ne l'aurait pas.
        shared_engine::close();
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
                let fallback = if build_err.error.to_string().contains(crate::core::audio_player::audio_output_wasapi::NO_FORMAT) {
                    Fallback::FormatRefuse
                } else {
                    Fallback::Unavailable
                };
                // Le consumer a été rendu, on peut le passer à CPAL ; le DAC doit être libre.
                wasapi_exclusive::close_engine();
                return cpal_symphonia::CpalSymphoniaOutput::try_new(
                    cpal_device,
                    cpal_config,
                    cpal_device_name,
                    build_err.consumer,
                    atomics,
                    shared,
                    samples::SHARED_MARGIN,
                    true,
                )
                .map(|o| {
                    log::info!("🎚️  Audio backend : CPAL shared ({})", o.device_name());
                    (Box::new(o) as Box<dyn AudioOutput>, Some(fallback))
                });
            }
        }
    }
    #[allow(unused_mut)]
    let mut fallback = None;
    // Le DAC s'ouvre au rate que produit le décodeur (celui négocié en amont) : jamais d'écart de vitesse.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let decoder_rate = cpal_config.sample_rate;
    // ─── Tentative ALSA hw: exclusive (Linux uniquement) ───
    // La faisabilité est vérifiée AVANT de consommer le `consumer` : tant que
    // `probe` n'a pas répondu, le repli CPAL reste possible sans acrobatie.
    #[cfg(target_os = "linux")]
    if matches!(desired_backend, AudioBackend::AlsaExclusive) {
        shared_engine::close();
        let negotiation = match dop_alsa::resolve_hw_id(&card_name) {
            Some(hw_id) => exclusive_alsa::probe(&hw_id, decoder_rate, source_channels)
                .map(|n| (hw_id, n)),
            None => {
                log::info!(
                    "🎚️  ALSA exclusive : aucune carte hw: ne correspond à « {card_name} » \
                     → CPAL partagé"
                );
                Err(Fallback::DeviceNotFound)
            }
        };
        match negotiation {
            Err(pattern) => fallback = Some(pattern),
            Ok((hw_id, negotiation)) => {
                match exclusive_alsa::AlsaExclusiveOutput::try_new(
                    hw_id,
                    card_name.clone(),
                    negotiation,
                    decoder_rate,
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
        shared_engine::close();
        if let Some(device_id) =
            exclusive_coreaudio::CoreAudioExclusiveOutput::probe(&cpal_device_name, decoder_rate)
        {
            match exclusive_coreaudio::CoreAudioExclusiveOutput::try_new(
                cpal_device.clone(),
                device_id,
                cpal_device_name.clone(),
                decoder_rate,
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
        fallback = Some(Fallback::Unavailable);
    }

    #[cfg(not(target_os = "windows"))]
    let _ = source_sample_rate;
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    let _ = source_channels;

    // ─── Path CPAL (default, cross-platform) ───
    #[cfg(target_os = "windows")]
    wasapi_exclusive::close_engine();
    // Carte choisie encore hors service (délai de grâce d'une lecture exclusive,
    // ou sonde exclusive refusée à l'instant) : elle revient à PipeWire, et le
    // flux partagé l'ouvre au lieu de la sortie système.
    #[cfg(target_os = "linux")]
    let (cpal_device, cpal_device_name) = match device_lookup::reclaim_if_held(&card_name) {
        Some(device) => {
            use cpal::traits::DeviceTrait;
            let name = device.description().map(|d| d.name().to_string()).unwrap_or(card_name);
            (device, name)
        }
        None => (cpal_device, cpal_device_name),
    };
    #[cfg(not(target_os = "linux"))]
    let _ = card_name;
    let _ = desired_backend; // évite warning unused quand aucun backend exclusif
    cpal_symphonia::CpalSymphoniaOutput::try_new(
        cpal_device,
        cpal_config,
        cpal_device_name,
        consumer,
        atomics,
        shared,
        samples::SHARED_MARGIN,
        true,
    )
    .map(|o| {
        log::info!("🎚️  Audio backend : CPAL shared ({})", o.device_name());
        (Box::new(o) as Box<dyn AudioOutput>, fallback)
    })
}
