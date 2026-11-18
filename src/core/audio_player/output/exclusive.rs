//! Sorties exclusives sous Linux et macOS : sonde du DAC, DSD converti en PCM.

use super::*;
use crate::core::audio_player::pipeline_info::Fallback;
use ringbuf::traits::Consumer;

/// Linux / macOS : le DAC accepte-t-il le rate source en exclusif ? Le décodeur produit alors ce rate.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn exclusive_at_source_rate(device_name: &str, source_rate: u32, channels: u16) -> bool {
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
    names: &[&str],
    rate: u32,
    channels: u16,
) -> Result<(String, exclusive_alsa::AlsaNegotiation), Fallback> {
    let hw_id = names
        .iter()
        .find_map(|name| dop_alsa::resolve_hw_id(name))
        .ok_or(Fallback::DeviceNotFound)?;
    let negotiation = exclusive_alsa::probe(&hw_id, rate, channels)?;
    Ok((hw_id, negotiation))
}

/// Sortie exclusive du DSD converti : le rendu des fichiers PCM, lu dans le ring
/// buffer du décodeur DSD (source « décodage en direct »).
#[cfg(target_os = "linux")]
#[allow(clippy::too_many_arguments)]
pub fn open_dsd_exclusive<C>(
    hw_id: String,
    device_name: String,
    negotiation: exclusive_alsa::AlsaNegotiation,
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
        promotion_armed: Arc::new(AtomicBool::new(false)),
    };
    let mut output = exclusive_alsa::AlsaExclusiveOutput::try_new(
        hw_id,
        device_name,
        negotiation,
        rate,
        channels,
        consumer,
        atomics,
        shared,
    )?;
    output.start()?;
    Ok(Box::new(output))
}
