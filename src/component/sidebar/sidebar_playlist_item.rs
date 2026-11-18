use crate::component::*;
use crate::app::resources::assets::Assets;
use crate::models::app::Msg;
use crate::models::home::PlaylistRow;
use crate::models::sidebar::RowTarget;
use karbon_ui::ui::prelude::*;

/// Une `SidebarRow` pour une playlist épinglée.
#[derive(Default)]
pub struct PlaylistItem {
    pub playlist: Option<PlaylistRow>,
    /// Barre repliée (`SidebarRow::collapsed`).
    pub collapsed: bool,
}

impl Component<Assets> for PlaylistItem {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let p = self.playlist.expect("PlaylistItem needs a playlist");
        let go = Msg::Go(p.route.clone());
        // Titres likés et Récemment joués n'ont pas de menu (`SidebarRow` sans `onmenu`).
        let target = match p.route {
            crate::models::app::Route::Playlist(id) => Some(RowTarget::Playlist(id)),
            _ => None,
        };
        ui! {@context(cx); @styles(css);
            <SidebarRow title={p.name.clone()} subtitle={p.subtitle.clone()} active={p.active}
                        target={target} collapsed={self.collapsed} onclick={go}>
                <SidebarThumbnail slot="vignette" covers={p.covers.clone()}
                                  color={p.color.clone()} icon={p.icon.clone()} />
            </SidebarRow>
        }
    }
}
