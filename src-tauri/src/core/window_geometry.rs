//! Size, position and maximized state of the main window, written as they change:
//! the app often ends without a clean exit (tray icon, `tauri dev`, crash), and the
//! window-state plugin only writes on exit. The compact player never overwrites them.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

pub const FLAGS: StateFlags = StateFlags::SIZE
    .union(StateFlags::POSITION)
    .union(StateFlags::MAXIMIZED)
    .union(StateFlags::FULLSCREEN);

static COMPACT: AtomicBool = AtomicBool::new(false);
static GENERATION: AtomicU64 = AtomicU64::new(0);
// File as written just before entering compact mode, put back if the app exits in it.
static NORMAL_SNAPSHOT: Mutex<Option<Vec<u8>>> = Mutex::new(None);

fn state_file<R: Runtime>(app: &AppHandle<R>) -> Option<std::path::PathBuf> {
    Some(app.path().app_config_dir().ok()?.join(app.filename()))
}

pub fn save_now<R: Runtime>(app: &AppHandle<R>) {
    if COMPACT.load(Ordering::Acquire) {
        return;
    }
    if let Err(e) = app.save_window_state(FLAGS) {
        log::warn!("État de la fenêtre non sauvegardé : {e}");
    }
}

/// Saves once the window has stopped moving for a moment.
pub fn schedule_save<R: Runtime>(app: AppHandle<R>) {
    if COMPACT.load(Ordering::Acquire) {
        return;
    }
    let generation = GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(800)).await;
        if GENERATION.load(Ordering::Acquire) == generation {
            save_now(&app);
        }
    });
}

pub fn set_compact<R: Runtime>(app: &AppHandle<R>, active: bool) {
    if active {
        save_now(app);
        *NORMAL_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()) =
            state_file(app).and_then(|f| std::fs::read(f).ok());
        COMPACT.store(true, Ordering::Release);
    } else {
        COMPACT.store(false, Ordering::Release);
        schedule_save(app.clone());
    }
}

/// On exit, after the plugin's own save: in compact mode, the normal geometry is put back.
pub fn on_exit<R: Runtime>(app: &AppHandle<R>) {
    if !COMPACT.load(Ordering::Acquire) {
        return;
    }
    let snapshot = NORMAL_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let (Some(bytes), Some(file)) = (snapshot, state_file(app)) {
        if let Err(e) = std::fs::write(&file, bytes) {
            log::warn!("État de la fenêtre non rétabli ({}) : {e}", file.display());
        }
    }
}
