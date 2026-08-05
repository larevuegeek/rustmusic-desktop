//! System-level Tauri commands (rendering mode, etc.).
//!
//! Currently exposes the WebKit/GDK render-mode override. Changing this
//! setting requires an app restart since env vars are read by WebKitGTK at
//! process init.

use serde::Serialize;
use tauri::State;

use crate::core::gpu_sentinel;
use crate::core::render_mode::RenderMode;
use crate::repository::settings::settings_repository::SettingsRepository;
use crate::state::AppState;

const KEY: &str = "render_mode";

#[derive(Debug, Serialize)]
pub struct RenderModeStatus {
    /// Current persisted setting (`"auto" | "force-gpu" | "force-software"`).
    pub mode: RenderMode,
    /// Detected virt kind (e.g. `"kvm"`) on Linux, or `null` on native / non-Linux.
    pub virt_kind: Option<String>,
    /// True on SteamOS (Steam Deck) — Auto mode defaults to software there.
    pub steamos: bool,
    /// True when a previous GPU boot crashed and Auto mode now falls back to
    /// software. Cleared by picking any render mode, or by a successful GPU
    /// boot (e.g. launched with `RUSTMUSIC_RENDER=gpu`).
    pub gpu_boot_failed: bool,
}

fn detect_virt_kind() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        return crate::core::system_detect::detect_linux_virt();
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn detect_steamos() -> bool {
    #[cfg(target_os = "linux")]
    {
        return crate::core::system_detect::detect_steamos();
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

async fn read_gpu_boot_failed(state: &State<'_, AppState>) -> bool {
    SettingsRepository::get(&state.pool, gpu_sentinel::GPU_BOOT_FAILED_KEY)
        .await
        .ok()
        .flatten()
        .as_deref()
        == Some("1")
}

#[tauri::command]
pub async fn get_render_mode(
    state: State<'_, AppState>,
) -> Result<RenderModeStatus, String> {
    let raw = SettingsRepository::get(&state.pool, KEY)
        .await
        .map_err(|e| format!("read {KEY}: {e}"))?;
    let gpu_boot_failed = read_gpu_boot_failed(&state).await;
    Ok(RenderModeStatus {
        mode: RenderMode::parse_or_auto(raw.as_deref()),
        virt_kind: detect_virt_kind(),
        steamos: detect_steamos(),
        gpu_boot_failed,
    })
}

#[tauri::command]
pub async fn set_render_mode(
    state: State<'_, AppState>,
    value: String,
) -> Result<RenderModeStatus, String> {
    let mode = RenderMode::parse_or_auto(Some(&value));
    SettingsRepository::set(&state.pool, KEY, mode.as_str())
        .await
        .map_err(|e| format!("save {KEY}: {e}"))?;
    // Any explicit user choice resets the remembered GPU failure — that's the
    // in-app way to retry GPU after a crash (e.g. post SteamOS update).
    SettingsRepository::set(&state.pool, gpu_sentinel::GPU_BOOT_FAILED_KEY, "0")
        .await
        .map_err(|e| format!("save {}: {e}", gpu_sentinel::GPU_BOOT_FAILED_KEY))?;
    log::info!("🖥  Render mode updated : {:?} (effective at next restart)", mode);
    Ok(RenderModeStatus {
        mode,
        virt_kind: detect_virt_kind(),
        steamos: detect_steamos(),
        gpu_boot_failed: false,
    })
}

/// Called by the frontend once the webview has painted real frames (double
/// requestAnimationFrame after mount). Disarms the GPU crash sentinel ; if
/// this boot ran on GPU while a past failure was remembered, the success
/// proves the GPU path works again so the flag is cleared too.
#[tauri::command]
pub async fn notify_ui_ready(state: State<'_, AppState>) -> Result<(), String> {
    gpu_sentinel::disarm();
    if gpu_sentinel::booted_gpu() && read_gpu_boot_failed(&state).await {
        SettingsRepository::set(&state.pool, gpu_sentinel::GPU_BOOT_FAILED_KEY, "0")
            .await
            .map_err(|e| format!("save {}: {e}", gpu_sentinel::GPU_BOOT_FAILED_KEY))?;
        log::info!("🖥  GPU boot confirmed OK → clearing remembered GPU failure");
    }
    Ok(())
}
