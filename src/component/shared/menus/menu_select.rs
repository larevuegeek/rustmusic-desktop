use crate::models::app::Choice;
use crate::app::resources::assets::Assets;
use crate::component::{Icon, MenuItem};
use crate::app::resources::css;
use crate::models::app::Menu;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

/// Le choix courant et la liste des options.
#[derive(Default)]
pub struct MenuSelect {
    /// Identifiant du déclencheur (`Menu::Popup`). Pas `id` : sur un composant, karbon-ui
    /// le prend pour l'identifiant de l'élément et ne le transmet pas.
    pub anchor: &'static str,
    pub open: bool,
    pub label: String,
    pub icon: String,
    pub indicator: String,
    pub options: Vec<Choice>,
    /// Masque le libellé quand la place manque (`@max-xl:hidden`).
    pub narrow: bool,
}

impl Component<Assets> for MenuSelect {
    fn render(self, cx: &TemplateContext<'_, Assets>, mut slots: Slots) -> Element {
        let id = self.anchor;
        let indicator = if self.indicator.is_empty() {
            "material-symbols:expand-more-rounded".to_string()
        } else {
            self.indicator.clone()
        };
        ui_file!("/ui/templates/shared/menus/MenuSelect.ui", context = cx, slots = slots, styles = css)
    }
}
