use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

/// Icône, libellé facultatif (sinon carré), enfoncé en vert.
#[derive(Default)]
pub struct ToolbarButton {
    pub icon: String,
    pub label: String,
    pub title: String,
    pub pressed: bool,
    pub onclick: Option<Msg>,
}

impl Component<Assets> for ToolbarButton {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let onclick = self.onclick.clone().unwrap_or(Msg::Noop);
        ui_file!("/ui/templates/shared/buttons/ToolbarButton.ui", context = cx, styles = css)
    }
}
