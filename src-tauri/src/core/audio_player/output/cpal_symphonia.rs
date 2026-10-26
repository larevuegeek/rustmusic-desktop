//! Backend CPAL pour la voie Symphonia.
//!
//! Gère les deux modes :
//! - **LiveDecode** : consomme le ring buffer alimenté par le décodeur.
//! - **FullBuffer** : lit dans `full_buffer_data` à `full_buffer_cursor`.
//!
//! Le seek est piloté par `seek_flush` + `pending_seek_frames` :
//! - Quand `seek_flush=true` est détecté, le buffer interne CPAL est drainé,
//!   le curseur FullBuffer est repositionné depuis `pending_seek_frames`
//!   (ou `current_position_frames` en LiveDecode), un fade-in de 2048
//!   samples est appliqué pour éviter le clic, et le flag est cleared.

use std::sync::atomic::Ordering;

use cpal::traits::{DeviceTrait, StreamTrait};
use ringbuf::traits::Consumer;

use super::traits::{AudioOutput, AudioOutputError};
use super::types::{AudioBackend, PlaybackAtomics, SymphoniaSharedState};

/// Backend CPAL pour la lecture Symphonia (LiveDecode + FullBuffer + seek).
pub struct CpalSymphoniaOutput {
    /// Flux propre (hog mode macOS) ; `None` en partagé, servi par le moteur persistant.
    stream: Option<cpal::Stream>,
    /// Partagé : de quoi se brancher au moteur, au démarrage.
    pending: Option<(cpal::Device, cpal::StreamConfig, super::shared_engine::RenderFn)>,
    /// Partagé : la piste branchée, débranchée au Drop.
    track: Option<u64>,
    sample_rate: u32,
    channels: u16,
    device_name: String,
}

impl CpalSymphoniaOutput {
    /// Construit le stream CPAL avec son callback complet. Ne le démarre pas
    /// (cf. `start()`).
    ///
    /// Le `Consumer` du ring buffer est consommé (déplacé dans la closure).
    /// Tous les atomics sont clonés en interne pour la closure.
    pub fn try_new<C>(
        device: cpal::Device,
        config: cpal::StreamConfig,
        device_name: String,
        consumer: C,
        atomics: PlaybackAtomics,
        shared: SymphoniaSharedState,
        // `MARGE_PARTAGEE` en partagé ; 1.0 sous le hog mode macOS (bit-perfect).
        marge: f32,
        // Partagé : flux gardé ouvert entre les pistes ; sinon un flux à soi.
        persistent: bool,
    ) -> Result<Self, AudioOutputError>
    where
        C: Consumer<Item = f32> + Send + 'static,
    {
        let sample_rate = config.sample_rate;
        let channels = config.channels;
        let render = render_fn(consumer, atomics, shared, marge, sample_rate);

        if persistent {
            return Ok(Self {
                stream: None,
                pending: Some((device, config.clone(), Box::new(render))),
                track: None,
                sample_rate,
                channels,
                device_name,
            });
        }

        let mut render = render;
        let stream = device
            .build_output_stream(
                config,
                move |output: &mut [f32], _| render(output),
                {
                    // Erreurs throttlées : 1re, puis 1 sur 100, puis 1 sur 1000.
                    let cpal_error_count =
                        std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
                    move |err| {
                        let n = cpal_error_count.fetch_add(1, Ordering::Relaxed);
                        if n == 0 || (n < 1000 && n % 100 == 0) || n % 1000 == 0 {
                            log::error!("❌ Erreur audio (#{}): {:?}", n + 1, err);
                        }
                    }
                },
                None,
            )
            .map_err(|e| AudioOutputError::BuildFailed(format!("build_output_stream: {e}")))?;

        Ok(Self {
            stream: Some(stream),
            pending: None,
            track: None,
            sample_rate,
            channels,
            device_name,
        })
    }
}

impl AudioOutput for CpalSymphoniaOutput {
    fn start(&mut self) -> Result<(), AudioOutputError> {
        if let Some((device, config, render)) = self.pending.take() {
            self.track = Some(super::shared_engine::attach(&device, &config, &self.device_name, render)?);
            return Ok(());
        }
        match &self.stream {
            Some(stream) => stream
                .play()
                .map_err(|e| AudioOutputError::StartFailed(format!("stream.play: {e}"))),
            None => Ok(()),
        }
    }

