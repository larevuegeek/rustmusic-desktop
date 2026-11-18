//! Qualité audio d'un morceau : palier, libellés DSD, format et débit.

use crate::models::tracks::*;
use karbon_ui::intl;

/// « FLAC 24/96 », « DSD64 », « MP3 ».
pub fn short_format(format: Option<&str>, bits: Option<i32>, rate: Option<i32>) -> String {
    let f = format.unwrap_or("").to_uppercase();
    if bits == Some(1) || f == "DSF" || f == "DFF" {
        return crate::helper::audio::dsd_label(rate.map_or(0, |r| r.max(0) as u32));
    }
    let (Some(bits), Some(rate)) = (bits.filter(|b| *b > 0), rate.filter(|r| *r > 0)) else {
        return f;
    };
    let khz = if rate % 1000 != 0 {
        format!("{:.1}", rate as f64 / 1000.).replace('.', ",")
    } else {
        (rate / 1000).to_string()
    };
    format!("{f} {bits}/{khz}")
}


pub fn tier(format: Option<&str>, bits: Option<i32>, rate: Option<i32>) -> Tier {
    if bits.unwrap_or(0) > 16 || rate.unwrap_or(0) > 48_000 {
        return Tier::Hires;
    }
    const LOSSLESS: [&str; 11] = ["FLAC", "ALAC", "WAV", "AIFF", "AIF", "APE", "WV", "WAVPACK", "TTA", "DSF", "DFF"];
    if LOSSLESS.contains(&format.unwrap_or("").to_uppercase().as_str()) {
        Tier::Lossless
    } else {
        Tier::Lossy
    }
}

/// « 833 kb/s », « 1,4 Mb/s ».
pub fn format_bitrate(kbps: Option<i32>) -> String {
    match kbps {
        Some(k) if k >= 1000 => format!("{:.1} Mb/s", k as f64 / 1000.).replace('.', ","),
        Some(k) if k > 0 => format!("{} kb/s", intl::number(k as i64)),
        _ => "—".into(),
    }
}
