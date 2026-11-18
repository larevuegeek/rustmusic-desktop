use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

/// Bouton discret sans cadre.
#[derive(Default)]
pub struct TextButton {
    pub icon: String,
    pub label: String,
    pub onclick: Option<Msg>,
}

impl Component<Assets> for TextButton {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let onclick = self.onclick.clone().unwrap_or(Msg::Noop);
        ui_file!("/ui/templates/shared/buttons/TextButton.ui", context = cx, styles = css)
    }
}
