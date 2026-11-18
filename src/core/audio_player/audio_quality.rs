//! Audio quality profile.
//!
//! Drives CPU/quality trade-offs across the whole audio pipeline :
//!   - resampler (rubato) — chunk size + sub-chunks
//!   - DSD converter — filter window length + decimation target
//!   - playback ring buffer — capacity + pre-fill ratio
//!
//! The user picks one of `Auto / High / Medium / Low` in Settings. `Auto`
//! resolves at startup based on the host environment (VM ? core count ?).
//!
//! Naming convention exposed to the user :
//!   - `High`   → "Qualité maximale"
//!   - `Medium` → "Équilibré"
//!   - `Low`    → "Compatibilité"

use serde::{Deserialize, Serialize};

use crate::core::system::detect;

/// Concrete quality preset applied to the audio pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioQualityProfile {
    High,
    Medium,
    Low,
    /// Drastic mode for very limited hardware (VMs without CPU passthrough,
    /// old machines). Sacrifies audio quality to guarantee playback.
    Minimal,
}

/// What the user configured in Settings. `Auto` is resolved at runtime to
/// one of the concrete profiles via [`AudioQualitySetting::resolve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioQualitySetting {
    Auto,
    High,
    Medium,
    Low,
    Minimal,
}

impl AudioQualitySetting {
    /// Parse the value stored in the `settings` table (defaults to `Auto`).
    pub fn parse_or_auto(value: Option<&str>) -> Self {
        match value.map(str::to_ascii_lowercase).as_deref() {
            Some("high") => Self::High,
            Some("medium") => Self::Medium,
            Some("low") => Self::Low,
            Some("minimal") => Self::Minimal,
            _ => Self::Auto,
        }
    }

    /// String form for persistence.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Minimal => "minimal",
        }
    }

    /// Resolve to a concrete [`AudioQualityProfile`]. For `Auto`, runs the
    /// host-environment detection.
    pub fn resolve(self) -> AudioQualityProfile {
        match self {
            Self::High => AudioQualityProfile::High,
            Self::Medium => AudioQualityProfile::Medium,
            Self::Low => AudioQualityProfile::Low,
            Self::Minimal => AudioQualityProfile::Minimal,
            Self::Auto => detect_auto_profile(),
        }
    }
}

impl AudioQualityProfile {
    // ── Resampler (rubato FFT) parameters ─────────────────────────────

    /// Frames per FFT chunk. Bigger = lower per-chunk overhead, higher latency.
    pub fn resampler_chunk_size(self, output_rate: u32) -> usize {
        let base = match self {
            Self::High => 4096,
            Self::Medium => 8192,
            Self::Low => 16384,
            Self::Minimal => 16384,
        };
        // For very high output rates, we still want bigger chunks to keep
        // the FFT call rate sane.
        match output_rate {
            0..=48000 => base,
            48001..=96000 => base * 2,
            _ => base * 4,
        }
    }

    /// Rubato sub-chunks. More = better stopband, more CPU.
    pub fn resampler_sub_chunks(self, output_rate: u32) -> usize {
        let high_rate = output_rate >= 96000;
        match self {
            Self::High => {
                if high_rate { 8 } else { 4 }
            }
            Self::Medium => {
                if high_rate { 4 } else { 2 }
            }
            Self::Low => 2,
            Self::Minimal => 1,
        }
    }

    // ── DSD converter parameters ──────────────────────────────────────

    /// Bytes in the DSD2PCM filter window. Total taps = window × 8.
    pub fn dsd_filter_window_bytes(self) -> usize {
        match self {
            Self::High => 256,    // 2048 taps — foobar2000 grade
            Self::Medium => 128,  // 1024 taps — audibly identical for most listeners
            Self::Low => 64,      // 512 taps  — still clean, big CPU saver
            Self::Minimal => 32,  // 256 taps  — drastic compromise pour VM faible
        }
    }

