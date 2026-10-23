//! Retrouver la sortie CPAL qui correspond au nom enregistré. Sous Linux, les
//! sorties portent les noms du serveur son ; les anciens noms ALSA
//! (« Carte, Pilote ») sont retrouvés par le nom de carte.

use cpal::traits::{DeviceTrait, HostTrait};

/// Nom brut et nom affiché (avec le fabricant quand il est connu).
fn names(device: &cpal::Device) -> Option<(String, String)> {
    let desc = device.description().ok()?;
    let name = desc.name().to_string();
    let display = match desc.manufacturer() {
        Some(maker) => format!("{name} ({maker})"),
        None => name.clone(),
    };
    Some((name, display))
}

/// Nom complet « Nom (Fabricant) », celui que l'interface enregistre.
pub fn full_name(device: &cpal::Device) -> Option<String> {
    let d = device.description().ok()?;
    let name = d.name().to_string();
    Some(match (d.manufacturer(), d.driver()) {
        (Some(mfr), _) => format!("{name} ({mfr})"),
        (_, Some(drv)) => format!("{name} ({drv})"),
        _ => name,
    })
}

pub fn find(host: &cpal::Host, name: &str) -> Option<cpal::Device> {
    let mut outputs: Vec<cpal::Device> = host.output_devices().ok()?.collect();
    if let Some(i) = outputs
        .iter()
        .position(|d| names(d).is_some_and(|(raw, display)| raw == name || display == name))
    {
        return Some(outputs.swap_remove(i));
    }

    #[cfg(target_os = "linux")]
    if let Some(i) = legacy_alsa_card(name).and_then(|card| {
        outputs
            .iter()
            .position(|d| names(d).is_some_and(|(raw, _)| raw.to_lowercase().contains(&card)))
    }) {
        return Some(outputs.swap_remove(i));
    }

    None
}

/// La sortie choisie, ou celle du système. Sous Linux, une sortie disparaît aussi
/// pendant une lecture exclusive : le chemin partagé la reprend avec
/// [`reclaim_if_held`].
pub fn choose(host: &cpal::Host, name: Option<&str>) -> Result<cpal::Device, String> {
    if let Some(name) = name {
        match find(host, name) {
            Some(device) => return Ok(device),
            #[cfg(target_os = "linux")]
            None if held_card(name).is_some() => log::info!(
                "Sortie '{name}' hors service le temps de la lecture exclusive : sortie système pour la configuration"
            ),
            // Sortie débranchée : repli normal sur celle du système, signalé une fois (pas à chaque morceau).
            None => {
                static WARNED: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
                let mut last = WARNED.lock().unwrap_or_else(|e| e.into_inner());
                if last.as_deref() != Some(name) {
                    log::warn!("Sortie '{name}' absente (débranchée ?) : sortie du système à la place");
                    *last = Some(name.to_string());
                }
            }
        }
    }
    host.default_output_device()
        .ok_or_else(|| "Pas de périphérique audio".to_string())
}

/// Le même périphérique, pris dans la liste des sorties. Celui de `default_output_device`
/// ouvre une unité qui suit la sortie système ; sous hog mode il cesse de l'être et
/// CoreAudio refuse le flux (`!obj`). L'exclusif doit viser le périphérique lui-même.
#[cfg(target_os = "macos")]
pub fn pin_device(device: &cpal::Device) -> cpal::Device {
    let Ok(id) = device.id() else { return device.clone() };
    cpal::default_host()
        .output_devices()
        .ok()
        .and_then(|mut sorties| sorties.find(|d| d.id().is_ok_and(|i| i == id)))
        .unwrap_or_else(|| device.clone())
}

/// Période des flux partagés sur le serveur son, en trames. Sans elle, le serveur
/// prend 2 s d'audio d'un coup (pause, volume et sauts en retard d'autant).
pub fn sound_server_period(host: &cpal::Host, rate: u32) -> Option<u32> {
    #[cfg(target_os = "linux")]
    if host.id() == cpal::HostId::PulseAudio {
        return Some(rate / 20);
    }
    let _ = (host, rate);
    None
}

/// Sortie retirée à PipeWire par notre lecture exclusive (elle reviendra après).
#[cfg(target_os = "linux")]
pub fn is_held(name: &str) -> bool {
    held_card(name).is_some()
}

#[cfg(target_os = "linux")]
fn held_card(name: &str) -> Option<u32> {
    use super::{device_reservation::card_index_from_hw_id, dop_alsa::resolve_hw_id, pipewire_profile};
    let card = card_index_from_hw_id(&resolve_hw_id(name)?)?;
    pipewire_profile::is_held(card).then_some(card)
}

/// Rend à PipeWire la carte qu'une lecture exclusive a laissée hors service, puis
/// attend que sa sortie réapparaisse.
#[cfg(target_os = "linux")]
pub fn reclaim_if_held(name: &str) -> Option<cpal::Device> {
    let card = held_card(name)?;
    if !super::pipewire_profile::restore_now(card) {
        return None;
    }
    let host = cpal::default_host();
    for _ in 0..30 {
        if let Some(device) = find(&host, name) {
            log::info!("Sortie '{name}' rendue à PipeWire pour la lecture partagée");
            return Some(device);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    log::warn!("Sortie '{name}' toujours absente après son retour à PipeWire");
    None
}

/// « Fosi Audio K7, USB Audio » → « fosi audio k7 ».
#[cfg(target_os = "linux")]
fn legacy_alsa_card(name: &str) -> Option<String> {
    let (card, _) = name.split_once(", ")?;
    let card = card.trim();
    (!card.is_empty()).then(|| card.to_lowercase())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn reads_card_from_legacy_name() {
        assert_eq!(legacy_alsa_card("Fosi Audio K7, USB Audio").as_deref(), Some("fosi audio k7"));
        assert_eq!(legacy_alsa_card("HD-Audio Generic, ALC269VC Analog").as_deref(), Some("hd-audio generic"));
        assert_eq!(legacy_alsa_card("Fosi Audio K7 Stéréo analogique"), None);
    }
}
