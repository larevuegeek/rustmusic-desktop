//! DSD → PCM converter using a Blackman-Harris windowed-sinc FIR filter,
//! implemented with the lookup-table approach popularized by Sebastian
//! Gesemann's `dsd2pcm` project.
//!
//! Concept:
//!   - The filter has N taps (N = `WINDOW_BYTES * 8`).
//!   - For each output PCM sample we sum the contribution of `WINDOW_BYTES`
//!     input bytes (a sliding window of the most recent input bytes).
//!   - Each byte (8 input bits) contributes a precomputed partial sum that
//!     depends on the byte value (0..256) and on the byte's position in the
//!     filter window (0 = oldest, WINDOW_BYTES-1 = newest).
//!   - We precompute a LUT of size `256 × WINDOW_BYTES` floats once at
//!     construction time. Runtime cost per output sample = `WINDOW_BYTES`
//!     LUT lookups + adds.
//!
//! Decimation ratio is `dsd_rate / target_pcm_rate`. For all standard DSD
//! and PCM rates (multiples of 44_100 Hz), this ratio is an integer
//! multiple of 8 — meaning each output sample consumes `ratio / 8` whole
//! bytes.
//!
//! Bit ordering: DSF stores bits LSB-first within each byte (bit 0 = first
//! sample, bit 7 = eighth sample). The LUT is built accordingly.

use std::f32::consts::PI;
use std::sync::Arc;

use rayon::prelude::*;

use crate::core::audio_decoder::error::DecodeError;

/// Default filter window in bytes. 256 bytes × 8 bits = 2048 taps total —
/// the "audiophile" preset, matched against foobar2000's DSD2PCM quality.
/// Reduced presets (128 → 1024 taps, 64 → 512 taps) are picked through
/// `AudioQualityProfile` for VM / low-CPU environments.
pub const DEFAULT_WINDOW_BYTES: usize = 256;

/// Initial value for each byte slot in the per-channel filter history.
/// `0x69 = 0b01101001` = 4 ones + 4 zeros → in DSD encoding, alternating
/// ±1 samples whose low-pass-filtered value is 0. Initializing to all
/// zeros (`0x00`) would make the filter see DC = -1.0 at startup, producing
/// a -1.0 → music step and an audible pop on the first samples.
const HISTORY_SILENCE: u8 = 0x69;

/// Per-channel filter state. Grouped in a struct so rayon can hand each
/// channel a disjoint `&mut` for parallel processing.
struct ChannelState {
    /// Rolling history (ring buffer of `window_bytes` slots).
    history: Vec<u8>,
    /// Write position (index into the ring buffer).
    position: usize,
    /// Countdown: bytes left until the next output sample.
    countdown: usize,
}

/// Stateful DSD → PCM converter.
/// One instance handles all channels (each channel keeps its own filter history).
/// The LUT is wrapped in an `Arc` so it can be shared (read-only) across
/// worker threads during parallel multichannel processing.
pub struct DsdToPcmConverter {
    /// Lookup table flattened as `lut[byte_value * window_bytes + byte_pos]`.
    lut: Arc<Vec<f32>>,
    /// Filter window length in bytes (= taps / 8). Configured at construction.
    window_bytes: usize,
    /// Number of input bytes consumed per output PCM sample (= `decimation / 8`).
    decimation_bytes: usize,
    target_pcm_rate: u32,
    /// Per-channel state (history + position + countdown), one entry per channel.
    channels: Vec<ChannelState>,
}

impl DsdToPcmConverter {
    /// Build a converter with the default (audiophile) filter window.
    pub fn new(
        dsd_rate: u32,
        target_pcm_rate: u32,
        channels: u8,
    ) -> Result<Self, DecodeError> {
        Self::new_with_window(dsd_rate, target_pcm_rate, channels, DEFAULT_WINDOW_BYTES)
    }