    /// Target intermediate PCM rate for DSD decoding.
    ///
    /// On High/Medium we keep 4× the audible band headroom (88.2 kHz for
    /// DSD64-family rates, 96 kHz for DSD64-base-48k). On Low/Minimal we
    /// drop to CD-quality which is **half** the convolution work since the
    /// decimation factor doubles.
    pub fn dsd_target_rate(self, dsd_rate: u32) -> u32 {
        let base_44 = dsd_rate % 44_100 == 0;
        match self {
            Self::High | Self::Medium => {
                if base_44 { 88_200 } else { 96_000 }
            }
            Self::Low | Self::Minimal => {
                if base_44 { 44_100 } else { 48_000 }
            }
        }
    }

    // ── Playback ring buffer ──────────────────────────────────────────

    /// Ring buffer duration in seconds. Bigger = more tolerance to CPU stalls.
    pub fn ring_buffer_seconds(self) -> f32 {
        match self {
            Self::High => 1.0,
            Self::Medium => 1.5,
            Self::Low => 2.0,
            // Minimal : 10 secondes de buffer. Permet au décodeur d'avoir
            // 10s d'avance pour absorber des stalls CPU sévères (WebKit
            // SW rendering, scan rayon, etc.) ou des latences I/O réseau.
            // Coût RAM : ~3.5 MB à 44.1k stéréo f32, anecdotique.
            Self::Minimal => 10.0,
        }
    }

    /// Fraction of the ring buffer to pre-fill before starting CPAL.
    pub fn pre_fill_ratio(self) -> f32 {
        match self {
            Self::High => 0.25,
            Self::Medium => 0.30,
            Self::Low => 0.40,
            // Minimal : on attend 60% (~3s sur un buffer 5s) avant de
            // démarrer la lecture. Au pire ~3s de latence au play, garanti
            // sans underrun même sur VM saturée.
            Self::Minimal => 0.60,
        }
    }
}

// ─── Auto detection ──────────────────────────────────────────────────

/// Pick a profile based on the host environment.
///   - virtualised (VM/container) → `Minimal` (sécurité max, l'utilisateur peut remonter à Low/Medium si la VM tient)
///   - fewer than 4 logical cores → `Medium`
///   - otherwise                  → `High`
pub fn detect_auto_profile() -> AudioQualityProfile {
    if detect::is_virtualized() {
        return AudioQualityProfile::Minimal;
    }
    if detect::logical_cpu_count() < 4 {
        return AudioQualityProfile::Medium;
    }
    AudioQualityProfile::High
}

// ─── Global cache of the resolved profile ───────────────────────────
//
// Player threads need to know the current profile cheaply, without touching
// the DB. We cache the resolved value in an `AtomicU8` set at app startup
// (from settings) and updated whenever the user changes the setting in the UI.

use std::sync::atomic::{AtomicU8, Ordering};

static RESOLVED_PROFILE: AtomicU8 = AtomicU8::new(0); // default: High

fn profile_to_u8(p: AudioQualityProfile) -> u8 {
    match p {
        AudioQualityProfile::High => 0,
        AudioQualityProfile::Medium => 1,
        AudioQualityProfile::Low => 2,
        AudioQualityProfile::Minimal => 3,
    }
}

fn profile_from_u8(v: u8) -> AudioQualityProfile {
    match v {
        0 => AudioQualityProfile::High,
        1 => AudioQualityProfile::Medium,
        2 => AudioQualityProfile::Low,
        _ => AudioQualityProfile::Minimal,
    }
}

/// Read the currently active profile. Called from player threads at the
/// start of every track to pick up changes the user made in Settings.
pub fn current_profile() -> AudioQualityProfile {
    profile_from_u8(RESOLVED_PROFILE.load(Ordering::Relaxed))
}

/// Update the active profile. Called once at app boot after reading the
/// setting, and again every time the user changes it in Settings.
pub fn set_current_profile(p: AudioQualityProfile) {
    RESOLVED_PROFILE.store(profile_to_u8(p), Ordering::Relaxed);
}
