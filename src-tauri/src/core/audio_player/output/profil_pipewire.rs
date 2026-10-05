//! Carte mise hors service côté PipeWire (profil « off ») pendant un accès
//! exclusif. Contrairement à la réservation D-Bus, la carte n'est pas recréée :
//! WirePlumber 0.4 ne la renomme pas et la sortie par défaut est conservée.

#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::Duration;

use serde_json::Value;

const PROFILE_OFF: &str = "off";
/// La sonde, le rendu et les pistes suivantes réutilisent la prise pendant ce délai.
const GRACE: Duration = Duration::from_secs(5);

struct Hold {
    count: u32,
    /// `None` si la carte était déjà hors service ou inconnue de PipeWire.
    restore: Option<Restore>,
    generation: u64,
}

struct Restore {
    /// `device.name` PipeWire : stable, contrairement à l'index ALSA.
    device: String,
    profile: String,
}

struct Card {
    id: u32,
    device: String,
    profile: String,
    profiles: Vec<(u32, String)>,
}

static HOLDS: LazyLock<Mutex<HashMap<u32, Hold>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static GENERATION: AtomicU64 = AtomicU64::new(0);

fn holds() -> MutexGuard<'static, HashMap<u32, Hold>> {
    HOLDS.lock().unwrap_or_else(|e| e.into_inner())
}

fn next_generation() -> u64 {
    GENERATION.fetch_add(1, Ordering::Relaxed) + 1
}

/// `Err` si PipeWire ne répond pas : l'appelant passe par la réservation D-Bus.
pub fn acquire(card: u32) -> Result<(), String> {
    let mut holds = holds();
    if let Some(hold) = holds.get_mut(&card) {
        hold.count += 1;
        hold.generation = next_generation();
        return Ok(());
    }

    let restore = match read_cards()?.into_iter().find(|c| c.0 == card).map(|c| c.1) {
        None => None,
        Some(found) if found.profile == PROFILE_OFF => None,
        Some(found) => {
            apply_profile(&found, PROFILE_OFF)?;
            log::info!(
                "🔒 Carte audio {card} libérée par PipeWire (profil « {} » → « {PROFILE_OFF} »)",
                found.profile
            );
            let restore = Restore { device: found.device, profile: found.profile };
            remember(&restore);
            Some(restore)
        }
    };
    holds.insert(card, Hold { count: 1, restore, generation: next_generation() });
    Ok(())
}

/// Le profil d'origine revient après le délai de grâce, si rien n'a repris la carte.
pub fn release(card: u32) {
    let generation = {
        let mut holds = holds();
        let Some(hold) = holds.get_mut(&card) else { return };
        hold.count = hold.count.saturating_sub(1);
        if hold.count > 0 {
            return;
        }
        hold.generation = next_generation();
        hold.generation
    };

    std::thread::spawn(move || {
        std::thread::sleep(GRACE);
        let mut holds = holds();
        if holds.get(&card).is_none_or(|h| h.count > 0 || h.generation != generation) {
            return;
        }
        if let Some(restore) = holds.remove(&card).and_then(|h| h.restore) {
            restore_profile(&restore);
        }
    });
}

/// Carte hors service de notre fait : sa sortie a disparu de PipeWire.
pub fn is_held(card: u32) -> bool {
    holds().get(&card).is_some_and(|h| h.restore.is_some())
}

/// Rend la carte sans attendre la fin du délai de grâce ; `false` si elle est encore prise.
pub fn restore_now(card: u32) -> bool {
    let hold = {
        let mut holds = holds();
        match holds.get(&card) {
            Some(h) if h.count > 0 => return false,
            Some(_) => holds.remove(&card),
            None => return true,
        }
    };
    if let Some(restore) = hold.and_then(|h| h.restore) {
        restore_profile(&restore);
    }
    true
}

/// À la fermeture de l'application.
pub fn restore_all() {
    let pending: Vec<Hold> = holds().drain().map(|(_, h)| h).collect();
    for restore in pending.into_iter().filter_map(|h| h.restore) {
        restore_profile(&restore);
    }
}

/// Au lancement : cartes laissées hors service par un plantage.
pub fn restore_pending() {
    let Some(path) = marker_path() else { return };
    let Ok(content) = std::fs::read_to_string(&path) else { return };
    for line in content.lines() {
        if let Some((device, profile)) = line.split_once('\t') {
            restore_profile(&Restore { device: device.to_string(), profile: profile.to_string() });
        }
    }
    let _ = std::fs::remove_file(&path);
}

fn restore_profile(restore: &Restore) {
    match read_cards() {
        Ok(cards) => match cards.into_iter().map(|c| c.1).find(|c| c.device == restore.device) {
            // Profil changé entre-temps ou carte débranchée : on n'y touche pas.
            Some(card) if card.profile == PROFILE_OFF => match apply_profile(&card, &restore.profile) {
                Ok(()) => log::info!(
                    "🔓 Carte audio rendue à PipeWire ({}, profil « {} »)",
                    restore.device,
                    restore.profile
                ),
                Err(e) => log::warn!("🔓 Profil de {} non rétabli : {e}", restore.device),
            },
            _ => {}
        },
        Err(e) => log::warn!("🔓 Profil de {} non rétabli : {e}", restore.device),
    }
    forget(&restore.device);
}

