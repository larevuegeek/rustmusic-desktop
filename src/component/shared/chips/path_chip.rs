use karbon_ui::intl::t;
use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

/// Dossier, chemin, copie. Écart : pas de presse-papiers côté
/// application, le bouton de copie ne fait rien.
#[derive(Default)]
pub struct PathChip {
    pub path: String,
    pub reveal: String,
}

impl Component<Assets> for PathChip {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        ui_file!("/ui/templates/shared/chips/PathChip.ui", context = cx, styles = css)
    }
}
