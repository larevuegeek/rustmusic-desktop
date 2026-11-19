use karbon_ui::window::{TrayEvent, TrayIcon, WindowChange, WindowCommand, WindowId};

use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use karbon_ui::intl::t;

/// Casque vert sur fond transparent, lisible à 16 px (l'icône de l'application a un fond
/// sombre qui disparaît dans une barre des tâches sombre).
const IMAGE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/tray-32x32.png"));
const SHOW: u32 = 1;
const QUIT: u32 = 2;
const NOW_PLAYING: u32 = 3;
const PLAY_PAUSE: u32 = 4;
const PREVIOUS: u32 = 5;
const NEXT: u32 = 6;

/// Linux : le clic gauche ne peut pas ouvrir le petit lecteur près de l'icône (Wayland ne
/// laisse pas une application placer sa fenêtre) ; le morceau en cours et les commandes
/// de lecture sont dans le menu de l'icône.
const PLAYER_MENU: bool = cfg!(target_os = "linux");

/// Ce que montre le menu de l'icône : morceau en cours (titre, artiste) et lecture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrayMenuState {
    now_playing: Option<(String, String)>,
    playing: bool,
}

impl MusicApp {
    /// Pose l'icône, ou la met à jour (langue changée, morceau ou lecture sous Linux).
    pub(crate) fn tray_refresh(&mut self) {
        let mut tray = TrayIcon::new(IMAGE)
            .tooltip("RustMusic")
            // macOS : le casque est une forme d'une couleur, teinte comme la barre des menus.
            .template(true);
        if PLAYER_MENU {
            let state = self.tray_menu_state();
            tray = match &state.now_playing {
                Some((title, artist)) => {
                    let label = if artist.is_empty() { title.clone() } else { format!("{title} — {artist}") };
                    tray.tooltip(format!("RustMusic — {label}"))
                        .disabled_item(NOW_PLAYING, label)
                        .item(PLAY_PAUSE, t(if state.playing { "tray.pause" } else { "tray.play" }))
                        .item(PREVIOUS, t("tray.previous"))
                        .item(NEXT, t("tray.next"))
                }
                None => tray.disabled_item(NOW_PLAYING, t("tray.nothing")),
            }
            .separator();
            self.tray_menu = Some(state);
        }
        let tray = tray.item(SHOW, t("tray.show")).separator().item(QUIT, t("tray.quit"));
        self.window_commands.push(WindowCommand::Tray(WindowId::MAIN, Some(tray)));
    }

    /// Linux : repose le menu de l'icône quand le morceau ou l'état de lecture change
    /// (appelé après chaque message, comme `tray_sync`).
    pub(crate) fn tray_menu_sync(&mut self) {
        // L'icône n'est posée qu'une fois les réglages lus (langue du menu).
        if PLAYER_MENU && self.tray_menu.is_some() && self.tray_menu.as_ref() != Some(&self.tray_menu_state()) {
            self.tray_refresh();
        }
    }

    fn tray_menu_state(&self) -> TrayMenuState {
        let snapshot = self.tray_snapshot();
        TrayMenuState {
            now_playing: snapshot.has_track.then(|| (snapshot.title, snapshot.artist)),
            playing: snapshot.playing,
        }
    }

    /// Clic sur l'icône ou choix dans son menu.
    pub(crate) fn tray_handle(&mut self, event: TrayEvent) -> bool {
        match event {
            // Windows et macOS : le clic gauche ouvre le petit lecteur près de l'icône.
            TrayEvent::Activated { anchor: Some(anchor) } if !PLAYER_MENU && !self.export => {
                self.tray_toggle(anchor);
            }
            TrayEvent::Activated { .. } | TrayEvent::MenuItem(SHOW) => {
                // Montrée, restaurée si réduite et passée au premier plan.
                self.window_commands.push(WindowCommand::Change(WindowId::MAIN, WindowChange::Focus));
            }
            TrayEvent::MenuItem(QUIT) => self.window_commands.push(WindowCommand::Exit),
            TrayEvent::MenuItem(PLAY_PAUSE) => return self.handle(Msg::TogglePlay),
            TrayEvent::MenuItem(PREVIOUS) => return self.handle(Msg::Previous),
            TrayEvent::MenuItem(NEXT) => return self.handle(Msg::Next),
            TrayEvent::MenuItem(_) => {}
        }
        false
    }

    /// « Réduire dans la barre système » (activé par défaut) : fermer la fenêtre la masque.
    pub(crate) fn closes_to_tray(&self) -> bool {
        self.data.flag_default_on("minimize_to_tray")
    }
}
