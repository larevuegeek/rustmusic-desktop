use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use karbon_ui::ui::prelude::*;

/// Interrupteur (taille normale, vert).
#[derive(Default)]
pub struct ToggleSwitch {
    pub on: bool,
}

impl Component<Assets> for ToggleSwitch {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        ui_file!("/ui/templates/settings/shared/ToggleSwitch.ui", context = cx, styles = css)
    }
}
