//! Les sorties audio : description d'un appareil et liste des sorties.

use crate::models::home::*;
use crate::models::app::Data;
use crate::entity::audio::audio_file::AudioFormat;
use crate::commands::network::network_player_command::DeviceView;

pub(crate) fn is_dsd(format: &AudioFormat) -> bool {
    matches!(format, AudioFormat::DSF | AudioFormat::DFF)
}


/// Nom lisible, détail et icône d'une sortie.
pub(crate) fn describe_output(d: &crate::commands::audio::device_command::AudioDeviceInfo) -> Output {
    fn split_name(name: &str) -> (String, Option<String>) {
        if !name.ends_with(')') {
            return (name.to_string(), None);
        }
        let mut depth = 0;
        for (i, c) in name.char_indices().rev() {
            if c == ')' {
                depth += 1;
            } else if c == '(' {
                depth -= 1;
                if depth == 0 {
                    return (name[..i].trim().to_string(), Some(name[i + 1..name.len() - 1].trim().to_string()));
                }
            }
        }
        (name.to_string(), None)
    }
    let lower = d.name.to_lowercase();
    let screen = ["hdmi", "displayport", "nvidia high definition audio", "amd high definition audio", "intel(r) display audio"]
        .iter()
        .any(|w| lower.contains(w));
    let digital = ["digital output", "spdif", "s/pdif", "optical", "optique"].iter().any(|w| lower.contains(w));
    let (mut before, mut inside) = split_name(&d.name);
    if inside.is_none() && d.name.contains(", ") {
        let (a, b) = d.name.split_once(", ").unwrap();
        before = a.to_string();
        inside = Some(b.to_string());
    }
    let device = inside.map(|s| s.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == ' ').to_string());
    let roles = [
        "haut-parleurs", "haut-parleur", "speakers", "speaker", "casque", "casques", "casque audio",
        "headphones", "headphone", "headset", "écouteurs", "earphones", "lautsprecher", "kopfhörer",
        "altavoces", "auriculares", "altoparlanti", "cuffie", "auricolari",
    ];
    let role = device.is_some() && roles.contains(&before.to_lowercase().as_str());
    let detail = if role { before.clone() } else { device.clone().unwrap_or_default() };
    let detail = if detail.is_empty() {
        d.manufacturer.clone().or_else(|| d.driver.clone()).unwrap_or_default()
    } else {
        detail
    };
    let bus = d.bus.as_deref().unwrap_or("").to_uppercase();
    let link = if screen {
        Some("HDMI")
    } else if digital {
        Some("S/PDIF")
    } else if bus.starts_with("BTH") {
        Some("Bluetooth")
    } else if bus == "USB" || lower.contains("usb") {
        Some("USB")
    } else {
        None
    };
    let detail = match link {
        Some(l) if !detail.to_lowercase().contains(&l.to_lowercase()) => {
            if detail.is_empty() { l.to_string() } else { format!("{detail} · {l}") }
        }
        _ => detail,
    };
    let headphones = ["casque", "headphone", "headset", "écouteur", "earphone", "kopfhörer", "auricular", "cuffi", "auricolar"]
        .iter()
        .any(|w| before.to_lowercase().contains(w));
    Output {
        name: d.name.clone(),
        label: if role { device.unwrap_or_default() } else { before },
        detail,
        icon: if screen {
            "material-symbols:tv-outline-rounded"
        } else if headphones {
            "material-symbols:headphones-outline-rounded"
        } else {
            "material-symbols:speaker-outline-rounded"
        },
        hires: d.is_hires,
    }
}

pub(crate) fn outputs(data: &Data) -> (Vec<Output>, usize) {
    let outputs: Vec<Output> = data.devices.iter().map(describe_output).collect();
    let selected = data
        .devices
        .iter()
        .position(|d| d.is_selected)
        .or_else(|| data.devices.iter().position(|d| d.is_default))
        .unwrap_or(0);
    (outputs, selected)
}

/// Les lecteurs réseau du menu, dans l'ordre affiché : ceux de la dernière recherche,
/// plus l'appareil choisi s'il n'a pas répondu cette fois.
pub(crate) fn network_devices(data: &Data) -> Vec<DeviceView> {
    let mut devices = data.network.devices.clone();
    if let Some(selected) = &data.network.selected {
        if !devices.iter().any(|d| d.id == selected.id) {
            devices.push(selected.clone());
        }
    }
    devices
}

/// Les lecteurs réseau comme des sorties, et l'index de celui qui joue.
pub(crate) fn network_outputs(data: &Data) -> (Vec<Output>, Option<usize>) {
    let devices = network_devices(data);
    let selected = data.network.selected.as_ref().and_then(|s| devices.iter().position(|d| d.id == s.id));
    let outputs = devices
        .iter()
        .map(|d| {
            let brand = if d.kind == "airplay" { "AirPlay" } else { d.manufacturer.as_str() };
            let detail = [brand, d.model.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
            Output {
                name: d.id.clone(),
                label: d.name.clone(),
                detail: if detail.is_empty() { d.ip.clone() } else { detail },
                icon: network_icon(d.kind),
                hires: false,
            }
        })
        .collect();
    (outputs, selected)
}

/// Icône d'un lecteur réseau selon sa famille.
pub(crate) fn network_icon(kind: &str) -> &'static str {
    if kind == "airplay" {
        "material-symbols-light:airplay-rounded"
    } else {
        "material-symbols-light:cast-rounded"
    }
}
