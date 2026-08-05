//! Crash sentinel for GPU-rendered boots (Linux).
//!
//! Some Linux stacks (Steam Deck / SteamOS, exotic Mesa builds) abort inside
//! WebKitGTK before the window ever paints (`EGL_BAD_PARAMETER`) or show a
//! blank white window. The user can never reach the render-mode setting in
//! that state, so we detect the failure ourselves :
//!
//! 1. Before booting the webview in GPU mode, we `arm()` a sentinel file.
//! 2. The frontend calls `notify_ui_ready` once real frames have been painted
//!    (double requestAnimationFrame), which `disarm()`s it. A graceful window
//!    close also disarms it (covers "user closed the app before JS mounted").
//! 3. If the sentinel is still armed at the next startup, the previous GPU
//!    boot never displayed anything → we persist `gpu_boot_failed` in the
//!    settings table and fall back to software rendering from now on.
//!
//! The sentinel lives next to the DB/logs in the app data dir. On Windows /
//! macOS the sentinel is never armed (webview stack is reliable there).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// Settings key remembering that a GPU boot failed (read by the Auto mode).
pub const GPU_BOOT_FAILED_KEY: &str = "gpu_boot_failed";

/// Whether the current boot went down the GPU path (set at startup, read by
/// `notify_ui_ready` to know if a successful boot should clear the flag).
static BOOTED_GPU: AtomicBool = AtomicBool::new(false);

/// Set when WebKit's render process died mid-session (`web-process-terminated`
/// signal). Blocks the graceful-close disarm : a user closing the resulting
/// white window with Alt+F4 must NOT erase the recorded failure.
static WEB_PROCESS_CRASHED: AtomicBool = AtomicBool::new(false);

fn sentinel_path() -> PathBuf {
    let mut p = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("com.larevuegeek.rustmusic");
    p.push("gpu_boot_attempt");
    p
}

/// True if a previous GPU boot never confirmed that the UI was displayed.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn is_armed() -> bool {
    sentinel_path().exists()
}

/// Write the sentinel file right before a GPU boot attempt.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn arm() {
    let path = sentinel_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&path, b"") {
        log::warn!("GPU sentinel : cannot arm {} : {}", path.display(), e);
    }
}

/// Remove the sentinel — the boot is confirmed OK (UI painted or clean exit).
pub fn disarm() {
    let _ = std::fs::remove_file(sentinel_path());
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn set_booted_gpu(v: bool) {
    BOOTED_GPU.store(v, Ordering::Relaxed);
}

pub fn booted_gpu() -> bool {
    BOOTED_GPU.load(Ordering::Relaxed)
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn mark_web_process_crashed() {
    WEB_PROCESS_CRASHED.store(true, Ordering::Relaxed);
}

pub fn web_process_crashed() -> bool {
    WEB_PROCESS_CRASHED.load(Ordering::Relaxed)
}
