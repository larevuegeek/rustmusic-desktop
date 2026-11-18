//! Socle de l'application : ce qui remplace Tauri (`platform/`), le thème embarqué
//! (`resources/`), l'ouverture de la fenêtre (`launch/`) et la fenêtre elle-même
//! (`music_app/`, `MusicApp`).

pub(crate) mod launch;
pub(crate) mod music_app;
pub mod platform;
pub(crate) mod resources;
pub(crate) mod tray_player;

pub use launch::preview::export;
pub use launch::window::run;
pub use music_app::MusicApp;
pub use platform::handle::{AppHandle, Event, State, WindowRequest};
