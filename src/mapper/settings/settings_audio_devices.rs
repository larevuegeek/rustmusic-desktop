//! Réglages · Audio, les sorties : la liste (une ligne par sortie, palier et
//! caractéristiques) et le test du DAC (fréquences acceptées en exclusif, DSD en DoP).

use crate::models::settings::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::settings::Ghost;
use crate::models::settings::SettingsMsg;

pub fn format_rate(hz: u32) -> String {
    if hz >= 1_000_000 {
        format!("{:.2} MHz", hz as f64 / 1_000_000.0)
    } else if hz >= 1_000 {
        if hz % 1000 == 0 {
            format!("{} kHz", hz / 1000)
        } else {
            format!("{:.1} kHz", hz as f64 / 1000.0)
        }
    } else {
        format!("{hz} Hz")
    }
}

impl MusicApp {
    /// Une ligne par sortie, la sortie en cours cochée.
    pub fn audio_outputs(&self) -> Vec<OutputRow> {
        let active = self.data.devices.iter().position(|d| d.is_selected).or_else(|| self.data.devices.iter().position(|d| d.is_default));
        self.data
            .devices
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let s = crate::mapper::player::outputs::describe_output(d);
                let (rate, bits, channels) = match self.views.settings.audio.caps.get(&d.display_name) {
                    Some(c) if !c.rates.is_empty() => (c.rates.iter().max().copied(), c.bits.iter().max().copied(), d.max_channels),
                    Some(c) if c.mix_rate.is_some() => (c.mix_rate, c.mix_bits, c.mix_channels.unwrap_or(d.max_channels)),
                    _ => (
                        d.sample_rates.iter().max().copied(),
                        d.sample_formats.iter().filter_map(|f| f.split(|c: char| !c.is_ascii_digit()).next()?.parse::<u16>().ok()).max(),
                        d.max_channels,
                    ),
                };
                let tier = rate.map(|r| {
                    if r < 88_200 || bits.is_some_and(|b| b < 24) {
                        (t("settings.audio_devices_rate_cd"), "neutral")
                    } else if r >= 352_800 {
                        (t("settings.audio_devices_rate_dsd_pcm"), "purple")
                    } else {
                        (t("settings.audio_devices_rate_hires"), "amber")
                    }
                });
                let mut specs: Vec<String> = Vec::new();
                specs.extend(rate.map(format_rate));
                specs.extend(bits.map(|b| format!("{b} bit")));
                specs.push(format!("{channels} ch"));
                OutputRow {
                    index: i,
                    title: d.display_name.clone(),
                    name: s.label,
                    detail: s.detail,
                    icon: s.icon,
                    active: active == Some(i),
                    default_value: d.is_default,
                    tier,
                    specs,
                }
            })
            .collect()
    }

    /// Le badge « Par défaut (Windows) ».
    pub fn default_os_label(&self) -> String {
        let system = if cfg!(target_os = "macos") { "macOS" } else if cfg!(target_os = "linux") { "Linux" } else { "Windows" };
        t("settings.audio_devices_default_os").replace("{os}", system)
    }

    /// `DacTest` : description, libellé du bouton, fréquences et DSD testés.
    pub fn dac_desc(&self) -> String {
        let d = self.data.devices.iter().find(|d| d.is_selected).or_else(|| self.data.devices.iter().find(|d| d.is_default));
        match d {
            Some(d) => t("settings.dac_test_desc").replace("{device}", &crate::mapper::player::outputs::describe_output(d).name),
            None => t("settings.dac_test_none"),
        }
    }

    pub fn dac_button(&self) -> Ghost {
        let a = &self.views.settings.audio;
        let label = if a.dac_running {
            t("settings.dac_test_running")
        } else if a.dac_caps.is_some() {
            t("settings.dac_test_again")
        } else {
            t("settings.dac_test_run")
        };
        let without_id = !self.data.devices.iter().any(|d| (d.is_selected || d.is_default) && d.wasapi_id.is_some());
        Ghost::new("material-symbols:science-outline-rounded", label, Msg::Settings(SettingsMsg::DacTest)).busy(a.dac_running).disabled(without_id)
    }

    pub fn dac_rates(&self) -> Option<(Vec<DacRate>, Option<String>)> {
        let c = self.views.settings.audio.dac_caps.as_ref()?;
        let rates = [44_100, 48_000, 88_200, 96_000, 176_400, 192_000, 352_800, 384_000, 705_600, 768_000]
            .into_iter()
            .map(|f| DacRate { label: format_rate(f), ok: c.rates.contains(&f) })
            .collect();
        let dsd: Vec<&str> = if c.bits.iter().any(|b| *b >= 24) {
            [("DSD64", 176_400), ("DSD128", 352_800), ("DSD256", 705_600)].into_iter().filter(|(_, r)| c.rates.contains(r)).map(|(d, _)| d).collect()
        } else {
            Vec::new()
        };
        Some((rates, (!dsd.is_empty()).then(|| dsd.join(" · "))))
    }
}
