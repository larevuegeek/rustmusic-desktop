//! La fenêtre du petit lecteur : l'état vient de la fenêtre principale (`MusicApp::tray_sync`),
//! les commandes y repartent par sa boîte aux lettres.

use karbon_ui::application::{Application, ColorPreferences, ColorScheme, Resources, ViewContext};
use karbon_ui::intl::t;
use karbon_ui::task::{Inbox, InboxSender};
use karbon_ui::ui::font::FontAsset;
use karbon_ui::ui::prelude::*;
use karbon_ui::window::{WindowChange, WindowCommand, WindowId, WindowState};
use crate::app::music_app::MusicApp;
use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use crate::component::*;
use crate::controller::tray::tray_player::TRAY_WINDOW;
use crate::models::app::Msg;
use crate::models::tray::{TrayMsg, TraySnapshot};

pub(crate) struct TrayPlayer {
    state: TraySnapshot,
    /// La fenêtre principale.
    main: InboxSender<MusicApp>,
    inbox: Inbox<TrayPlayer>,
    assets: Option<Assets>,
    commands: Vec<WindowCommand>,
    /// La fenêtre a eu le focus : le perdre la referme.
    focused: bool,
}

impl TrayPlayer {
    pub(crate) fn new(state: TraySnapshot, main: InboxSender<MusicApp>, inbox: Inbox<TrayPlayer>) -> Self {
        Self { state, main, inbox, assets: None, commands: Vec::new(), focused: false }
    }

    /// Nouvel état de la fenêtre principale.
    pub(crate) fn show(&mut self, state: TraySnapshot) -> bool {
        self.state = state;
        true
    }

    /// Commande jouée par la fenêtre principale, qui renvoie aussitôt l'état à jour.
    fn send(&self, msg: Msg) {
        self.main.post(move |app| {
            let changed = app.update(msg);
            app.tray_sync();
            changed
        });
    }

    /// Referme le petit lecteur et prévient la fenêtre principale.
    fn close(&mut self) {
        self.commands.push(WindowCommand::Close(TRAY_WINDOW));
        self.main.post(|app| {
            app.tray_closed();
            false
        });
    }
}

impl Application for TrayPlayer {
    type Message = TrayMsg;

    fn fonts(&self) -> &[FontAsset] {
        &crate::app::resources::assets::FONTS
    }

    fn load(&mut self, resources: &dyn Resources) -> karbon_ui::Result<()> {
        self.assets = Some(Assets::load(resources)?);
        Ok(())
    }

    fn color_preferences(&self) -> ColorPreferences {
        ColorPreferences { scheme: ColorScheme::Dark, high_contrast: false }
    }

    fn view(&self, _: &ViewContext) -> Element {
        let assets = self.assets.as_ref().expect("Application::load precedes view");
        let cx = TemplateContext::new(assets, "tray");
        let s = &self.state;
        ui_file!("/ui/templates/tray/TrayPlayer.ui", context = cx, styles = css)
    }

    fn update(&mut self, message: TrayMsg) -> bool {
        match message {
            TrayMsg::Previous => self.send(Msg::Previous),
            TrayMsg::TogglePlay => self.send(Msg::TogglePlay),
            TrayMsg::Next => self.send(Msg::Next),
            TrayMsg::Open => {
                self.main.post(|app| {
                    app.window_commands.push(WindowCommand::Change(WindowId::MAIN, WindowChange::Focus));
                    false
                });
                self.close();
            }
        }
        false
    }

    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands)
    }

    /// Focus perdu (clic ailleurs, Échap n'existe pas ici) : le petit lecteur se referme.
    fn window_changed(&mut self, state: WindowState) -> bool {
        if state.focused {
            self.focused = true;
        } else if std::mem::take(&mut self.focused) {
            self.close();
        }
        false
    }

    fn image_loader(&self) -> Option<karbon_ui::resources::ImageLoader> {
        self.assets.as_ref().map(|a| a.covers.clone())
    }

    fn inbox(&self) -> Option<Inbox<Self>> {
        Some(self.inbox.clone())
    }
}
