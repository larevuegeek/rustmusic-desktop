use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use karbon_ui::ui::prelude::*;

/// Palier (Hi-Res, sans perte, compressé) et fiche.
#[derive(Default)]
pub struct QualityBadge {
    pub tier: Option<crate::models::tracks::Tier>,
    pub text: String,
}

impl Component<Assets> for QualityBadge {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        use crate::models::tracks::Tier;
        let tier = self.tier.unwrap_or(Tier::Lossy);
        ui_file!("/ui/templates/shared/display/QualityBadge.ui", context = cx, styles = css)
    }
}
