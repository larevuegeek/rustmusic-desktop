use crate::component::*;
use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

pub struct SidebarButton {
    pub icon: String,
    pub label: String,
    pub size: f32,
    pub onclick: Option<Msg>,
}

impl Default for SidebarButton {
    fn default() -> Self {
        Self {
            icon: String::new(),
            label: String::new(),
            size: 20.,
            onclick: None,
        }
    }
}

impl Component<Assets> for SidebarButton {
    fn render(self, cx: &TemplateContext<'_, Assets>, mut slots: Slots) -> Element {
        let onclick = self.onclick.clone().unwrap_or(Msg::Noop);
        ui_file!(
            "/ui/templates/sidebar/SidebarButton.ui",
            context = cx,
            slots = slots,
            styles = css
        )
    }
}
