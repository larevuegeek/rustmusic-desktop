//! Le lecteur de la zone de notification : ouvert par un clic gauche sur l'icône, tenu à
//! jour par la fenêtre principale, refermé dès qu'il perd le focus.

use std::time::{Duration, Instant};
use karbon_ui::task::Inbox;
use karbon_ui::window::{WindowChange, WindowCommand, WindowDescriptor, WindowId, WindowOptions};
use crate::app::music_app::MusicApp;
use crate::app::tray_player::TrayPlayer;

/// La fenêtre du petit lecteur.
pub(crate) const TRAY_WINDOW: WindowId = WindowId(7);
/// Taille du petit lecteur (unités logiques).
pub(crate) const TRAY_SIZE: (i32, i32) = (340, 116);
/// Écart avec l'icône.
const GAP: i32 = 14;
/// Refermé à l'instant par le clic sur l'icône elle-même : ce clic ne le rouvre pas.
const REOPEN_GUARD: Duration = Duration::from_millis(400);

impl MusicApp {
    /// Clic gauche sur l'icône : ouvre le petit lecteur au-dessus d'elle (en dessous si la
    /// barre des tâches est en haut), ou le referme.
    pub(crate) fn tray_toggle(&mut self, anchor: (i32, i32)) {
        if self.tray_player.take().is_some() {
            self.window_commands.push(WindowCommand::Close(TRAY_WINDOW));
            self.tray_closed();
            return;
        }
        if self.tray_closed_at.is_some_and(|t| t.elapsed() < REOPEN_GUARD) {
            return;
        }
        let (width, height) = TRAY_SIZE;
        let below = anchor.1 < height + 2 * GAP;
        let options = WindowOptions {
            title: "RustMusic",
            x: (anchor.0 - width / 2).max(GAP),
            y: if below { anchor.1 + GAP } else { anchor.1 - height - GAP },
            width,
            height,
            min_width: width,
            min_height: height,
            title_bar: false,
            taskbar: false,
            topmost: true,
            ..WindowOptions::DEFAULT
        };
        let inbox = Inbox::new();
        let sender = inbox.sender();
        let snapshot = self.tray_snapshot();
        let player = TrayPlayer::new(snapshot.clone(), self.inbox.sender(), inbox);
        self.window_commands.push(WindowCommand::Open(WindowDescriptor::new(TRAY_WINDOW, options, player)));
        // Barre des tâches de côté, écran de bord : la fenêtre est ramenée dans l'écran.
        self.window_commands.push(WindowCommand::Change(TRAY_WINDOW, WindowChange::KeepOnScreen));
        self.tray_player = Some(sender);
        self.tray_sent = Some(snapshot);
    }

    /// Le petit lecteur s'est refermé (focus perdu, fenêtre ouverte, clic sur l'icône).
    pub(crate) fn tray_closed(&mut self) {
        self.tray_player = None;
        self.tray_sent = None;
        self.tray_closed_at = Some(Instant::now());
    }

    /// Tient le petit lecteur à jour : son état ne lui est envoyé que s'il a changé.
    pub(crate) fn tray_sync(&mut self) {
        let Some(sender) = &self.tray_player else {
            return;
        };
        let snapshot = self.tray_snapshot();
        if self.tray_sent.as_ref() == Some(&snapshot) {
            return;
        }
        let state = snapshot.clone();
        if sender.post(move |player| player.show(state)) {
            self.tray_sent = Some(snapshot);
        } else {
            self.tray_closed();
        }
    }
}
