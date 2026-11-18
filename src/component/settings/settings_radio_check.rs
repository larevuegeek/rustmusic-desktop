use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use karbon_ui::ui::prelude::*;

#[derive(Default)]
pub struct RadioCheck {
    pub checked: bool,
}

impl Component<Assets> for RadioCheck {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        ui_file!("/ui/templates/settings/shared/RadioCheck.ui", context = cx, styles = css)
    }
}
