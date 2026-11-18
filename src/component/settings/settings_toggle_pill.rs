use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::settings::Pill;
use karbon_ui::ui::prelude::*;

#[derive(Default)]
pub struct TogglePill {
    pub p: Option<Pill>,
}

impl Component<Assets> for TogglePill {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        ui_file!("/ui/templates/settings/shared/TogglePill.ui", context = cx, styles = css)
    }
}
