use crate::models::app::Choice;
use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use karbon_ui::ui::prelude::*;

/// Choix segmenté, variante `subtle` ou `plein`.
#[derive(Default)]
pub struct SegmentedControl {
    pub label: String,
    pub options: Vec<Choice>,
    pub subtle: bool,
}

impl Component<Assets> for SegmentedControl {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        ui_file!("/ui/templates/shared/inputs/SegmentedControl.ui", context = cx, styles = css)
    }
}
