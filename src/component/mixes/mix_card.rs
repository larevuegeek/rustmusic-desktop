use crate::component::*;
use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use crate::models::app::Menu;
use crate::models::app::Msg;
use crate::models::home::Mix;
use karbon_ui::intl::t;
use crate::helper::covers::mosaic;
use karbon_ui::ui::prelude::*;

/// Carte d'un mix, variante « horizontale », avec son menu d'actions.
#[derive(Default)]
pub struct MixCard {
    pub mix: Option<Mix>,
    /// Position dans « Pour vous » (`Menu::Mix`).
    pub index: usize,
    /// Le menu d'actions est ouvert.
    pub menu: bool,
}

impl Component<Assets> for MixCard {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let mix = self.mix.expect("MixCard needs a mix");
        let actions = crate::mapper::mixes::mix_list::mix_actions(self.index);
        ui_file!("/ui/templates/mixes/MixCard.ui", context = cx, styles = css)
    }
}
