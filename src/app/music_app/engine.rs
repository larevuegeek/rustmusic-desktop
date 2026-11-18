//! Événements du moteur : `emit`, demandes de la fenêtre, sortie.

use karbon_ui::{
    ui::prelude::*,
    window::WindowCommand,
};
use crate::models::app::ScanProgress;
use crate::models::app::Msg;
use crate::app::{Event, WindowRequest};
use super::*;

impl MusicApp {
    // ─── Événements du moteur ───────────────────────────────────────────────

    pub(crate) fn on_engine(&mut self, event: Event) -> bool {
        match event {
            Event::Emit { name, payload } => self.on_emit(&name, payload),
            Event::Window(WindowRequest::Show) => {
                log::info!("Demande d'affichage de la fenêtre");
                false
            }
            Event::Window(WindowRequest::CompactSize(..)) => false,
            Event::Exit(_) => {
                self.exiting = true;
                self.window_commands.push(WindowCommand::Exit);
                false
            }
            Event::Restart => {
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).spawn();
                }
                self.exiting = true;
                self.window_commands.push(WindowCommand::Exit);
                false
            }
        }
    }

    pub(crate) fn on_emit(&mut self, name: &str, payload: serde_json::Value) -> bool {
        match name {
            "playback-preparing" => {
                self.data.player.preparing = payload.as_bool().unwrap_or(false);
                true
            }
            "playback-ended" => {
                let path = payload.as_str().unwrap_or("").to_string();
                self.on_track_ended(&path);
                true
            }
            "playback-pipeline" => {
                self.data.player.pipeline = Some(payload);
                true
            }
            "track-advanced" => {
                // Enchaînement sans blanc : le moteur est passé au morceau suivant.
                if let Some(path) = payload.as_str() {
                    let next = self
                        .data
                        .upcoming()
                        .find(|(_, t)| t.path == path)
                        .map(|(i, _)| i);
                    if let Some(index) = next {
                        self.data.queue.current_index = index as i32;
                        self.data.player.path = Some(path.to_string());
                        self.data.player.position = 0.;
                        self.data.player.audio = None;
                        self.restore_current_track();
                    }
                }
                true
            }
            "rescan-start" | "import-start" => {
                let import = name.starts_with("import");
                self.data.scan = Some(ScanProgress {
                    label: if import { "Import" } else { "Scan" }.into(),
                    import,
                    library_id: payload.get("library_id").and_then(|v| v.as_i64()),
                    directory: payload.get("directory").and_then(|v| v.as_str()).unwrap_or_default().into(),
                    started: Some(std::time::Instant::now()),
                    ..Default::default()
                });
                true
            }
            "import-listing" => {
                let scan = self.data.scan.get_or_insert_with(|| ScanProgress {
                    label: "Import".into(),
                    import: true,
                    started: Some(std::time::Instant::now()),
                    ..Default::default()
                });
                scan.listing = true;
                scan.found = payload.get("found").and_then(|v| v.as_i64()).unwrap_or(0);
                if let Some(dir) = payload.get("directory").and_then(|v| v.as_str()) {
                    scan.directory = dir.into();
                }
                true
            }
            "batch-progress" | "batch-done" => self.batch_event(name, &payload),
            "artist-check-progress" | "artist-check-done" => self.artist_event(name, &payload),
            "rescan-progress" | "import-progress" => {
                let current = payload.get("current").and_then(|v| v.as_i64()).unwrap_or(0);
                let total = payload.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
                let scan = self.data.scan.get_or_insert_with(|| ScanProgress {
                    started: Some(std::time::Instant::now()),
                    ..Default::default()
                });
                scan.current = current;
                scan.total = total;
                scan.listing = false;
                if name.starts_with("import") {
                    scan.import = true;
                    let text = |k: &str| payload.get(k).and_then(|v| v.as_str()).map(str::to_string);
                    if let Some(file) = text("file_name") {
                        scan.file_name = file;
                    }
                    if let Some(dir) = text("directory") {
                        scan.directory = dir;
                    }
                    scan.added = payload.get("added").and_then(|v| v.as_i64()).unwrap_or(scan.added);
                    if scan.library_id.is_none() {
                        scan.library_id = payload.get("library_id").and_then(|v| v.as_i64());
                    }
                }
                true
            }
            "rescan-complete" | "import-complete" => {
                self.data.scan = None;
                if let Some(assets) = self.assets.as_mut() {
                    assets.covers.forget_failures();
                }
                self.load_profile_data();
                true
            }
            "smtc-command" | "cli-command" => {
                let msg = match payload.as_str().unwrap_or("") {
                    "play" | "pause" | "toggle" => Some(Msg::TogglePlay),
                    "next" => Some(Msg::Next),
                    "previous" => Some(Msg::Previous),
                    "stop" => Some(Msg::Stop),
                    "quit" => Some(Msg::Quit),
                    _ => None,
                };
                msg.is_some_and(|m| self.update(m))
            }
            "amp-remote-state" => self.amp_remote_refresh(),
            "amp-remote-cover" => self.amp_remote_cover(),
            "systeme-veille" => {
                if payload.as_bool() == Some(true) && self.playing() {
                    self.toggle_play();
                }
                true
            }
            "podcasts-updated" | "radios-recent-updated" | "offline-changed" => {
                self.load_profile_data();
                true
            }
            other => {
                log::debug!("Événement non traité par l'interface : {other} {payload}");
                self.data.last_event = Some(other.to_string());
                false
            }
        }
    }
}
