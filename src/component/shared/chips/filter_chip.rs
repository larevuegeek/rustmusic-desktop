use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

/// Bascule (verte une fois active) ou ouvre un menu.
#[derive(Default)]
pub struct FilterChip {
    pub icon: String,
    pub pressed: bool,
    pub menu: bool,
    pub title: String,
    pub onclick: Option<Msg>,
    /// Identifiant (ancre d'un menu).
    pub anchor: String,
}

impl Component<Assets> for FilterChip {
    fn render(self, cx: &TemplateContext<'_, Assets>, mut slots: Slots) -> Element {
        let onclick = self.onclick.clone().unwrap_or(Msg::Noop);
        // Un identifiant unique : celui de l'ancre d'un menu, sinon tiré du libellé.
        let anchor = if self.anchor.is_empty() {
            format!("chip.{}", self.title)
        } else {
            self.anchor.clone()
        };
        ui_file!("/ui/templates/shared/chips/FilterChip.ui", context = cx, slots = slots, styles = css)
    }
}
