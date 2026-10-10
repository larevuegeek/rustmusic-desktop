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
    /// Output frames still to be dropped: the filter delay at the start of a
    /// stream, which would otherwise show up as leading silence.
    skip_out: usize,
    /// Input frames pushed / output frames emitted since the last reset,
    /// used by [`Self::finish`] to know how much tail is still owed.
    in_frames: usize,
    out_frames: usize,
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
        let skip_out = fft.output_delay();

        Ok(Some(Self {
            fft,
            channels,
            chunk_size: actual_chunk_size,
            accumulator,
            skip_out,
            in_frames: 0,
            out_frames: 0,
        }))
    }

    /// Push interleaved input samples, accumulate per channel, and return any
    /// resampled output frames ready (interleaved). May return an empty Vec
    /// if not enough input has accumulated for a full FFT chunk yet.
    pub fn process_interleaved(&mut self, input: &[f32]) -> Vec<f32> {
        self.in_frames += input.len() / self.channels;

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

        // Drop the start-of-stream filter delay (leading silence).
        if self.skip_out > 0 {
            let drop_frames = self.skip_out.min(output_samples.len() / self.channels);
            output_samples.drain(..drop_frames * self.channels);
            self.skip_out -= drop_frames;
        }
        self.out_frames += output_samples.len() / self.channels;

        output_samples
    }

    /// Flush the end of the stream: pushes zero padding through the filter so
    /// the input frames still held back (partial chunk + filter delay) come
    /// out, then truncates to exactly `input_frames * ratio` output frames.
    /// Call once at end of media; the resampler must be `reset` before reuse.
    pub fn finish(&mut self) -> Vec<f32> {
        let expected = (self.in_frames as f64 * self.fft.resample_ratio()).round() as usize;
        let mut tail: Vec<f32> = Vec::new();
        let mut passes = 0;

        while self.out_frames < expected && passes < 16 {
            let pad_frames = self.chunk_size - self.accumulator[0].len();
            let zeros = vec![0.0f32; pad_frames * self.channels];
            tail.extend(self.process_interleaved(&zeros));
            passes += 1;
        }

        if self.out_frames > expected {
            let excess = (self.out_frames - expected).min(tail.len() / self.channels);
            tail.truncate(tail.len() - excess * self.channels);
            self.out_frames -= excess;
        }
        tail
    }

    /// Reset internal state (call after seek).
    pub fn reset(&mut self) {
        for ch in self.accumulator.iter_mut() {
            ch.clear();
        }
        self.fft.reset();
        self.skip_out = self.fft.output_delay();
        self.in_frames = 0;
        self.out_frames = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frames: usize, rate: u32) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let v = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / rate as f32).sin() * 0.5;
                [v, v]
            })
            .collect()
    }

    #[test]
    fn finish_emits_full_length_without_leading_silence() {
        let in_frames = 44_100 * 3 + 123;
        let mut rs = Resampler::maybe_new(44_100, 48_000, 2).unwrap().unwrap();
        let input = sine(in_frames, 44_100);

        let mut out = Vec::new();
        for block in input.chunks(2 * 1152) {
            out.extend(rs.process_interleaved(block));
        }
        out.extend(rs.finish());

        let expected = (in_frames as f64 * 48_000.0 / 44_100.0).round() as usize;
        assert_eq!(out.len() / 2, expected);

        // The first frames must carry signal, not filter-delay silence.
        let lead = out.chunks(2).take(200).map(|f| f[0].abs()).fold(0.0f32, f32::max);
        assert!(lead > 0.05, "leading silence, peak {lead}");
        // And the end must not have been cut off.
        let tail = out.chunks(2).rev().take(200).map(|f| f[0].abs()).fold(0.0f32, f32::max);
        assert!(tail > 0.05, "tail cut off, peak {tail}");
    }
}
