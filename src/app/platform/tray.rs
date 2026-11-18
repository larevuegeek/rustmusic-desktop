use karbon_ui::window::{TrayEvent, TrayIcon, WindowChange, WindowCommand, WindowId};

use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;

/// Casque vert sur fond transparent, lisible à 16 px (l'icône de l'application a un fond
/// sombre qui disparaît dans une barre des tâches sombre).
const IMAGE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/tray-32x32.png"));
const SHOW: u32 = 1;
const QUIT: u32 = 2;

impl MusicApp {
    /// Pose l'icône, ou la met à jour (langue changée).
    pub(crate) fn tray_refresh(&mut self) {
        let tray = TrayIcon::new(IMAGE)
            .tooltip("RustMusic")
            // macOS : le casque est une forme d'une couleur, teinte comme la barre des menus.
            .template(true)
            .item(SHOW, t("tray.show"))
            .separator()
            .item(QUIT, t("tray.quit"));
        self.window_commands.push(WindowCommand::Tray(WindowId::MAIN, Some(tray)));
    }

    /// Clic sur l'icône ou choix dans son menu.
    pub(crate) fn tray_handle(&mut self, event: TrayEvent) -> bool {
        match event {
            // Windows : le clic gauche ouvre le petit lecteur près de l'icône.
            TrayEvent::Activated { anchor: Some(anchor) } if cfg!(windows) && !self.export => {
                self.tray_toggle(anchor);
            }
            TrayEvent::Activated { .. } | TrayEvent::MenuItem(SHOW) => {
                // Montrée, restaurée si réduite et passée au premier plan.
                self.window_commands.push(WindowCommand::Change(WindowId::MAIN, WindowChange::Focus));
            }
            TrayEvent::MenuItem(QUIT) => self.window_commands.push(WindowCommand::Exit),
            TrayEvent::MenuItem(_) => {}
        }
        false
    }

    /// « Réduire dans la barre système » (activé par défaut) : fermer la fenêtre la masque.
    pub(crate) fn closes_to_tray(&self) -> bool {
        self.data.flag_default_on("minimize_to_tray")
    }
}
