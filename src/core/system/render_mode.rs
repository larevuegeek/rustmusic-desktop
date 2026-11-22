//! Mode de rendu de la fenêtre (Réglages → Apparence → Options expert) : GPU, logiciel
//! ou automatique (GPU, sauf dans une machine virtuelle ou après un plantage du rendu GPU,
//! où le rendu logiciel est plus sûr). Lu à l'ouverture de la fenêtre : un changement
//! attend le redémarrage.
//!
//! Ordre de priorité : `--renderer=…` sur la ligne de commande, la variable
//! `RUSTMUSIC_RENDER` (`gpu`, `software`, `auto`), puis le réglage enregistré.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenderMode {
    /// GPU, sauf machine virtuelle ou plantage GPU retenu.
    Auto,
    /// Toujours le GPU.
    ForceGpu,
    /// Toujours le processeur.
    ForceSoftware,
}

impl RenderMode {
    /// Parse the value persisted in the `settings` table (defaults to `Auto`).
    pub fn parse_or_auto(value: Option<&str>) -> Self {
        match value.map(str::to_ascii_lowercase).as_deref() {
            Some("force-gpu") | Some("gpu") => Self::ForceGpu,
            Some("force-software") | Some("software") => Self::ForceSoftware,
            _ => Self::Auto,
        }
    }

    /// Read the `RUSTMUSIC_RENDER` env override, if set.
    ///
    /// Escape hatch for users whose app crashes before they can reach the
    /// settings UI (e.g. `RUSTMUSIC_RENDER=software ./RustMusic.AppImage`),
    /// and for re-testing GPU on a machine remembered as failing
    /// (`RUSTMUSIC_RENDER=gpu`). Takes precedence over the persisted setting;
    /// `auto` re-runs the detection while ignoring the DB value.
    pub fn from_env() -> Option<Self> {
        let raw = std::env::var("RUSTMUSIC_RENDER").ok()?;
        match raw.trim().to_ascii_lowercase().as_str() {
            "gpu" | "force-gpu" => Some(Self::ForceGpu),
            "software" | "force-software" | "cpu" => Some(Self::ForceSoftware),
            "auto" => Some(Self::Auto),
            "" => None,
            other => {
                log::warn!("RUSTMUSIC_RENDER : unknown value '{}' (expected gpu | software | auto), ignoring", other);
                None
            }
        }
    }

    /// Stable string form for persistence.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::ForceGpu => "force-gpu",
            Self::ForceSoftware => "force-software",
        }
    }
}

/// Clé du réglage dans la table `settings`.
pub const KEY: &str = "render_mode";

static BOOT_MODE: std::sync::OnceLock<RenderMode> = std::sync::OnceLock::new();

/// Mode retenu au démarrage, pour signaler qu'un changement attend la relance.
pub fn boot_mode() -> RenderMode {
    BOOT_MODE.get().copied().unwrap_or(RenderMode::Auto)
}

/// Rendu de la fenêtre pour le réglage `saved` : la variable `RUSTMUSIC_RENDER` passe
/// avant ; automatique, une machine virtuelle ou une session GPU qui a planté
/// (`gpu_failed`) passe au rendu logiciel. Retient le mode du démarrage ([`boot_mode`]).
pub fn startup_renderer(saved: Option<&str>, gpu_failed: bool) -> karbon_ui::window::RenderMode {
    let env = RenderMode::from_env();
    let mode = env.unwrap_or_else(|| RenderMode::parse_or_auto(saved));
    let _ = BOOT_MODE.set(RenderMode::parse_or_auto(saved));
    let virtualized = crate::core::system::detect::is_virtualized();
    let renderer = renderer_for(mode, virtualized || gpu_failed);
    log::info!(
        "🖥  Rendu : {renderer:?} (réglage {}{}{}{})",
        RenderMode::parse_or_auto(saved).as_str(),
        if env.is_some() { ", forcé par RUSTMUSIC_RENDER" } else { "" },
        if virtualized { ", machine virtuelle" } else { "" },
        if gpu_failed { ", plantage GPU retenu" } else { "" },
    );
    set_software_active(renderer == karbon_ui::window::RenderMode::Software);
    renderer
}

/// Rendu de karbon-ui pour `mode` ; en automatique, `avoid_gpu` (machine virtuelle,
/// plantage GPU passé) passe au logiciel.
fn renderer_for(mode: RenderMode, avoid_gpu: bool) -> karbon_ui::window::RenderMode {
    use karbon_ui::window::RenderMode as Renderer;
    match mode {
        RenderMode::ForceGpu => Renderer::Gpu,
        RenderMode::ForceSoftware => Renderer::Software,
        RenderMode::Auto if avoid_gpu => Renderer::Software,
        RenderMode::Auto => Renderer::Auto,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karbon_ui::window::RenderMode as Renderer;

    #[test]
    fn saved_modes_map_to_the_window_renderer() {
        let mode = |v| RenderMode::parse_or_auto(Some(v));
        assert_eq!(renderer_for(mode("force-gpu"), true), Renderer::Gpu);
        assert_eq!(renderer_for(mode("force-software"), false), Renderer::Software);
        assert_eq!(renderer_for(mode("auto"), false), Renderer::Auto);
        assert_eq!(renderer_for(mode("auto"), true), Renderer::Software);
        assert_eq!(RenderMode::parse_or_auto(None), RenderMode::Auto);
    }
}

static SOFTWARE_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Fixé au démarrage : la fenêtre est-elle dessinée au processeur pour cette session ?
pub fn set_software_active(active: bool) {
    SOFTWARE_ACTIVE.store(active, std::sync::atomic::Ordering::Relaxed);
}

pub fn software_active() -> bool {
    SOFTWARE_ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
}