    fn output_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn output_channels(&self) -> u16 {
        self.channels
    }

    fn device_name(&self) -> &str {
        &self.device_name
    }

    fn backend(&self) -> AudioBackend {
        AudioBackend::CpalShared
    }
}

impl Drop for CpalSymphoniaOutput {
    fn drop(&mut self) {
        if let Some(id) = self.track.take() {
            super::shared_engine::detach(id);
        }
    }
}

/// Lecture de la source (ring buffer ou tampon complet, enchaînement compris) et position, en temps du morceau.
struct SourceReader<C> {
    consumer: C,
    current_position_frames: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    full_buffer_data: std::sync::Arc<std::sync::RwLock<Vec<f32>>>,
    full_buffer_cursor: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    is_full_buffer_ready: std::sync::Arc<std::sync::atomic::AtomicBool>,
    current_source: std::sync::Arc<std::sync::atomic::AtomicU8>,
    seek_flush: std::sync::Arc<std::sync::atomic::AtomicBool>,
    promotion_armed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    output_channels: u16,
}

impl<C: Consumer<Item = f32>> SourceReader<C> {
    /// Remplit `output` depuis la source ; rend le nombre d'échantillons lus (le reste est mis à zéro).
    fn read(&mut self, output: &mut [f32]) -> usize {
        let source: u8 = self.current_source.load(Ordering::Acquire);
        let mut samples_read: usize = 0;

        if source == 0 {
            // Mode LiveDecode (RingBuffer)
            samples_read = self.consumer.pop_slice(output);
            if samples_read < output.len() {
                output[samples_read..].fill(0.0);
            }
        } else {
            // Mode FullBuffer
            if self.is_full_buffer_ready.load(Ordering::Relaxed) {
                if let Ok(fb) = self.full_buffer_data.read() {
                    let cursor: usize = self.full_buffer_cursor.load(Ordering::Relaxed);
                    let available: usize = fb.len().saturating_sub(cursor);
                    samples_read = available.min(output.len());

                    if samples_read > 0 {
                        output[..samples_read]
                            .copy_from_slice(&fb[cursor..cursor + samples_read]);
                        self.full_buffer_cursor
                            .fetch_add(samples_read, Ordering::Relaxed);
                    }
                    if samples_read < output.len() {
                        output[samples_read..].fill(0.0);
                    }
                } else {
                    output.fill(0.0);
                }

                // Enchaînement sans blanc : la piste finit au milieu de ce
                // tampon, on prend la suivante ici même pour le compléter,
                // au lieu de laisser des zéros jusqu'au fil de contrôle.
                if samples_read < output.len()
                    && self.promotion_armed.load(Ordering::Acquire)
                    && crate::core::audio_player::preload::gapless_enabled()
                {
                    if let Ok(mut fb) = self.full_buffer_data.try_write() {
                        let cursor: usize = self.full_buffer_cursor.load(Ordering::Relaxed);
                        if cursor >= fb.len()
                            && crate::core::audio_player::preload::promote_from_callback(
                                &mut fb,
                            )
                        {
                            let n = fb.len().min(output.len() - samples_read);
                            output[samples_read..samples_read + n]
                                .copy_from_slice(&fb[..n]);
                            self.full_buffer_cursor.store(n, Ordering::Release);
                            samples_read += n;
                        }
                    }
                }
            } else {
                output.fill(0.0);
            }
        }

        // --- Mise à jour position ---
        if samples_read > 0 && !self.seek_flush.load(Ordering::Relaxed) {
            let source_mode = self.current_source.load(Ordering::Relaxed);
            if source_mode == 1 {
                // FullBuffer : dériver depuis le curseur (source de vérité)
                let cursor = self.full_buffer_cursor.load(Ordering::Relaxed);
                self.current_position_frames
                    .store(cursor / self.output_channels as usize, Ordering::Relaxed);
            } else {
                // LiveDecode : incrémenter les frames jouées
                let frames = samples_read / self.output_channels as usize;
                self.current_position_frames.fetch_add(frames, Ordering::Relaxed);
            }
        }
        samples_read
    }
}

