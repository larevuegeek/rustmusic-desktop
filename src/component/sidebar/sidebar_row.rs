use crate::component::*;
use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use crate::models::app::Msg;
use crate::models::tracks::TrackMenuAt;
use crate::models::sidebar::RowTarget;
use crate::models::sidebar::SidebarMsg;
use karbon_ui::ui::popover::ContextMenuRequest;
use karbon_ui::ui::prelude::*;

/// Ligne de la barre latérale ; la vignette passe par un slot nommé.
pub struct SidebarRow {
    pub title: String,
    pub subtitle: String,
    pub active: bool,
    /// L'album ou l'artiste du morceau en cours (égaliseur, titre vert).
    pub playing: bool,
    /// `onmenu` : ce que la ligne désigne. Le bouton « … » (barre dépliée) et le clic
    /// droit ouvrent son menu.
    pub target: Option<RowTarget>,
    /// Barre repliée : la vignette de 44 px seule, centrée, le titre en infobulle.
    pub collapsed: bool,
    pub onclick: Msg,
}

impl Default for SidebarRow {
    fn default() -> Self {
        Self {
            title: String::new(),
            subtitle: String::new(),
            active: false,
            playing: false,
            target: None,
            collapsed: false,
            onclick: Msg::Noop,
        }
    }
}

impl Component<Assets> for SidebarRow {
    fn render(self, cx: &TemplateContext<'_, Assets>, mut slots: Slots) -> Element {
        let more_id = self.target.as_ref().map(RowTarget::more_id).unwrap_or_default();
        let onmenu = match &self.target {
            Some(c) => Msg::Sidebar(SidebarMsg::RowMenu(TrackMenuAt::Named(more_id.clone()), c.clone())),
            None => Msg::Noop,
        };
        let target = self.target.clone();
        let oncontextmenu = move |q: ContextMenuRequest| match &target {
            Some(c) => Msg::Sidebar(SidebarMsg::RowMenu(TrackMenuAt::from_request(&q), c.clone())),
            None => Msg::Noop,
        };
        ui_file!(
            "/ui/templates/sidebar/SidebarRow.ui",
            context = cx,
            slots = slots,
            styles = css
        )
    }
}
