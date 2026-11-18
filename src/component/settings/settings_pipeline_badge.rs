use karbon_ui::intl::t;
use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use karbon_ui::ui::prelude::*;

/// Pastille de la chaîne audio, thème sombre.
#[derive(Default)]
pub struct PipelineBadge {
    pub mode: String,
}

impl Component<Assets> for PipelineBadge {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let label = t(crate::mapper::settings::badge_keys(&self.mode).0);
        ui_file!("/ui/templates/settings/shared/PipelineBadge.ui", context = cx, styles = css)
    }
}
