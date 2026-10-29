//! Commandes Tauri pour tester / explorer WASAPI exclusive depuis le frontend.
//!
//! Sur Windows, ces commandes appellent réellement la couche `audio_output_wasapi`.
//! Sur les autres OS, elles renvoient une erreur claire pour que l'UI ne tente
//! pas de proposer WASAPI là où ce n'est pas pertinent.

use serde::Serialize;

#[cfg(target_os = "windows")]
use crate::core::audio_player::audio_output_wasapi;

const NOT_WINDOWS_ERR: &str = "WASAPI exclusive est disponible uniquement sur Windows.";

#[derive(Debug, Serialize)]
pub struct WasapiDeviceInfo {
    pub id: String,
    pub friendly_name: String,
}

#[derive(Debug, Serialize)]
pub struct WasapiFormatInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub sample_type: String,
}

/// Capacités réelles d'un device WASAPI, obtenues par probing du driver.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WasapiDeviceCapabilities {
    /// Sample rates supportés en mode Exclusive (bit-perfect).
    pub exclusive_rates: Vec<u32>,
    /// Bit-depths supportés en mode Exclusive.
    pub exclusive_bit_depths: Vec<u16>,
    /// Sample rates supportés en mode Shared (via mixer Windows).
    pub shared_rates: Vec<u32>,
    /// Sample rate du "mix format" par défaut Windows pour ce device.
    pub mix_rate: Option<u32>,
    /// Bit-depth du "mix format" par défaut Windows.
    pub mix_bit_depth: Option<u16>,
    /// Nombre de canaux du "mix format" par défaut.
    pub mix_channels: Option<u16>,
}

/// Liste les devices de sortie audio détectés par WASAPI.
#[tauri::command]
pub fn wasapi_list_output_devices() -> Result<Vec<WasapiDeviceInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        return audio_output_wasapi::list_output_devices().map(|devs| {
            devs.into_iter()
                .map(|d| WasapiDeviceInfo {
                    id: d.id,
                    friendly_name: d.friendly_name,
                })
                .collect()
        });
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(NOT_WINDOWS_ERR.to_string())
    }
}

/// Renvoie le nom du device par défaut (sortie audio Windows actuelle).
#[tauri::command]
pub fn wasapi_default_device_name() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        return audio_output_wasapi::default_output_device_name();
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(NOT_WINDOWS_ERR.to_string())
    }
}

/// Met à jour la préférence backend audio (atomic global lu par le thread
/// de playback). Appelée par le frontend en side-effect du toggle settings,
/// et au boot après chargement des settings.
#[tauri::command]
pub fn set_wasapi_exclusive_preference(enabled: bool) {
    crate::core::audio_player::output::set_wasapi_exclusive(enabled);
}

/// Met à jour la préférence « DSD natif (DoP) ». Effet au prochain morceau DSD.
/// Cross-OS : no-op à l'intérieur du backend sur non-Windows.
#[tauri::command]
pub fn set_dop_preference(enabled: bool) {
    crate::core::audio_player::output::set_dop_enabled(enabled);
}

/// Probe des capacités réelles d'un device par son ID WASAPI.
///
/// Sur Windows uniquement — CPAL expose sur Windows la table shared-mode
/// uniforme pour tous les endpoints, ce qui affiche à tort des capacités
/// identiques sur toutes les sorties. Cette commande probe le driver
/// directement via `AudioClient::is_supported()` pour renvoyer les rates
/// et bit-depths réellement acceptés par CE device précis.
///
/// Coût : ~200-500 ms selon le nombre de rates × bit-depths probes.
#[tauri::command]
pub fn wasapi_probe_device_capabilities(
    device_id: String,
) -> Result<WasapiDeviceCapabilities, String> {
    #[cfg(target_os = "windows")]
    {
        return audio_output_wasapi::probe_device_capabilities(device_id).map(|c| {
            WasapiDeviceCapabilities {
                exclusive_rates: c.exclusive_rates,
                exclusive_bit_depths: c.exclusive_bit_depths,
                shared_rates: c.shared_rates,
                mix_rate: c.mix_rate,
                mix_bit_depth: c.mix_bit_depth,
                mix_channels: c.mix_channels,
            }
        });
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = device_id;
        Err(NOT_WINDOWS_ERR.to_string())
    }
}

/// Teste si un format donné est supporté par le device par défaut en mode
/// exclusive. Suit la cascade de fallback : sample rate source d'abord,
/// puis 96k / 88.2k / 48k / 44.1k, chaque rate testé en 24-bit puis 16-bit.
/// Renvoie le premier format accepté, ou une erreur explicite si aucun format
/// n'est compatible.
#[tauri::command]
pub fn wasapi_test_format_negotiation(
    source_rate: u32,
    channels: u16,
) -> Result<WasapiFormatInfo, String> {
    #[cfg(target_os = "windows")]
    {
        return audio_output_wasapi::try_negotiate_exclusive_format(source_rate, channels, None)
            .map(|f| WasapiFormatInfo {
                sample_rate: f.sample_rate,
                channels: f.channels,
                bits_per_sample: f.bits_per_sample,
                sample_type: format!("{:?}", f.sample_type),
            })
            .map_err(|e| e.to_string());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (source_rate, channels);
        Err(NOT_WINDOWS_ERR.to_string())
    }
}