/// Rendu d'une piste (lecture, seek, enchaînement, vitesse, gain), commun aux sorties qui
/// tirent le son à leur rythme : carte son partagée, AirPlay.
pub(super) fn render_fn<C>(
    consumer: C,
    atomics: PlaybackAtomics,
    shared: SymphoniaSharedState,
    marge: f32,
    sample_rate: u32,
) -> impl FnMut(&mut [f32]) + Send + 'static
where
    C: Consumer<Item = f32> + Send + 'static,
{
    let output_channels = shared.output_channels;
    // ─── Clones pour la closure ───
    let is_paused = atomics.is_paused.clone();
    let is_stopped = atomics.is_stopped.clone();
    let volume = atomics.volume.clone();
    let current_position_frames = atomics.current_position_frames.clone();
    let full_buffer_cursor = shared.full_buffer_cursor.clone();
    let seek_flush = shared.seek_flush.clone();
    let pending_seek_frames = shared.pending_seek_frames.clone();
    let mut reader = SourceReader {
        consumer,
        current_position_frames: current_position_frames.clone(),
        full_buffer_data: shared.full_buffer_data.clone(),
        full_buffer_cursor: full_buffer_cursor.clone(),
        is_full_buffer_ready: shared.is_full_buffer_ready.clone(),
        current_source: shared.current_source.clone(),
        seek_flush: seek_flush.clone(),
        promotion_armed: shared.promotion_armed.clone(),
        output_channels,
    };

    // Compteur fade-in post-seek, mut local à la closure.
    let mut fade_in_samples: usize = 0;
    // Vitesse (podcasts) : étireur préparé d'avance, rien n'est alloué dans le rappel.
    let mut stretcher = crate::core::audio_player::time_stretch::TimeStretcher::new(sample_rate, output_channels as usize);
    let mut stretching = false;

    move |output: &mut [f32]| {
        // --- Pause & Stop ---
        if is_paused.load(Ordering::Relaxed) || is_stopped.load(Ordering::Relaxed) {
            output.fill(0.0);
            return;
        }

        // --- Seek flush : repositionner après un seek ---
        if seek_flush.load(Ordering::Acquire) {
            // Drain ring buffer (LiveDecode)
            let to_skip = reader.consumer.occupied_len();
            reader.consumer.skip(to_skip);

            // Repositionner le curseur FullBuffer depuis
            // pending_seek_frames (single-writer Phase 6 →
            // single-reader CPAL). Fallback sur current_position_frames
            // en LiveDecode pur.
            let target = pending_seek_frames.load(Ordering::Acquire);
            let new_frames = if target != usize::MAX {
                pending_seek_frames.store(usize::MAX, Ordering::Relaxed);
                current_position_frames.store(target, Ordering::Relaxed);
                target
            } else {
                current_position_frames.load(Ordering::Relaxed)
            };
            let new_cursor = new_frames * output_channels as usize;
            full_buffer_cursor.store(new_cursor, Ordering::Relaxed);

            seek_flush.store(false, Ordering::Release);
            stretcher.reset();
            fade_in_samples = 2048; // ~42 ms à 48 kHz
            output.fill(0.0);
            return;
        }

        // --- Lecture audio, à la vitesse demandée ---
        let speed = crate::core::audio_player::time_stretch::speed();
        if crate::core::audio_player::time_stretch::is_normal(speed) {
            if stretching {
                // Retour à 1× : le son en réserve dans l'étireur est lâché, raccord adouci.
                stretching = false;
                stretcher.reset();
                fade_in_samples = fade_in_samples.max(1024);
            }
            reader.read(output);
        } else {
            stretching = true;
            stretcher.process(output, speed, |buf| reader.read(buf));
        }

        // --- Volume + Replay Gain + clipping + fade-in post-seek ---
        let vol: f32 = volume.load(Ordering::Relaxed) as f32 / 100.0
            * crate::core::audio_player::replay_gain::current_factor();
        super::samples::apply_gain(output, vol, &mut fade_in_samples, marge);
    }
}
