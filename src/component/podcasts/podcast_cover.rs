use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use karbon_ui::ui::prelude::*;

/// La pochette, sinon un dégradé violet et l'antenne (la boîte
/// de l'appelant fixe taille et arrondi).
#[derive(Default)]
pub struct PodcastCover {
    pub cover: Option<String>,
    pub icon: i32,
}

impl Component<Assets> for PodcastCover {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        ui_file!("/ui/templates/podcasts/PodcastCover.ui", context = cx, styles = css)
    }
}
