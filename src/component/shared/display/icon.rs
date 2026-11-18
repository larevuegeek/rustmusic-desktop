use crate::app::resources::assets::Assets;
use karbon_ui::ui::prelude::*;

/// `<Icon icon="lucide:more" width={16} />` : l'icône Iconify compilée, `width` × `width`,
/// dessinée avec `currentColor`. `thick` prend la variante de `assets/icons-thick/` (trait
/// `.sb-icone` des tuiles) quand elle existe. Une icône absente de `assets/icons/` est
/// remplacée par une note de musique (voir `icons.rs` pour en ajouter une).
pub struct Icon {
    pub icon: String,
    pub width: f32,
    pub thick: bool,
}

impl Default for Icon {
    fn default() -> Self {
        Self {
            icon: String::new(),
            width: 20.,
            thick: false,
        }
    }
}

impl Component<Assets> for Icon {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let size = self.width * cx.scale;
        let thick = if self.thick { crate::app::resources::icons::thick::svg(cx, &self.icon) } else { None };
        thick
            .or_else(|| crate::app::resources::icons::plain::svg(cx, &self.icon))
            .or_else(|| {
                log::warn!("icône {} absente de ui/assets/icons", self.icon);
                crate::app::resources::icons::plain::svg(cx, "lucide:music")
            })
            .expect("lucide:music est compilée")
            .width(size)
            .height(size)
    }
}