    /// Build a converter that maps `dsd_rate` Hz of 1-bit DSD into
    /// `target_pcm_rate` Hz of f32 PCM, for the given number of channels,
    /// using a custom filter window length (in bytes — taps = ×8).
    ///
    /// Both rates must share a base of 44_100 Hz, and the ratio
    /// `dsd_rate / target_pcm_rate` must be a positive multiple of 8.
    pub fn new_with_window(
        dsd_rate: u32,
        target_pcm_rate: u32,
        channels: u8,
        window_bytes: usize,
    ) -> Result<Self, DecodeError> {
        if target_pcm_rate == 0 || channels == 0 {
            return Err(DecodeError::InvalidConfig(
                "target_pcm_rate and channels must be > 0".into(),
            ));
        }
        if window_bytes == 0 || window_bytes > 1024 {
            return Err(DecodeError::InvalidConfig(format!(
                "window_bytes ({}) must be in 1..=1024",
                window_bytes
            )));
        }
        if dsd_rate < target_pcm_rate || dsd_rate % target_pcm_rate != 0 {
            return Err(DecodeError::InvalidConfig(format!(
                "dsd_rate ({}) must be an integer multiple of target_pcm_rate ({})",
                dsd_rate, target_pcm_rate
            )));
        }
        let decimation = (dsd_rate / target_pcm_rate) as usize;
        if decimation % 8 != 0 {
            return Err(DecodeError::InvalidConfig(format!(
                "decimation ({}) must be a multiple of 8 (was {} / {})",
                decimation, dsd_rate, target_pcm_rate
            )));
        }
        let decimation_bytes = decimation / 8;

        let n_taps = window_bytes * 8;
        let coeffs = build_filter_coefficients(n_taps, decimation);
        let lut = Arc::new(build_lut(&coeffs, window_bytes));

        let channel_states: Vec<ChannelState> = (0..channels)
            .map(|_| ChannelState {
                history: vec![HISTORY_SILENCE; window_bytes],
                position: 0,
                countdown: decimation_bytes,
            })
            .collect();

        log::debug!(
            "🎛️  DSD2PCM converter ready: {} Hz → {} Hz (decimation {}, {} taps, {} channels)",
            dsd_rate,
            target_pcm_rate,
            decimation,
            n_taps,
            channels
        );

        Ok(Self {
            lut,
            window_bytes,
            decimation_bytes,
            target_pcm_rate,
            channels: channel_states,
        })
    }

    pub fn target_pcm_rate(&self) -> u32 {
        self.target_pcm_rate
    }

    /// Feed a block of DSD bytes for a single channel.
    /// Returns the PCM samples produced (may be empty if not enough new input
    /// has accumulated since the last call).
    pub fn process_block(&mut self, channel: u8, dsd_bytes: &[u8]) -> Vec<f32> {
        let ch = channel as usize;
        process_channel(
            &mut self.channels[ch],
            &self.lut,
            self.window_bytes,
            self.decimation_bytes,
            dsd_bytes,
        )
    }

    /// Process one block per channel in parallel (rayon).
    /// `blocks[ch]` is the DSD bytes for channel `ch`. Returns one PCM Vec per channel.
    ///
    /// For 1-2 channels the sequential path is faster (rayon thread-pool
    /// overhead > parallel speedup). For 3+ channels (multichannel SACD rips)
    /// it gives a near-linear speedup, which is what makes 5.0 DSD64 playable
    /// on a single-core budget at the High profile.
    pub fn process_blocks(&mut self, blocks: &[Vec<u8>]) -> Vec<Vec<f32>> {
        // En release `par_iter_mut().zip()` prend silencieusement le plus court
        // des deux itérateurs → un canal manqué fausserait l'audio sans erreur.
        // On préfère paniquer fort plutôt que de produire du son corrompu.
        assert_eq!(
            self.channels.len(),
            blocks.len(),
            "process_blocks: blocks.len() ({}) != channels ({})",
            blocks.len(),
            self.channels.len()
        );

        if self.channels.len() <= 2 {
            // Séquentiel — overhead rayon pas justifié pour mono/stéréo
            self.channels
                .iter_mut()
                .zip(blocks.iter())
                .map(|(state, block)| {
                    process_channel(
                        state,
                        &self.lut,
                        self.window_bytes,
                        self.decimation_bytes,
                        block,
                    )
                })
                .collect()
        } else {
            let lut = self.lut.clone();
            let window_bytes = self.window_bytes;
            let decimation_bytes = self.decimation_bytes;
            self.channels
                .par_iter_mut()
                .zip(blocks.par_iter())
                .map(|(state, block)| {
                    process_channel(state, &lut, window_bytes, decimation_bytes, block)
                })
                .collect()
        }
    }

