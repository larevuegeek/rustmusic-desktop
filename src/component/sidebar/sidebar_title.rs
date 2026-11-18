use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

/// Titre de section de la barre latérale ; les actions passent par un slot nommé.
pub struct SidebarTitle {
    pub title: String,
    pub help: String,
    pub onclick: Msg,
}

impl Default for SidebarTitle {
    fn default() -> Self {
        Self {
            title: String::new(),
            help: String::new(),
            onclick: Msg::Noop,
        }
    }
}

impl Component<Assets> for SidebarTitle {
    fn render(self, cx: &TemplateContext<'_, Assets>, mut slots: Slots) -> Element {
        ui_file!(
            "/ui/templates/sidebar/SidebarTitle.ui",
            context = cx,
            slots = slots,
            styles = css
        )
    }
}
