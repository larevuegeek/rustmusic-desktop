use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use karbon_ui::ui::prelude::*;

/// Indicateur de chargement (`material-symbols:progress-activity` qui tourne).
pub struct Spinner {
    pub width: f32,
}

impl Default for Spinner {
    fn default() -> Self {
        Self { width: 26. }
    }
}

impl Component<Assets> for Spinner {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        ui_file!("/ui/templates/shared/display/Spinner.ui", context = cx, styles = css)
    }
}
