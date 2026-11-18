use karbon_ui::intl::t;
use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::app::Msg;
use crate::models::playlists::GroupHead;
use crate::models::playlists::SmartMsg;
use karbon_ui::ui::prelude::*;

/// « Condition » et « Groupe ».
#[derive(Default)]
pub struct RuleGroupFoot {
    pub g: GroupHead,
}

impl Component<Assets> for RuleGroupFoot {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let g = self.g;
        ui_file!("/ui/templates/playlists/PlaylistRuleGroupFoot.ui", context = cx, styles = css)
    }
}
