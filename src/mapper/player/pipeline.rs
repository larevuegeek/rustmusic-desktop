//! La chaîne audio du morceau en cours : son mode et ses étapes, du fichier au DAC, telles
//! que le moteur les décrit (`playback-pipeline`).

use crate::app::music_app::MusicApp;
use crate::models::player::{PipelineStep, PipelineView, StepTone};
use karbon_ui::intl::{self, t};
use serde_json::Value;

/// Mode de la chaîne : DSD natif (DoP), DSD converti en PCM, rééchantillonnée,
/// bit-perfect (volume à 100 %), exclusive, ou partagée.
pub(crate) fn chain_mode(info: &Value, volume: u8) -> &'static str {
    let backend = info.get("backend").and_then(Value::as_str).unwrap_or("");
    let flag = |key: &str| info.get(key).and_then(Value::as_bool).unwrap_or(false);
    if backend.ends_with("DoP") {
        "dop"
    } else if info.get("intermediate_pcm_rate").is_some_and(|v| !v.is_null()) {
        "dsd"
    } else if flag("resampler_active") {
        "resampled"
    } else if flag("bit_perfect") && volume >= 100 {
        "bit-perfect"
    } else if backend.contains("exclusive") {
        "exclusive"
    } else {
        "shared"
    }
}

/// « 44,1 kHz », « 2,82 MHz ».
pub(crate) fn readable_rate(hz: u64) -> String {
    if hz >= 1_000_000 {
        format!("{} MHz", intl::decimal(hz as f64 / 1_000_000., 2))
    } else if hz >= 1_000 {
        format!("{} kHz", intl::decimal(hz as f64 / 1_000., 2))
    } else {
        format!("{hz} Hz")
    }
}

/// « Mono », « Stéréo », « 5.1 »…
fn channels_label(n: u64) -> String {
    match n {
        1 => t("pipeline.channels_mono"),
        2 => t("pipeline.channels_stereo"),
        3 => "3.0".into(),
        4 => "4.0".into(),
        5 => "5.0".into(),
        6 => "5.1".into(),
        7 => "6.1".into(),
        8 => "7.1".into(),
        n => format!("{n} ch"),
    }
}

/// Ce qui n'a pas marché pour ce morceau, en clair.
fn fallback_message(info: &Value) -> Option<String> {
    if let Some(reason) = info.get("repli").and_then(Value::as_str) {
        return Some(t(&format!("pipeline.fallback_{reason}")));
    }
    let number = |key: &str| info.get(key).and_then(Value::as_u64);
    let backend = info.get("backend").and_then(Value::as_str).unwrap_or("");
    let resampled = info.get("resampler_active").and_then(Value::as_bool).unwrap_or(false);
    // Exclusif obtenu, mais à une autre fréquence que celle du fichier.
    (backend.contains("exclusive") && resampled && number("intermediate_pcm_rate").is_none()).then(|| {
        t("pipeline.fallback_rate")
            .replace("{source}", &readable_rate(number("source_sample_rate").unwrap_or(0)))
            .replace("{output}", &readable_rate(number("output_sample_rate").unwrap_or(0)))
    })
}

