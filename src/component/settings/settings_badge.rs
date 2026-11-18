use crate::app::resources::assets::Assets;
use crate::app::resources::css;
use karbon_ui::ui::prelude::*;

/// Ton « neutral », « green », « amber » ou « purple ».
#[derive(Default)]
pub struct SettingsBadge {
    pub text: String,
    pub tone: String,
}

impl Component<Assets> for SettingsBadge {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        ui_file!("/ui/templates/settings/shared/Badge.ui", context = cx, styles = css)
    }
}
