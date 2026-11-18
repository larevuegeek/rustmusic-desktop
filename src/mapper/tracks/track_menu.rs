use crate::models::tracks::*;
use karbon_ui::ui::geometry::PointF;
use karbon_ui::ui::prelude::{Anchor, ElementId};
use crate::app::music_app::MusicApp;

impl TrackMenuAt {
    pub fn anchor(&self) -> Anchor {
        match self {
            TrackMenuAt::Point(x, y) => Anchor::Point(PointF { x: *x, y: *y }),
            TrackMenuAt::Named(n) => Anchor::Element(ElementId::named(n)),
        }
    }

    /// Écart au déclencheur : 4 px sous un bouton « … » (`r.bottom + 4`), aucun au pointeur.
    pub fn offset(&self) -> f32 {
        match self {
            TrackMenuAt::Point(..) => 0.,
            TrackMenuAt::Named(_) => 4.,
        }
    }

    /// Depuis la demande de karbon-ui (clic droit, Maj+F10).
    pub fn from_request(r: &karbon_ui::ui::popover::ContextMenuRequest) -> Self {
        TrackMenuAt::Point(r.position.x, r.position.y)
    }
}

impl MusicApp {
    /// Les playlists qui acceptent ce morceau.
    pub fn track_menu_playlists(&self) -> Vec<&crate::entity::playlist::playlist::Playlist> {
        let Some(m) = self.views.track_menu.as_ref() else { return Vec::new() };
        let server = crate::service::external_server::track_service::is_remote(&m.track.path);
        self.data.playlists.iter().filter(|p| !p.is_smart && p.server_id.is_some() == server).collect()
    }
}
