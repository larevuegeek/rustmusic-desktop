use crate::component::*;
use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use crate::models::app::Msg;
use crate::models::home::PlaylistRow;
use crate::helper::color::hex;
use karbon_ui::intl::t;
use crate::helper::covers::mosaic;
use karbon_ui::ui::prelude::*;

/// Carte carrée d'une playlist.
#[derive(Default)]
pub struct PlaylistCard {
    pub playlist: Option<PlaylistRow>,
}

impl Component<Assets> for PlaylistCard {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let p = self.playlist.expect("PlaylistCard needs a playlist");
        let go = Msg::Go(p.route.clone());
        ui_file!("/ui/templates/playlists/PlaylistCard.ui", context = cx, styles = css)
    }
}
