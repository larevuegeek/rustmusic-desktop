use crate::models::sidebar::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::artists::ArtistCheckMsg;
use crate::models::tracks::TrackMenuAt;
use crate::models::layout::ToastKind;
use crate::models::sidebar::PinKind;
use crate::commands::library::{artist_split_command, library_command, pin_command};

/// Épingle ou désépingle, puis recharge les épingles et le dit.
pub(crate) fn set_pinned(me: &mut MusicApp, kind: PinKind, library: i64, id: String, pinned: bool, title: String) {
    let app = me.backend.app.clone();
    me.backend.call(
        async move {
            match (kind, pinned) {
                (PinKind::Album, true) => pin_command::pin_album(app.state(), library, id).await,
                (PinKind::Album, false) => pin_command::unpin_album(app.state(), library, id).await,
                (PinKind::Artist, true) => pin_command::pin_artist(app.state(), library, id).await,
                (PinKind::Artist, false) => pin_command::unpin_artist(app.state(), library, id).await,
            }
        },
        move |me, r| {
            match r {
                Ok(()) => {
                    me.toast(ToastKind::Success, title, Some(t(if pinned { "sidebar.pinned" } else { "sidebar.unpinned" })));
                    me.sidebar_load_pins();
                }
                Err(e) => me.toast(ToastKind::Error, t("notify.error"), Some(e)),
            }
            true
        },
    );
}

impl MusicApp {
    pub(crate) fn sidebar_item_open(&mut self, at: TrackMenuAt, kind: PinKind, id: String) -> bool {
        let Some(library) = self.data.library_id else { return false };
        let Some(pin) = self.views.sidebar.pins.iter().find(|p| p.kind == kind.key() && p.id == id) else { return false };
        let title = pin.title.clone();
        self.menu = None;
        let doubtful = kind == PinKind::Artist && artist_split_command::artist_is_doubtful(title.clone());
        self.views.sidebar.item_menu = Some(ItemMenu { at, kind, id, title, library_id: library, pinned: true, doubtful });
        true
    }

    pub(crate) fn sidebar_item_update(&mut self, msg: ItemMsg) -> bool {
        // Tout est lu avant la fermeture : le menu disparaît avec son état.
        let Some(m) = self.views.sidebar.item_menu.take() else { return false };
        match msg {
            ItemMsg::Play => {
                let app = self.backend.app.clone();
                let (kind, library, id) = (m.kind, m.library_id, m.id);
                self.backend.call(
                    async move {
                        match kind {
                            PinKind::Album => library_command::get_tracks_by_album(app.state(), library, id).await,
                            PinKind::Artist => library_command::get_tracks_by_artist(app.state(), library, id).await,
                        }
                    },
                    |me, r| match r {
                        Ok(tracks_list) if !tracks_list.is_empty() => {
                            let first = tracks_list[0].path.clone();
                            me.play_tracks_view(tracks_list, &first);
                            true
                        }
                        Ok(_) => false,
                        Err(e) => {
                            me.toast(ToastKind::Error, t("notify.error"), Some(e));
                            true
                        }
                    },
                );
            }
            ItemMsg::Open => return self.handle(Msg::Go(MusicApp::sidebar_pin_route(m.kind.key(), &m.id))),
            ItemMsg::Pin => set_pinned(self, m.kind, m.library_id, m.id, !m.pinned, m.title),
            ItemMsg::Check => return self.artist_check_update(ArtistCheckMsg::One(m.title)),
            ItemMsg::Close => {}
        }
        true
    }

    /// Épingle l'album ou l'artiste en cours, depuis le menu d'une section.
    pub(crate) fn sidebar_pin_playing(&mut self, kind: PinKind) -> bool {
        self.menu = None;
        let (Some(id), Some(library)) = (self.sidebar_playing(kind).map(str::to_string), self.data.library_id) else { return true };
        let title = self.sidebar_playing_title(kind);
        set_pinned(self, kind, library, id, true, title);
        true
    }
}
