use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::settings::Ghost;
use karbon_ui::ui::prelude::*;

#[derive(Default)]
pub struct GhostButton {
    pub g: Option<Ghost>,
}

impl Component<Assets> for GhostButton {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        ui_file!("/ui/templates/shared/buttons/GhostButton.ui", context = cx, styles = css)
    }
}
