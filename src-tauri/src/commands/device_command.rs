//! Enumération enrichie des périphériques de sortie audio.
//!
//! Fournit au frontend une description structurée pour chaque device :
//! fréquences supportées, formats d'échantillon, nombre de canaux, marqueur
//! « défaut système ». Utilisé par la vitrine Réglages > Audio et par le
//! dropdown compact du player.

use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

/// Fréquences PCM standard qu'on teste contre les plages CPAL de chaque
/// device. On limite délibérément à des valeurs « musicalement pertinentes »
/// pour éviter de lister toutes les intermediaires exposées par certains
/// drivers (11025, 22050, 32000, ...).
const CANDIDATE_RATES: &[u32] = &[
    44_100, 48_000, 88_200, 96_000, 176_400, 192_000, 352_800, 384_000, 705_600, 768_000,
];

/// Description publique d'un device de sortie.
///
/// Sérialisée vers le frontend en camelCase pour rester idiomatique TS.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    /// Nom brut CPAL (utilisé pour matcher côté backend lors de `set_device`).
    pub name: String,
    /// Nom affiché côté UI ("Nom (Fabricant)" ou fallback nom brut).
    pub display_name: String,
    pub manufacturer: Option<String>,
    pub driver: Option<String>,
    /// `true` quand le device correspond au défaut système CPAL.
    pub is_default: bool,
    /// Fréquences supportées, triées ascendant, dédupliquées. Filtre sur
    /// `CANDIDATE_RATES` — si le driver expose [44100..=192000] on renvoie
    /// [44100, 48000, 88200, 96000, 176400, 192000].
    pub sample_rates: Vec<u32>,
    /// Formats d'échantillon exposés : "16-bit int" / "24-bit int" /
    /// "32-bit float" / etc.
    pub sample_formats: Vec<String>,
    /// Nombre maximum de canaux exposé par le device.
    pub max_channels: u16,
    /// ID WASAPI du device (Windows uniquement). Sert à cibler ce device
    /// précis pour `wasapi_probe_device_capabilities`. `None` sur les
    /// autres OS ou si la corrélation par nom a échoué.
    pub wasapi_id: Option<String>,
    /// Taille min de buffer exposée (en frames). `None` si la plage est
    /// « Unknown » côté driver.
    pub min_buffer_frames: Option<u32>,
    /// Taille max de buffer exposée (en frames).
    pub max_buffer_frames: Option<u32>,
    /// Résumé "Hi-Res" : la chaîne peut traiter du 24-bit / >= 88.2 kHz.
    /// Sert au badge du dropdown compact.
    pub is_hires: bool,
}

/// Traduit un `SampleFormat` CPAL en libellé humain court.
fn format_label(fmt: cpal::SampleFormat) -> &'static str {
    use cpal::SampleFormat::*;
    match fmt {
        I8 => "8-bit int",
        I16 => "16-bit int",
        I32 => "32-bit int",
        I64 => "64-bit int",
        U8 => "8-bit uint",
        U16 => "16-bit uint",
        U32 => "32-bit uint",
        U64 => "64-bit uint",
        F32 => "32-bit float",
        F64 => "64-bit float",
        _ => "unknown",
    }
}

#[cfg(target_os = "linux")]
fn is_alsa_virtual_device(name: &str) -> bool {
    const VIRTUAL_PREFIXES: &[&str] = &[
        "surround", "front", "rear", "side", "center_lfe", "iec958", "spdif", "hdmi", "dmix",
        "dsnoop", "plughw", "hw:", "plug:", "usbstream", "samplerate", "speexrate", "upmix",
        "vdownmix", "null", "jack", "oss",
    ];
    let lower = name.to_lowercase();
    VIRTUAL_PREFIXES
        .iter()
        .any(|p| lower.starts_with(p) || lower.contains(&format!(" {}", p)))
}

#[cfg(not(target_os = "linux"))]
fn is_alsa_virtual_device(_name: &str) -> bool {
    false
}

/// Corrèle un nom CPAL → ID WASAPI par matching de friendly_name. Sur
/// Windows uniquement. Renvoie `None` sur les autres OS ou si aucun match.
/// La liste WASAPI est construite une seule fois par appel de
/// `get_output_devices`.
#[cfg(target_os = "windows")]
fn find_wasapi_id(name: &str, wasapi_list: &[(String, String)]) -> Option<String> {
    // Match exact d'abord (friendly_name identique)
    for (id, friendly) in wasapi_list {
        if friendly == name {
            return Some(id.clone());
        }
    }
    // Fallback : substring (parfois CPAL renvoie "Speakers" là où WASAPI
    // renvoie "Speakers (Realtek Audio)")
    for (id, friendly) in wasapi_list {
        if friendly.starts_with(name) || name.starts_with(friendly.as_str()) {
            return Some(id.clone());
        }
    }
    None
}

