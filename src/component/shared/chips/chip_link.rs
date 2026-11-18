use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

/// Lien en pastille (genre…).
#[derive(Default)]
pub struct ChipLink {
    pub label: String,
    pub title: String,
    pub onclick: Option<Msg>,
}

impl Component<Assets> for ChipLink {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let onclick = self.onclick.clone().unwrap_or(Msg::Noop);
        ui_file!("/ui/templates/shared/chips/ChipLink.ui", context = cx, styles = css)
    }
}
