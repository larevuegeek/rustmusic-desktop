use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

/// Pilule « Lire » (`lg`) ou rond vert (`md`).
#[derive(Default)]
pub struct PlayButton {
    pub label: String,
    pub title: String,
    pub large: bool,
    pub muted: bool,
    pub onclick: Option<Msg>,
}

impl Component<Assets> for PlayButton {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let onclick = self.onclick.clone().unwrap_or(Msg::Noop);
        let title = if self.title.is_empty() { self.label.clone() } else { self.title.clone() };
        ui_file!("/ui/templates/shared/buttons/PlayButton.ui", context = cx, styles = css)
    }
}