/// Description CPAL → AudioDeviceInfo pour un device donné. `is_default`
/// est calculé par comparaison de nom avec le default host output.
fn describe_device(
    device: &cpal::Device,
    default_name: &str,
    #[cfg(target_os = "windows")] wasapi_list: &[(String, String)],
) -> Option<AudioDeviceInfo> {
    // CPAL 0.18 : `Device::name()` supprimé → tout passe par `description()`.
    let desc = device.description().ok()?;
    let raw_name = desc.name().to_string();
    let drv = desc.driver().map(str::to_string);

    // Filtre ALSA : le nom brut est identique pour tous les alias d'une même
    // carte (ex. "Fosi Audio K7, USB Audio"), le discriminant virtuel
    // (hw:/plughw:/surround/iec958/sysdefault…) est dans le champ `driver`.
    // On teste donc les DEUX, sinon la liste explose en doublons ALSA.
    if is_alsa_virtual_device(&raw_name)
        || drv.as_deref().map(is_alsa_virtual_device).unwrap_or(false)
    {
        return None;
    }

    let (display_name, manufacturer, driver) = {
        let name = raw_name.clone();
        let mfr = desc.manufacturer().map(str::to_string);
        // Windows expose déjà le fabricant dans le nom brut CPAL
        // (« Haut-parleurs (3- Fosi Audio K7) ») : ne l'ajouter que s'il n'y
        // est pas déjà, sinon on affiche « Haut-parleurs (X) (X) ».
        let suffix = mfr.as_deref().or(drv.as_deref());
        let disp = match suffix {
            Some(s) if !name.to_lowercase().contains(&s.to_lowercase()) => {
                format!("{} ({})", name, s)
            }
            _ => name.clone(),
        };
        (disp, mfr, drv)
    };

    // Agrégation des `supported_output_configs`.
    let mut rates_set = std::collections::BTreeSet::<u32>::new();
    let mut formats_set = std::collections::BTreeSet::<&'static str>::new();
    let mut max_channels: u16 = 0;
    let mut min_buf: Option<u32> = None;
    let mut max_buf: Option<u32> = None;

    if let Ok(configs) = device.supported_output_configs() {
        for cfg in configs {
            let min = cfg.min_sample_rate();
            let max = cfg.max_sample_rate();
            for &r in CANDIDATE_RATES {
                if min <= r && r <= max {
                    rates_set.insert(r);
                }
            }
            // Skip "unknown" — SampleFormat CPAL est #[non_exhaustive], on
            // ne veut pas polluer l'UI avec des variants inconnus.
            let label = format_label(cfg.sample_format());
            if label != "unknown" {
                formats_set.insert(label);
            }
            let ch = cfg.channels();
            if ch > max_channels {
                max_channels = ch;
            }
            // On ignore les plages `Unknown` et les valeurs sentinelles
            // (u32::MAX ~= 4,3 G frames) que certains pilotes WASAPI
            // renvoient à la place d'une vraie borne.
            if let cpal::SupportedBufferSize::Range { min, max } = cfg.buffer_size() {
                if *max < u32::MAX / 2 {
                    min_buf = Some(min_buf.map(|v| v.min(*min)).unwrap_or(*min));
                    max_buf = Some(max_buf.map(|v| v.max(*max)).unwrap_or(*max));
                }
            }
        }
    }

    let sample_rates: Vec<u32> = rates_set.into_iter().collect();
    let sample_formats: Vec<String> =
        formats_set.into_iter().map(|s| s.to_string()).collect();

    // Hi-Res = ≥ 24-bit ET ≥ 88.2 kHz (règle usuelle audiophile).
    let has_24bit_or_more = sample_formats.iter().any(|f| {
        f.contains("32-bit") || f.contains("24-bit") || f.contains("64-bit")
    });
    let has_hi_rate = sample_rates.iter().any(|&r| r >= 88_200);
    let is_hires = has_24bit_or_more && has_hi_rate;

    #[cfg(target_os = "windows")]
    let wasapi_id = find_wasapi_id(&raw_name, wasapi_list);
    #[cfg(not(target_os = "windows"))]
    let wasapi_id: Option<String> = None;

    Some(AudioDeviceInfo {
        name: raw_name,
        display_name,
        manufacturer,
        driver,
        is_default: display_name_matches(device, default_name),
        sample_rates,
        sample_formats,
        max_channels,
        min_buffer_frames: min_buf,
        max_buffer_frames: max_buf,
        is_hires,
        wasapi_id,
    })
}

fn display_name_matches(device: &cpal::Device, default_name: &str) -> bool {
    match device.description() {
        Ok(d) => d.name() == default_name,
        Err(_) => false,
    }
}

#[tauri::command]
pub async fn get_output_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    let host: cpal::Host = cpal::default_host();

    let default_name = host
        .default_output_device()
        .and_then(|d| d.description().ok())
        .map(|d| d.name().to_string())
        .unwrap_or_default();

    let output_devices = host
        .output_devices()
        .map_err(|e| format!("Failed to get output devices: {}", e))?;

    // Table WASAPI (Windows uniquement) pour cross-référencer les IDs.
    #[cfg(target_os = "windows")]
    let wasapi_list: Vec<(String, String)> =
        crate::core::audio_player::audio_output_wasapi::list_output_devices()
            .map(|list| list.into_iter().map(|d| (d.id, d.friendly_name)).collect())
            .unwrap_or_default();

    // Dédup sur le display_name (nom brut + fabricant / pilote) — Windows
    // expose souvent plusieurs endpoints qui partagent le nom brut mais
    // diffèrent par manufacturer (Realtek / NVIDIA HDMI / USB…). On veut
    // conserver tous les endpoints distincts pour laisser le choix.
    let mut seen = std::collections::HashSet::<String>::new();
    let mut out: Vec<AudioDeviceInfo> = Vec::new();

    for device in output_devices {
        #[cfg(target_os = "windows")]
        let info_opt = describe_device(&device, &default_name, &wasapi_list);
        #[cfg(not(target_os = "windows"))]
        let info_opt = describe_device(&device, &default_name);

        if let Some(info) = info_opt {
            if seen.insert(info.display_name.clone()) {
                out.push(info);
            }
        }
    }

    // Tri : défaut en tête, puis alpha.
    out.sort_by(|a, b| {
        b.is_default
            .cmp(&a.is_default)
            .then_with(|| a.display_name.cmp(&b.display_name))
    });

    Ok(out)
}
