use karbon_ui::intl::t;
use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::app::Msg;
use crate::models::playlists::GroupHead;
use crate::models::playlists::SmartMsg;
use karbon_ui::ui::prelude::*;

/// Le liant d'un groupe, et sa croix hors de la racine.
#[derive(Default)]
pub struct RuleGroupHead {
    pub g: GroupHead,
}

impl Component<Assets> for RuleGroupHead {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let g = self.g;
        ui_file!("/ui/templates/playlists/PlaylistRuleGroupHead.ui", context = cx, styles = css)
    }
}
