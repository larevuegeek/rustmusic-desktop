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
use ringbuf::traits::{Consumer, Observer};

use super::traits::{AudioOutput, AudioOutputError};
use super::types::{AudioBackend, PlaybackAtomics, SymphoniaSharedState};

/// Backend CPAL pour la lecture Symphonia (LiveDecode + FullBuffer + seek).
pub struct CpalSymphoniaOutput {
    /// Le stream CPAL. Le Drop arrête la lecture automatiquement.
    stream: cpal::Stream,
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
        mut consumer: C,
        atomics: PlaybackAtomics,
        shared: SymphoniaSharedState,
    ) -> Result<Self, AudioOutputError>
    where
        C: Consumer<Item = f32> + Send + 'static,
    {
        let sample_rate = config.sample_rate;
        let channels = config.channels;
        let output_channels = shared.output_channels;

        // ─── Clones pour la closure ───
        let is_paused = atomics.is_paused.clone();
        let is_stopped = atomics.is_stopped.clone();
        let volume = atomics.volume.clone();
        let current_position_frames = atomics.current_position_frames.clone();
        let full_buffer_data = shared.full_buffer_data.clone();
        let full_buffer_cursor = shared.full_buffer_cursor.clone();
        let is_full_buffer_ready = shared.is_full_buffer_ready.clone();
        let current_source = shared.current_source.clone();
        let seek_flush = shared.seek_flush.clone();
        let pending_seek_frames = shared.pending_seek_frames.clone();

        // Compteur fade-in post-seek, mut local à la closure.
        let mut fade_in_samples: usize = 0;

        let stream = device
            .build_output_stream(
                config,
                move |output: &mut [f32], _| {
                    // --- Pause & Stop ---
                    if is_paused.load(Ordering::Relaxed) || is_stopped.load(Ordering::Relaxed) {
                        output.fill(0.0);
                        return;
                    }

                    // --- Seek flush : repositionner après un seek ---
                    if seek_flush.load(Ordering::Acquire) {
                        // Drain ring buffer (LiveDecode)
                        let to_skip = consumer.occupied_len();
                        consumer.skip(to_skip);

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
                        fade_in_samples = 2048; // ~42 ms à 48 kHz
                        output.fill(0.0);
                        return;
                    }

                    // --- Lecture audio selon le mode source ---
                    let source: u8 = current_source.load(Ordering::Relaxed);
                    let mut samples_read: usize = 0;

                    if source == 0 {
                        // Mode LiveDecode (RingBuffer)
                        samples_read = consumer.pop_slice(output);
                        if samples_read < output.len() {
                            output[samples_read..].fill(0.0);
                        }
                    } else {
                        // Mode FullBuffer
                        if is_full_buffer_ready.load(Ordering::Relaxed) {
                            if let Ok(fb) = full_buffer_data.read() {
                                let cursor: usize = full_buffer_cursor.load(Ordering::Relaxed);
                                let available: usize = fb.len().saturating_sub(cursor);
                                samples_read = available.min(output.len());

                                if samples_read > 0 {
                                    output[..samples_read]
                                        .copy_from_slice(&fb[cursor..cursor + samples_read]);
                                    full_buffer_cursor
                                        .fetch_add(samples_read, Ordering::Relaxed);
                                }
                                if samples_read < output.len() {
                                    output[samples_read..].fill(0.0);
                                }
                            } else {
                                output.fill(0.0);
                            }
                        } else {
                            output.fill(0.0);
                        }
                    }

                    // --- Mise à jour position ---
                    if samples_read > 0 && !seek_flush.load(Ordering::Relaxed) {
                        let source_mode = current_source.load(Ordering::Relaxed);
                        if source_mode == 1 {
                            // FullBuffer : dériver depuis le curseur (source de vérité)
                            let cursor = full_buffer_cursor.load(Ordering::Relaxed);
                            current_position_frames
                                .store(cursor / output_channels as usize, Ordering::Relaxed);
                        } else {
                            // LiveDecode : incrémenter les frames jouées
                            let frames = samples_read / output_channels as usize;
                            current_position_frames.fetch_add(frames, Ordering::Relaxed);
                        }
                    }

                    // --- Volume + Replay Gain + clipping + fade-in post-seek ---
                    let vol: f32 = volume.load(Ordering::Relaxed) as f32 / 100.0
                        * crate::core::audio_player::replay_gain::current_factor();
                    for s in output.iter_mut() {
                        let fade = if fade_in_samples > 0 {
                            fade_in_samples -= 1;
                            (2048 - fade_in_samples) as f32 / 2048.0
                        } else {
                            1.0
                        };
                        *s = (*s * vol * fade * 0.98).clamp(-1.0, 1.0);
                    }
                },
                {
                    // Throttle des erreurs CPAL : 1ère, puis 1 sur 100, puis 1
                    // sur 1000. Sur VM contrainte, on peut recevoir des
                    // centaines de BufferUnderrun/s alors que la lecture est
                    // OK (juste un retard de scheduler OS).
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
            stream,
            sample_rate,
            channels,
            device_name,
        })
    }
}

impl AudioOutput for CpalSymphoniaOutput {
    fn start(&mut self) -> Result<(), AudioOutputError> {
        self.stream
            .play()
            .map_err(|e| AudioOutputError::StartFailed(format!("stream.play: {e}")))
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
