use karbon_ui::intl::t;
use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::app::Msg;
use crate::models::playlists::RuleView;
use crate::models::playlists::SmartMsg;
use karbon_ui::ui::prelude::*;

/// Une condition : champ, opérateur, valeur(s), retirer.
#[derive(Default)]
pub struct RuleRow {
    pub r: RuleView,
}

impl Component<Assets> for RuleRow {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let r = self.r;
        ui_file!("/ui/templates/playlists/PlaylistRuleRow.ui", context = cx, styles = css)
    }
}