    /// Reset the filter history of all channels (call after seek).
    /// Without this, the first samples after a seek will contain stale audio.
    pub fn reset(&mut self) {
        for state in self.channels.iter_mut() {
            for b in state.history.iter_mut() {
                *b = HISTORY_SILENCE;
            }
            state.position = 0;
            state.countdown = self.decimation_bytes;
        }
    }
}

/// Decode one DSD block into PCM for a single channel. Pure function over
/// the channel state — designed to be called concurrently by rayon for
/// different channels (each has its own disjoint `&mut ChannelState`).
fn process_channel(
    state: &mut ChannelState,
    lut: &[f32],
    window_bytes: usize,
    decimation_bytes: usize,
    dsd_bytes: &[u8],
) -> Vec<f32> {
    let max_outputs = dsd_bytes.len() / decimation_bytes + 1;
    let mut output = Vec::with_capacity(max_outputs);

    for &byte in dsd_bytes {
        // 1. Write byte to the channel's ring buffer
        let pos = state.position;
        state.history[pos] = byte;
        state.position = (pos + 1) % window_bytes;

        // 2. Tick countdown; emit a PCM sample when it hits zero
        state.countdown -= 1;
        if state.countdown == 0 {
            let new_pos = state.position;
            let mut sum = 0.0f32;
            for filter_pos in 0..window_bytes {
                let byte_idx = (new_pos + filter_pos) % window_bytes;
                let byte_val = state.history[byte_idx] as usize;
                sum += lut[byte_val * window_bytes + filter_pos];
            }
            output.push(sum);
            state.countdown = decimation_bytes;
        }
    }

    output
}

// ─── Filter design ───────────────────────────────────────────────────

/// Build a windowed-sinc low-pass FIR filter, then normalise so DC gain = 1.
///
/// `n_taps` is the total number of FIR taps.
/// `decimation` is `dsd_rate / pcm_rate` (used to derive the cutoff).
fn build_filter_coefficients(n_taps: usize, decimation: usize) -> Vec<f32> {
    // Cutoff slightly below output Nyquist (≈ 0.475 × pcm_rate / dsd_rate),
    // expressed normalised against the input (DSD) sample rate.
    let fc = 0.475f32 / decimation as f32;
    let center = (n_taps as f32 - 1.0) / 2.0;

    let mut coeffs = vec![0.0f32; n_taps];
    let mut sum = 0.0f32;
    for k in 0..n_taps {
        let x = k as f32 - center;
        let sinc_arg = 2.0 * fc * x;
        let sinc_val = if sinc_arg.abs() < 1e-10 {
            1.0
        } else {
            (PI * sinc_arg).sin() / (PI * sinc_arg)
        };

        // Blackman-Harris window
        let w_arg = 2.0 * PI * k as f32 / (n_taps as f32 - 1.0);
        let window = 0.35875
            - 0.48829 * w_arg.cos()
            + 0.14128 * (2.0 * w_arg).cos()
            - 0.01168 * (3.0 * w_arg).cos();

        coeffs[k] = sinc_val * window;
        sum += coeffs[k];
    }

    // Normalise so DC gain = 1 (all-ones DSD → +1.0 PCM)
    if sum.abs() > 1e-10 {
        for c in coeffs.iter_mut() {
            *c /= sum;
        }
    }

    coeffs
}

/// Precompute the lookup table: for each (byte_value, byte_position_in_window),
/// store the partial sum of `coeff[k] × bit_value` over the 8 bits of that byte.
///
/// Bit order: LSB-first (DSF spec) — bit 0 of the byte is the oldest sample
/// inside that byte.
///
/// Bit value: 0 → -1.0, 1 → +1.0 (DSD samples are bipolar).
fn build_lut(coeffs: &[f32], window_bytes: usize) -> Vec<f32> {
    let n_taps = coeffs.len();
    debug_assert_eq!(n_taps, window_bytes * 8);

    let mut lut = vec![0.0f32; 256 * window_bytes];

    for byte_val in 0..256usize {
        for byte_pos in 0..window_bytes {
            let mut partial = 0.0f32;
            for bit in 0..8 {
                let tap_idx = byte_pos * 8 + bit;
                let bit_set = (byte_val >> bit) & 1 == 1;
                let sample = if bit_set { 1.0f32 } else { -1.0f32 };
                partial += coeffs[tap_idx] * sample;
            }
            lut[byte_val * window_bytes + byte_pos] = partial;
        }
    }

    lut
}
