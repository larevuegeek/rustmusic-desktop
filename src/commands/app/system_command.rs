//! Commandes système : témoin GPU, allègement automatique des effets, taille des
//! mini-lecteurs.

use serde::Serialize;
use crate::app::State;

use crate::core::system::gpu_sentinel;
use crate::app::platform::app_state::AppState;

/// Type de machine virtuelle détecté (Linux), `None` sur une vraie machine.
pub(crate) fn detect_virt_kind() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        return crate::core::system::detect::detect_linux_virt();
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Interface affichée : le témoin GPU sera levé si la session tient.
pub fn notify_ui_ready(state: State<'_, AppState>) {
    log::info!("🖥  Interface affichée");
    gpu_sentinel::confirm_when_stable(state.pool.clone());
}

/// Faut-il alléger les effets visuels (flous, fondus) quand le réglage est sur
/// `auto` ? Oui en rendu logiciel, sur un GPU faible et sur une machine modeste.
#[derive(Debug, Serialize)]
pub struct LightEffectsAuto {
    pub light: bool,
    /// Ce qui a motivé l'allègement, pour l'afficher dans les réglages.
    pub reason: Option<String>,
}

pub fn get_light_effects_auto() -> LightEffectsAuto {
    let reason = light_effects_reason();
    LightEffectsAuto { light: reason.is_some(), reason }
}

fn light_effects_reason() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        if crate::core::system::render_mode::software_active() {
            return Some("software rendering".into());
        }
        if let Some(gpu) = crate::core::system::detect::weak_gpu() {
            return Some(gpu);
        }
        if let Some(virt) = detect_virt_kind() {
            return Some(format!("virtual machine ({virt})"));
        }
    }
    crate::core::system::detect::modest_hardware()
}

/// Taille du mini et du micro-lecteur sous Linux : GTK ignore tout redimensionnement
/// d'une fenêtre non redimensionnable, sauf la taille demandée au widget. La fenêtre
/// reste fixe, sans les bords saisissables que tao ajoute aux fenêtres redimensionnables.
pub fn set_compact_window_size(
    app: crate::app::AppHandle,
    width: Option<i32>,
    height: Option<i32>,
) -> Result<(), String> {
    app.window(crate::app::WindowRequest::CompactSize(width, height));
    Ok(())
}

/// Mini ou micro-lecteur actif : la taille et la position du mode normal ne sont plus réécrites.
pub fn set_compact_mode(_app: crate::app::AppHandle, active: bool) {
    crate::core::system::window_geometry::set_compact(active);
}
