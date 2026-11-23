use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

/// Rond cerclé, vert une fois enfoncé.
#[derive(Default)]
pub struct RoundButton {
    pub icon: String,
    pub title: String,
    pub pressed: bool,
    pub disabled: bool,
    pub onclick: Option<Msg>,
    /// Identifiant de l'élément (ancre d'un menu) ; vide : aucun.
    pub anchor: String,
}

impl Component<Assets> for RoundButton {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let onclick = self.onclick.clone().unwrap_or(Msg::Noop);
        ui_file!("/ui/templates/shared/buttons/RoundButton.ui", context = cx, styles = css)
    }
}
