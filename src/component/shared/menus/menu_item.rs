use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

/// Icône, libellé, fin (compteur, coche) ; courant en vert.
#[derive(Default)]
pub struct MenuItem {
    pub icon: String,
    pub active: bool,
    pub danger: bool,
    pub onclick: Option<Msg>,
}

impl Component<Assets> for MenuItem {
    fn render(self, cx: &TemplateContext<'_, Assets>, mut slots: Slots) -> Element {
        let onclick = self.onclick.clone().unwrap_or(Msg::Noop);
        ui_file!("/ui/templates/shared/menus/MenuItem.ui", context = cx, slots = slots, styles = css)
    }
}
