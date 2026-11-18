use crate::models::settings::*;
use karbon_ui::intl;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::settings::OutputCaps;
use crate::mapper::settings::format_rate;

pub(crate) fn bit_tone(f: &str) -> Tone {
    if f.contains("32-bit") || f.contains("24-bit") {
        0
    } else if f.contains("16-bit") {
        1
    } else {
        2
    }
}

pub(crate) fn rate_tone(hz: u32) -> Tone {
    if hz >= 352_800 {
        3
    } else if hz >= 88_200 {
        4
    } else {
        2
    }
}

pub(crate) fn rate_category(hz: u32) -> String {
    t(if hz >= 352_800 {
        "settings.audio_devices_rate_dsd_pcm"
    } else if hz >= 88_200 {
        "settings.audio_devices_rate_hires"
    } else {
        "settings.audio_devices_rate_cd"
    })
}

/// Le message d'un sondage échoué, pour le gabarit.
pub fn failed(b: &Banner) -> Option<String> {
    match b {
        Banner::Failed(e) => Some(e.clone()),
        _ => None,
    }
}

impl MusicApp {
    /// La fiche de la sortie ouverte.
    pub fn device_view(&self) -> Option<DeviceView> {
        let dd = self.views.device_details.as_ref()?;
        let d = self.data.devices.get(dd.index)?;
        let caps: Option<&OutputCaps> = self.views.settings.audio.caps.get(&d.display_name);
        let probed = caps.is_some_and(|c| !c.rates.is_empty() || !c.bits.is_empty());
        let banner = if dd.probing {
            Banner::Probing
        } else if probed {
            Banner::Probed
        } else if caps.is_some() {
            Banner::NoExclusive
        } else if let Some(e) = &dd.error {
            Banner::Failed(e.clone())
        } else if d.wasapi_id.is_none() {
            Banner::CpalOnly
        } else {
            Banner::None
        };
        let rates: Vec<u32> = match caps {
            Some(c) if probed => c.rates.clone(),
            _ => d.sample_rates.clone(),
        };
        let formats: Vec<String> = match caps {
            Some(c) if probed => c.bits.iter().map(|b| format!("{b}-bit int")).collect(),
            _ => d.sample_formats.clone(),
        };
        let mut groups: Vec<(String, Vec<(String, Tone)>)> = Vec::new();
        for hz in rates {
            let cat = rate_category(hz);
            match groups.iter_mut().find(|g| g.0 == cat) {
                Some(g) => g.1.push((format_rate(hz), rate_tone(hz))),
                None => groups.push((cat, vec![(format_rate(hz), rate_tone(hz))])),
            }
        }
        let mix = caps.and_then(|c| {
            let r = c.mix_rate?;
            let mut s = format_rate(r);
            if let Some(b) = c.mix_bits {
                s += &format!(" · {b}-bit");
            }
            if let Some(ch) = c.mix_channels {
                s += &format!(" · {ch} ch");
            }
            Some(s)
        });
        let fr = intl::number;
        let active = self.data.devices.iter().position(|x| x.is_selected).or_else(|| self.data.devices.iter().position(|x| x.is_default));
        Some(DeviceView {
            title: d.display_name.clone(),
            icon: if d.is_hires { "lucide:zap" } else { "lucide:speaker" },
            is_default: d.is_default,
            is_hires: d.is_hires,
            manufacturer: d.manufacturer.clone().filter(|m| !m.is_empty()),
            raw_name: d.name.clone(),
            driver: d.driver.clone().filter(|m| !m.is_empty()),
            channels: format!("{} ch{}", d.max_channels, if d.max_channels >= 2 { " (max)" } else { "" }),
            banner,
            mix,
            formats_title: t(if probed { "settings.audio_devices_formats_exclusive" } else { "settings.audio_devices_formats" }),
            formats: formats.iter().map(|f| (f.clone(), bit_tone(f))).collect(),
            rates_title: t(if probed { "settings.audio_devices_sample_rates_exclusive" } else { "settings.audio_devices_sample_rates" }),
            rate_groups: groups,
            buffer: match (d.min_buffer_frames, d.max_buffer_frames) {
                (Some(a), Some(b)) => Some(format!("{} – {} frames", fr(a as i64), fr(b as i64))),
                _ => None,
            },
            active: active == Some(dd.index),
        })
    }
}
