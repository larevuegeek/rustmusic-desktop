//! Description of the running playback pipeline, emitted to the frontend
//! at the start of every track. Lets the UI display the source vs. output
//! signal (e.g. "DSD64 → 88.2 kHz") in the player status bar.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Sent as a Tauri event named `"playback-pipeline"` whenever a new track
/// starts decoding.
#[derive(Debug, Clone, Serialize)]
pub struct PlaybackPipelineInfo {
    // ── Source (file on disk) ──
    /// Friendly format label : `"DSD64"`, `"DSD128"`, `"FLAC"`, `"MP3"`, ...
    pub source_format: String,
    /// Native sample rate of the encoded file (`2_822_400` for DSD64,
    /// `44_100` / `96_000` / ... for PCM).
    pub source_sample_rate: u32,
    /// Native bit depth (`1` for DSD, `16` / `24` / `32` for PCM).
    /// Lossy formats (MP3, AAC, ...) report their decoded depth (`16` typ.).
    pub source_bits: u32,
    pub source_channels: u8,

    // ── Intermediate (after DSD2PCM, before resampler) ──
    /// Only set on DSD playback : rate of the PCM stream emitted by
    /// `DsdToPcmConverter`. `None` for PCM sources (no intermediate).
    pub intermediate_pcm_rate: Option<u32>,
    /// Filter taps used by the DSD2PCM converter (`Some(2048)`, `1024`,
    /// `512` according to the active quality profile). `None` for PCM sources.
    pub dsd_filter_taps: Option<u32>,
    /// DSD decimation factor (`dsd_rate / intermediate_pcm_rate`, e.g. 32
    /// for DSD64 → 88.2 kHz). `None` for PCM sources.
    pub dsd_decimation: Option<u32>,

    // ── Output (delivered to the audio backend) ──
    /// Device sample rate (after resampler, if any).
    pub output_sample_rate: u32,
    pub output_channels: u8,
    /// Human-readable name of the active output device (`"Realtek HD Audio"`,
    /// `"USB DAC"`, ...). Truncated client-side for display in the status bar.
    pub device_name: String,

    // ── Pipeline state ──
    /// `true` when a resampler is active in the chain (source rate ≠ device rate).
    pub resampler_active: bool,
    /// Active quality profile : `"high"` / `"medium"` / `"low"`.
    pub quality_profile: String,
    /// Backend audio effectif : `"CPAL shared"` ou `"WASAPI exclusive"`. Sert
    /// à afficher un badge côté frontend et à contextualiser la sortie.
    pub backend: String,
    /// `true` quand la chaîne est bit-perfect (WASAPI exclusive + pas de
    /// resampling + source rate accepté par le DAC). Sert au badge
    /// « Bit-perfect » côté frontend.
    pub bit_perfect: bool,
    /// Sortie demandée non obtenue : l'interface dit pourquoi.
    pub repli: Option<Repli>,
}

/// Pourquoi la sortie exclusive ou le DoP n'ont pas pu servir ce morceau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Repli {
    /// Le DAC refuse le format en exclusif.
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    FormatRefuse,
    /// Une autre application tient le DAC.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    AppareilOccupe,
    /// Aucun accès direct pour cette sortie (Linux : pas de carte `hw:`).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    AppareilIntrouvable,
    /// Échec d'ouverture d'une autre nature.
    Indisponible,
    /// Le DAC n'accepte pas le DoP : DSD converti en PCM.
    DopRefuse,
}

impl PlaybackPipelineInfo {
    /// Emit on the `"playback-pipeline"` channel. Errors are logged but
    /// never propagated — a missing UI status indicator is non-critical.
    pub fn emit(self, app: &AppHandle) {
        if let Err(e) = app.emit("playback-pipeline", &self) {
            log::warn!("playback-pipeline emit failed: {}", e);
        }
    }
}

/// Map a DSD raw rate (e.g. 2_822_400 Hz) to a human label "DSD64", "DSD128"...
pub fn dsd_label(rate: u32) -> String {
    if rate % 44_100 == 0 {
        format!("DSD{}", rate / 44_100)
    } else if rate % 48_000 == 0 {
        format!("DSD{} (48k base)", rate / 48_000)
    } else {
        format!("DSD {} Hz", rate)
    }
}

/// Source entière et sans perte (FLAC, ALAC, PCM entier) : décodée en f32, elle se
/// reconvertit à l'identique. Une source avec perte ou flottante ne l'est pas.
pub fn est_entier_sans_perte(codec: symphonia::core::codecs::audio::AudioCodecId) -> bool {
    use symphonia::core::codecs::audio::well_known as ids;
    [
        ids::CODEC_ID_FLAC, ids::CODEC_ID_ALAC,
        ids::CODEC_ID_PCM_S8, ids::CODEC_ID_PCM_S8_PLANAR, ids::CODEC_ID_PCM_U8, ids::CODEC_ID_PCM_U8_PLANAR,
        ids::CODEC_ID_PCM_S16LE, ids::CODEC_ID_PCM_S16LE_PLANAR, ids::CODEC_ID_PCM_S16BE, ids::CODEC_ID_PCM_S16BE_PLANAR,
        ids::CODEC_ID_PCM_U16LE, ids::CODEC_ID_PCM_U16LE_PLANAR, ids::CODEC_ID_PCM_U16BE, ids::CODEC_ID_PCM_U16BE_PLANAR,
        ids::CODEC_ID_PCM_S24LE, ids::CODEC_ID_PCM_S24LE_PLANAR, ids::CODEC_ID_PCM_S24BE, ids::CODEC_ID_PCM_S24BE_PLANAR,
        ids::CODEC_ID_PCM_U24LE, ids::CODEC_ID_PCM_U24LE_PLANAR, ids::CODEC_ID_PCM_U24BE, ids::CODEC_ID_PCM_U24BE_PLANAR,
        ids::CODEC_ID_PCM_S32LE, ids::CODEC_ID_PCM_S32LE_PLANAR, ids::CODEC_ID_PCM_S32BE, ids::CODEC_ID_PCM_S32BE_PLANAR,
    ]
    .contains(&codec)
}

/// Profondeur de la source : déclarée par le conteneur, sinon (ALAC) lue dans son magic cookie.
pub fn profondeur_source(params: &symphonia::core::codecs::audio::AudioCodecParameters) -> Option<u32> {
    use symphonia::core::codecs::audio::well_known::CODEC_ID_ALAC;
    params.bits_per_sample.or_else(|| {
        if params.codec != CODEC_ID_ALAC {
            return None;
        }
        let cookie = params.extra_data.as_ref()?;
        symphonia_common::apple::audio::alac::MagicCookie::read(cookie).ok().map(|c| u32::from(c.bit_depth))
    })
}

/// Best-effort label for a Symphonia codec id.
pub fn symphonia_format_label(codec: symphonia::core::codecs::audio::AudioCodecId) -> &'static str {
    use symphonia::core::codecs::audio::well_known as ids;
    match codec {
        c if c == ids::CODEC_ID_MP3 => "MP3",
        c if c == ids::CODEC_ID_FLAC => "FLAC",
        c if c == ids::CODEC_ID_VORBIS => "OGG Vorbis",
        c if c == ids::CODEC_ID_OPUS => "Opus",
        c if c == ids::CODEC_ID_AAC => "AAC",
        c if c == ids::CODEC_ID_ALAC => "ALAC",
        c if c == ids::CODEC_ID_PCM_S16LE
            || c == ids::CODEC_ID_PCM_S24LE
            || c == ids::CODEC_ID_PCM_F32LE =>
        {
            "WAV (PCM)"
        }
        _ => "PCM",
    }
}
