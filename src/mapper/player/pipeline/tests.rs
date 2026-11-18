use super::{chain_mode, chain_summary, chain_text, pipeline_steps, readable_rate};
use crate::models::player::StepTone;
use serde_json::json;

#[test]
fn modes_follow_the_engine_description() {
    let shared = json!({ "backend": "CPAL shared" });
    assert_eq!(chain_mode(&shared, 100), "shared");
    let exact = json!({ "backend": "WASAPI exclusive", "bit_perfect": true });
    assert_eq!(chain_mode(&exact, 100), "bit-perfect");
    // Volume logiciel : le signal n'est plus intact.
    assert_eq!(chain_mode(&exact, 80), "exclusive");
    assert_eq!(chain_mode(&json!({ "resampler_active": true }), 100), "resampled");
    assert_eq!(chain_mode(&json!({ "intermediate_pcm_rate": 88200 }), 100), "dsd");
    assert_eq!(chain_mode(&json!({ "backend": "WASAPI DoP" }), 100), "dop");
}

#[test]
fn rates_read_in_khz_or_mhz() {
    assert!(readable_rate(44_100).ends_with(" kHz"));
    assert!(readable_rate(2_822_400).ends_with(" MHz"));
    assert_eq!(readable_rate(800), "800 Hz");
}

#[test]
fn a_resampled_dsd_track_lists_every_step_from_file_to_dac() {
    let info = json!({
        "source_format": "DSD64", "source_sample_rate": 2_822_400, "source_bits": 1, "source_channels": 2,
        "intermediate_pcm_rate": 88_200, "dsd_filter_taps": 2048, "dsd_decimation": 32,
        "output_sample_rate": 48_000, "output_channels": 2, "resampler_active": true, "backend": "CPAL shared",
    });
    let steps = pipeline_steps(&info, "DAC".into());
    let tones: Vec<_> = steps.iter().map(|s| s.tone).collect();
    assert_eq!(tones, [StepTone::Neutral, StepTone::Amber, StepTone::Amber, StepTone::Neutral]);
    assert!(steps[0].value.starts_with("DSD64"));
    assert!(steps[0].value.ends_with("1 bit"));
    assert!(steps[1].value.starts_with("DSD → PCM"));
    assert_eq!(steps[3].value, "DAC");
}

#[test]
fn a_pcm_track_on_its_native_rate_has_a_source_and_an_output() {
    let info = json!({
        "source_format": "FLAC", "source_sample_rate": 44_100, "source_bits": 16, "source_channels": 2,
        "output_sample_rate": 44_100, "output_channels": 2, "backend": "WASAPI exclusive",
    });
    let steps = pipeline_steps(&info, "Fosi K7".into());
    assert_eq!(steps.len(), 2);
    assert!(steps[0].value.contains("16 bit"));
    assert!(steps[1].detail.as_deref().is_some_and(|d| d.ends_with("WASAPI exclusive")));
}

#[test]
fn the_summary_shows_the_output_rate_only_when_it_differs() {
    let dsd = json!({ "source_format": "DSD64", "source_sample_rate": 2_822_400, "output_sample_rate": 48_000 });
    let summary = chain_summary(&dsd);
    assert!(summary.starts_with("DSD64 · ") && summary.contains(" MHz → ") && summary.ends_with(" kHz"), "{summary}");
    let native = json!({ "source_format": "FLAC", "source_sample_rate": 44_100, "output_sample_rate": 44_100 });
    assert!(!chain_summary(&native).contains('→'));
}

#[test]
fn the_copied_text_lists_the_state_every_step_and_the_profile() {
    let info = json!({
        "source_format": "FLAC", "source_sample_rate": 44_100, "source_bits": 16, "source_channels": 2,
        "output_sample_rate": 44_100, "output_channels": 2, "backend": "WASAPI exclusive",
    });
    let steps = pipeline_steps(&info, "Fosi K7".into());
    let text = chain_text("Bit-perfect", None, &steps, Some("Qualité maximale"));
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 4);
    assert!(lines[0].ends_with("Bit-perfect"));
    assert!(lines[2].contains("Fosi K7"));
    assert!(lines[3].ends_with("Qualité maximale"));
}
