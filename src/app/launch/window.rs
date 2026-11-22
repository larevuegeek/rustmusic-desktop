//! Ouverture de la fenêtre principale.

use crate::app::*;
use crate::mapper::app::app_data::page_route;
use std::sync::mpsc::Receiver;
use karbon_ui::{platform, window::WindowOptions};
use crate::app::{AppHandle, Event};

/// Icône de la fenêtre (`WindowOptions::icon`) : barre des tâches et Alt+Tab sous Windows
/// (l'exécutable a aussi la sienne, `build.rs`), `_NET_WM_ICON` sous X11, Dock sous macOS
/// hors paquet `.app` (dessin au format macOS, coins arrondis et marge).
#[cfg(target_os = "macos")]
pub(crate) const WINDOW_ICON: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/icon-macos.png"));

#[cfg(not(target_os = "macos"))]
pub(crate) const WINDOW_ICON: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/128x128@2x.png"));

/// Ouvre la fenêtre principale et rend la main à sa fermeture.
pub fn run(app: AppHandle, events: Receiver<Event>) -> karbon_ui::Result<()> {
    // Langue par défaut (traductions et formats de `karbon_ui::intl`) jusqu'aux réglages.
    crate::app::resources::translations::set_language("fr");
    // Petites préférences de l'interface, gardées
    // dans `ui-local.json` du dossier de données.
    karbon_ui::storage::open_local(crate::app::platform::paths::app_data_dir().join("ui-local.json"));
    // Options natives passées sur la ligne de commande (`--renderer=software`…) ; les
    // commandes de RustMusic (`/play`, `--next`) sont lues par `launch_args`.
    let native: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a.starts_with("--renderer") || a.starts_with("--present") || a == "--profile")
        .collect();
    let chosen = native.iter().any(|a| a.starts_with("--renderer"));
    let options = WindowOptions::from_args(native).unwrap_or(WindowOptions::DEFAULT);
    // Sans `--renderer=…` : le mode de rendu des réglages (ou `RUSTMUSIC_RENDER`).
    let pool = app.state::<crate::app::platform::app_state::AppState>().pool.clone();
    let saved = crate::app::platform::runtime::block_on(
        crate::repository::settings::settings_repository::SettingsRepository::get(
            &pool,
            crate::core::system::render_mode::KEY,
        ),
    )
    .ok()
    .flatten();
    let gpu_failed = crate::core::system::gpu_sentinel::gpu_failed_before(&pool);
    let renderer = crate::core::system::render_mode::startup_renderer(saved.as_deref(), gpu_failed);
    // Lancement au démarrage activé : entrée réécrite, pour suivre l'appli déplacée ou
    // mise à jour (et reprendre celle de la 0.4).
    let auto_start = crate::app::platform::runtime::block_on(
        crate::repository::settings::settings_repository::SettingsRepository::get(
            &app.state::<crate::app::platform::app_state::AppState>().pool,
            crate::core::system::autostart::KEY,
        ),
    )
    .ok()
    .flatten();
    if auto_start.as_deref() == Some("true") {
        crate::app::platform::runtime::spawn_blocking(|| {
            if let Err(e) = crate::core::system::autostart::set(true) {
                log::warn!("Lancement au démarrage : {e}");
            }
        });
    }
    let options = WindowOptions {
        renderer: if chosen { options.renderer } else { renderer },
        ..options
    };
    let mut options = WindowOptions {
        title: "RustMusic",
        width: 1280,
        height: 900,
        min_width: crate::controller::player::mini_window::NORMAL_MIN.0,
        min_height: crate::controller::player::mini_window::NORMAL_MIN.1,
        title_bar: false,
        icon: Some(WINDOW_ICON),
        ..options
    };
    let saved = crate::core::system::window_geometry::load();
    if let Some(g) = saved {
        if g.width >= options.min_width && g.height >= options.min_height {
            options.x = g.x;
            options.y = g.y;
            options.width = g.width;
            options.height = g.height;
        }
    }
    let mut music = MusicApp::new(app, events);
    // Fenêtre agrandie ou en plein écran à la dernière fermeture : rouverte ainsi, la
    // taille normale gardée pour la restauration.
    if let Some(g) = saved {
        use karbon_ui::window::{WindowChange, WindowCommand, WindowId};
        if g.fullscreen {
            music.window_commands.push(WindowCommand::Change(WindowId::MAIN, WindowChange::Fullscreen(true)));
        } else if g.maximized {
            music.window_commands.push(WindowCommand::Change(WindowId::MAIN, WindowChange::Maximized(true)));
        }
    }
    // Fenêtre ouverte sur une page (`--page tracks`) : mesures de fluidité à la main.
    let args: Vec<String> = std::env::args().collect();
    music.export_route = args
        .iter()
        .position(|a| a == "--page")
        .and_then(|i| args.get(i + 1))
        .map(|p| page_route(p));
    if options.renderer != karbon_ui::window::RenderMode::Software {
        crate::core::system::gpu_sentinel::arm();
    }
    log::info!("🖥  Ouverture de la fenêtre ({:?})", options.renderer);
    let result = platform::run(options, music);
    // Fermeture normale : la session GPU n'a pas planté.
    crate::core::system::gpu_sentinel::disarm();
    result
}
