//! PCM resampler wrapper around `rubato`'s FFT-based resampler.
//!
//! Used by both Symphonia-decoded paths (FLAC, MP3, WAV…) and DSD playback
//! to bring intermediate PCM rates to the device's output rate.
//!
//! The wrapper hides the rubato chunk-size / sub-chunks tuning, the
//! deinterleave→process→re-interleave dance, and the per-channel
//! accumulator buffers — callers see only an interleaved API.

use audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::{Fft, FixedSync, Resampler as RubatoResampler, WindowFunction};

use crate::core::audio_quality::AudioQualityProfile;

/// Stateful resampler. Input and output are interleaved f32 frames.
pub struct Resampler {
    fft: Fft<f32>,
    channels: usize,
    /// Number of input frames the FFT consumes per chunk.
    chunk_size: usize,
    /// Per-channel accumulator (deinterleaved staging buffer).
    accumulator: Vec<Vec<f32>>,
}

impl Resampler {
    /// Construct a resampler with the High (audiophile) quality preset.
    /// Convenience wrapper around [`Self::maybe_new_with_profile`].
    pub fn maybe_new(
        input_rate: u32,
        output_rate: u32,
        channels: usize,
    ) -> Result<Option<Self>, String> {
        Self::maybe_new_with_profile(
            input_rate,
            output_rate,
            channels,
            AudioQualityProfile::High,
        )
    }

    /// Construct a resampler if `input_rate != output_rate`, tuned for the
    /// given quality profile.
    /// Returns `Ok(None)` when input == output (no resampling needed → caller passes through).
    /// Returns `Err(_)` if rubato cannot build the FFT (rare; bad ratio).
    pub fn maybe_new_with_profile(
        input_rate: u32,
        output_rate: u32,
        channels: usize,
        profile: AudioQualityProfile,
    ) -> Result<Option<Self>, String> {
        if input_rate == output_rate {
            return Ok(None);
        }

        let chunk_size = profile.resampler_chunk_size(output_rate);
        let sub_chunks = profile.resampler_sub_chunks(output_rate);

        log::debug!(
            "🔄 Resampler {} → {} | profile {:?} | chunk {} | sub {}",
            input_rate, output_rate, profile, chunk_size, sub_chunks,
        );

        // `new_custom` garde les sous-blocs du profil ; la fenêtre est celle que
        // rubato appliquait d'office avant la 5.
        let fft = Fft::<f32>::new_custom(
            input_rate as usize,
            output_rate as usize,
            chunk_size,
            sub_chunks,
            channels,
            WindowFunction::BlackmanHarris2,
            FixedSync::Input,
        )
        .map_err(|e| format!("Erreur resampler: {:?}", e))?;

        // input_frames_next() peut différer du chunk_size demandé (rubato l'ajuste
        // selon le ratio interne) ; on utilise la valeur effective.
        let actual_chunk_size = fft.input_frames_next();
        let accumulator = vec![Vec::new(); channels];

        Ok(Some(Self {
            fft,
            channels,
            chunk_size: actual_chunk_size,
            accumulator,
        }))
    }

    /// Push interleaved input samples, accumulate per channel, and return any
    /// resampled output frames ready (interleaved). May return an empty Vec
    /// if not enough input has accumulated for a full FFT chunk yet.
    pub fn process_interleaved(&mut self, input: &[f32]) -> Vec<f32> {
        // Deinterleave into per-channel accumulator
        for (i, &sample) in input.iter().enumerate() {
            let ch = i % self.channels;
            self.accumulator[ch].push(sample);
        }

        let mut output_samples: Vec<f32> = Vec::new();

        // Drain full chunks while we have enough accumulated input
        while self.accumulator[0].len() >= self.chunk_size {
            let mut input_chunk: Vec<Vec<f32>> = Vec::with_capacity(self.channels);
            for ch in 0..self.channels {
                input_chunk.push(self.accumulator[ch].drain(..self.chunk_size).collect());
            }

            let input_adapter =
                match SequentialSliceOfVecs::new(&input_chunk, self.channels, self.chunk_size) {
                    Ok(a) => a,
                    Err(e) => {
                        log::error!("Input adapter error: {}", e);
                        break;
                    }
                };

            let max_output_frames: usize = self.fft.output_frames_max();
            let mut output_chunk: Vec<Vec<f32>> =
                vec![vec![0.0; max_output_frames]; self.channels];

            let mut output_adapter = match SequentialSliceOfVecs::new_mut(
                &mut output_chunk,
                self.channels,
                max_output_frames,
            ) {
                Ok(a) => a,
                Err(e) => {
                    log::error!("Output adapter error: {}", e);
                    break;
                }
            };

            let (frames_read, frames_written) =
                match self
                    .fft
                    .process_into_buffer(&input_adapter, &mut output_adapter, None)
                {
                    Ok(result) => result,
                    Err(e) => {
                        log::error!("⚠️ Erreur resampling: {:?}", e);
                        continue;
                    }
                };

            if frames_read != self.chunk_size {
                log::error!(
                    "⚠️ Frames lues ({}) != attendues ({})",
                    frames_read,
                    self.chunk_size
                );
            }

            // Re-interleave output chunk into the result
            for i in 0..frames_written {
                for ch in 0..self.channels {
                    output_samples.push(output_chunk[ch][i]);
                }
            }
        }

        output_samples
    }

    /// Reset internal state (call after seek).
    pub fn reset(&mut self) {
        for ch in self.accumulator.iter_mut() {
            ch.clear();
        }
        self.fft.reset();
    }
}
