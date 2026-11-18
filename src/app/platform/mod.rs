//! Ce qui remplace Tauri : la poignée partagée par les commandes et les tâches de fond
//! (`AppHandle`, `State`), l'état géré du moteur (`AppState` : base, lecteurs, serveurs),
//! le runtime tokio, les dossiers de l'application, le handle natif de la fenêtre, le pont
//! entre le moteur et la fenêtre, l'icône de la zone de notification.

pub mod app_state;
pub mod bridge;
pub mod handle;
pub mod native_window;
pub mod paths;
pub mod runtime;
pub(crate) mod tray;
