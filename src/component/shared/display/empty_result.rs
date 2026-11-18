use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

/// Liste vide après filtrage.
#[derive(Default)]
pub struct EmptyResult {
    pub message: String,
    pub clear_label: String,
    pub onclear: Option<Msg>,
}

impl Component<Assets> for EmptyResult {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        ui_file!("/ui/templates/shared/display/EmptyResult.ui", context = cx, styles = css)
    }
}
