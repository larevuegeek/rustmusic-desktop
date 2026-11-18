use crate::component::*;
use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use crate::helper::color::hex;
use crate::helper::covers::mosaic;
use karbon_ui::ui::prelude::*;

#[derive(Default)]
pub struct SidebarThumbnail {
    pub covers: Vec<String>,
    pub color: String,
    pub icon: String,
}

impl Component<Assets> for SidebarThumbnail {
    fn render(self, cx: &TemplateContext<'_, Assets>, mut slots: Slots) -> Element {
        ui_file!(
            "/ui/templates/sidebar/SidebarThumbnail.ui",
            context = cx,
            slots = slots,
            styles = css
        )
    }
}
