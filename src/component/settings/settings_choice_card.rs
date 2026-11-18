use crate::component::*;
use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use crate::mapper::settings::theme_palette;
use crate::models::settings::Card;
use karbon_ui::ui::prelude::*;

/// Carte de choix et ses aperçus.
#[derive(Default)]
pub struct ChoiceCard {
    pub c: Option<Card>,
}

impl Component<Assets> for ChoiceCard {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        ui_file!("/ui/templates/settings/shared/ChoiceCard.ui", context = cx, styles = css)
    }
}
