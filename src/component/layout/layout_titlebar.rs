use crate::component::*;
use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use crate::models::app::Msg;
use karbon_ui::intl::t;
use crate::models::player::MiniMsg;
use karbon_ui::ui::prelude::*;

/// Marque, contenu de la page (slot), contrôles.
#[derive(Default)]
pub struct Titlebar {
    pub with_sidebar: bool,
    /// La barre latérale est repliée : le bouton la déplie.
    pub collapsed: bool,
    /// Style des boutons de la fenêtre : `macos`, `windows` ou `linux`.
    pub controls: &'static str,
    /// Boutons de la fenêtre à gauche, avant la marque.
    pub controls_left: bool,
}

impl Component<Assets> for Titlebar {
    fn render(self, cx: &TemplateContext<'_, Assets>, mut slots: Slots) -> Element {
        ui_file!(
            "/ui/templates/layout/Titlebar.ui",
            context = cx,
            slots = slots,
            styles = css
        )
    }
}