/// Les étapes de la chaîne ; `device` est le nom lisible de la sortie.
pub(crate) fn pipeline_steps(info: &Value, device: String) -> Vec<PipelineStep> {
    let text = |key: &str| info.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let number = |key: &str| info.get(key).and_then(Value::as_u64);
    let backend = text("backend");
    let dop = backend.ends_with("DoP");
    let intermediate = number("intermediate_pcm_rate");
    let source_bits = number("source_bits").unwrap_or(0);
    let bits = if intermediate.is_some() || dop {
        Some("1 bit".to_string())
    } else {
        (source_bits > 0).then(|| format!("{source_bits} bit"))
    };
    let source_rate = number("source_sample_rate").unwrap_or(0);
    let output_rate = number("output_sample_rate").unwrap_or(0);
    let mut steps = vec![PipelineStep {
        icon: "material-symbols:audio-file-outline-rounded",
        title: t("pipeline.source"),
        value: [Some(text("source_format")), Some(readable_rate(source_rate)), bits]
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · "),
        detail: Some(channels_label(number("source_channels").unwrap_or(2))),
        mono: true,
        tone: StepTone::Neutral,
    }];
    if let Some(pcm) = intermediate {
        let detail = [
            number("dsd_filter_taps").map(|taps| t("pipeline.dsd_filter").replace("{taps}", &taps.to_string())),
            number("dsd_decimation").map(|d| format!("×{d}")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        steps.push(PipelineStep {
            icon: "material-symbols:tune-rounded",
            title: t("pipeline.decoding"),
            value: format!("DSD → PCM {}", readable_rate(pcm)),
            detail: (!detail.is_empty()).then_some(detail),
            mono: true,
            tone: StepTone::Amber,
        });
    }
    if dop {
        steps.push(PipelineStep {
            icon: "material-symbols:swap-horiz-rounded",
            title: t("pipeline.transport"),
            value: format!("DoP {}", readable_rate(output_rate)),
            detail: Some(t("pipeline.dop_transport_hint")),
            mono: true,
            tone: StepTone::Violet,
        });
    }
    if info.get("resampler_active").and_then(Value::as_bool).unwrap_or(false) {
        steps.push(PipelineStep {
            icon: "material-symbols:graphic-eq-rounded",
            title: t("pipeline.resampling"),
            value: format!("{} → {}", readable_rate(intermediate.unwrap_or(source_rate)), readable_rate(output_rate)),
            detail: None,
            mono: true,
            tone: StepTone::Amber,
        });
    }
    // « WASAPI DoP » → « WASAPI · DoP (DSD natif) ».
    let engine = if dop {
        format!("{} · {}", backend.trim_end_matches(" DoP"), t("pipeline.dop_backend"))
    } else if backend.is_empty() {
        "CPAL shared".to_string()
    } else {
        backend
    };
    steps.push(PipelineStep {
        icon: "material-symbols-light:speaker-outline-rounded",
        title: t("pipeline.output"),
        value: device,
        detail: Some([readable_rate(output_rate), channels_label(number("output_channels").unwrap_or(2)), engine].join(" · ")),
        mono: false,
        tone: StepTone::Neutral,
    });
    steps
}

impl MusicApp {
    /// Le moteur a décrit la chaîne du morceau en cours : son détail peut s'ouvrir.
    pub fn chain_available(&self) -> bool {
        self.data.player.pipeline.is_some()
    }

    /// Le détail de la chaîne audio du morceau en cours.
    pub fn pipeline_view(&self) -> Option<PipelineView> {
        let info = self.data.player.pipeline.as_ref()?;
        let mode = chain_mode(info, self.data.volume);
        let (label, description) = crate::mapper::settings::badge_keys(mode);
        let raw = info.get("device_name").and_then(Value::as_str).unwrap_or("");
        let device = self
            .data
            .devices
            .iter()
            .find(|d| d.name == raw)
            .map(|d| crate::mapper::player::outputs::describe_output(d).label)
            .unwrap_or_else(|| raw.to_string());
        let profile = info
            .get("quality_profile")
            .and_then(Value::as_str)
            .map(|p| match p {
                "high" | "medium" | "low" | "minimal" => t(&format!("settings.audio_quality_{p}")),
                other => other.to_string(),
            })
            .filter(|p| !p.is_empty());
        let label = t(label);
        let fallback = fallback_message(info);
        let steps = pipeline_steps(info, device);
        Some(PipelineView {
            mode,
            clipboard: chain_text(&label, fallback.as_deref(), &steps, profile.as_deref()),
            summary: chain_summary(info),
            label,
            description: t(description),
            fallback,
            steps,
            profile,
        })
    }
}

/// « DSD64 · 2,82 MHz → 48 kHz », « FLAC · 44,1 kHz » : le format du fichier, sa fréquence
/// et celle de la sortie quand elle diffère.
pub(crate) fn chain_summary(info: &Value) -> String {
    let format = info.get("source_format").and_then(Value::as_str).unwrap_or("");
    let source = info.get("source_sample_rate").and_then(Value::as_u64).unwrap_or(0);
    let output = info.get("output_sample_rate").and_then(Value::as_u64).unwrap_or(0);
    let mut summary = [format.to_string(), readable_rate(source)]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    if output > 0 && output != source {
        summary.push_str(&format!(" → {}", readable_rate(output)));
    }
    summary
}

/// Le détail en lignes de texte : état, repli, étapes, profil.
pub(crate) fn chain_text(label: &str, fallback: Option<&str>, steps: &[PipelineStep], profile: Option<&str>) -> String {
    let mut lines = vec![format!("{} : {label}", t("player.audio_chain"))];
    lines.extend(fallback.map(String::from));
    for step in steps {
        let detail = step.detail.as_deref().map(|d| format!(" ({d})")).unwrap_or_default();
        lines.push(format!("{} : {}{detail}", step.title, step.value));
    }
    lines.extend(profile.map(|p| format!("{} : {p}", t("pipeline.profile"))));
    lines.join("\n")
}

#[cfg(test)]
mod tests;