fn read_cards() -> Result<Vec<(u32, Card)>, String> {
    let output = Command::new("pw-dump").output().map_err(|e| format!("pw-dump : {e}"))?;
    if !output.status.success() {
        return Err(format!("pw-dump : {}", output.status));
    }
    let objects: Vec<Value> =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("pw-dump illisible : {e}"))?;
    Ok(objects.iter().filter_map(parse_card).collect())
}

fn parse_card(object: &Value) -> Option<(u32, Card)> {
    if object["type"] != "PipeWire:Interface:Device" {
        return None;
    }
    let info = &object["info"];
    let index = &info["props"]["api.alsa.card"];
    let index = index.as_u64().or_else(|| index.as_str()?.parse().ok())? as u32;
    let profiles = info["params"]["EnumProfile"]
        .as_array()?
        .iter()
        .filter_map(|p| Some((p["index"].as_u64()? as u32, p["name"].as_str()?.to_string())))
        .collect();
    Some((
        index,
        Card {
            id: object["id"].as_u64()? as u32,
            device: info["props"]["device.name"].as_str()?.to_string(),
            profile: info["params"]["Profile"].get(0)?["name"].as_str()?.to_string(),
            profiles,
        },
    ))
}

/// `save: false` : WirePlumber ne garde pas ce profil d'une session à l'autre.
fn apply_profile(card: &Card, profile: &str) -> Result<(), String> {
    let (index, _) = card
        .profiles
        .iter()
        .find(|(_, name)| name == profile)
        .ok_or_else(|| format!("profil « {profile} » inconnu de {}", card.device))?;
    let output = Command::new("pw-cli")
        .args([
            "set-param",
            &card.id.to_string(),
            "Profile",
            &format!("{{ \"index\": {index}, \"save\": false }}"),
        ])
        .output()
        .map_err(|e| format!("pw-cli : {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("pw-cli : {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

fn marker_path() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("com.larevuegeek.rustmusic").join("profils_pipewire_a_retablir"))
}

fn marker_lines() -> Vec<String> {
    marker_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|c| c.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn write_marker(lines: &[String]) {
    let Some(path) = marker_path() else { return };
    if lines.is_empty() {
        let _ = std::fs::remove_file(&path);
    } else if let Err(e) = std::fs::write(&path, lines.join("\n") + "\n") {
        log::warn!("Témoin de profil PipeWire non écrit ({}) : {e}", path.display());
    }
}

fn remember(restore: &Restore) {
    let mut lines = marker_lines();
    lines.retain(|l| l.split('\t').next() != Some(restore.device.as_str()));
    lines.push(format!("{}\t{}", restore.device, restore.profile));
    write_marker(&lines);
}

fn forget(device: &str) {
    let mut lines = marker_lines();
    lines.retain(|l| l.split('\t').next() != Some(device));
    write_marker(&lines);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_card_from_pw_dump() {
        let object: Value = serde_json::from_str(
            r#"{ "id": 79, "type": "PipeWire:Interface:Device", "info": {
                "props": { "api.alsa.card": 3, "device.name": "alsa_card.usb-Fosi_Audio_Fosi_Audio_K7-00" },
                "params": {
                    "EnumProfile": [ { "index": 0, "name": "off" }, { "index": 1, "name": "output:analog-stereo" } ],
                    "Profile": [ { "index": 1, "name": "output:analog-stereo", "save": false } ] } } }"#,
        )
        .unwrap();
        let (index, card) = parse_card(&object).expect("carte");
        assert_eq!((index, card.id, card.profile.as_str()), (3, 79, "output:analog-stereo"));
        assert_eq!(card.device, "alsa_card.usb-Fosi_Audio_Fosi_Audio_K7-00");
        assert_eq!(card.profiles, vec![(0, "off".to_string()), (1, "output:analog-stereo".to_string())]);
    }

    #[test]
    fn ignores_other_objects() {
        let node: Value = serde_json::from_str(r#"{ "id": 50, "type": "PipeWire:Interface:Node", "info": {} }"#).unwrap();
        assert!(parse_card(&node).is_none());
        // Certaines versions de PipeWire écrivent l'index ALSA en texte.
        let text: Value = serde_json::from_str(
            r#"{ "id": 7, "type": "PipeWire:Interface:Device", "info": {
                "props": { "api.alsa.card": "0", "device.name": "alsa_card.pci" },
                "params": { "EnumProfile": [], "Profile": [ { "index": 0, "name": "off" } ] } } }"#,
        )
        .unwrap();
        assert_eq!(parse_card(&text).map(|c| c.0), Some(0));
    }
}
