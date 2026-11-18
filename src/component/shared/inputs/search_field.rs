use crate::app::resources::assets::Assets;
use crate::component::Icon;
use crate::app::resources::css;
use crate::models::app::Field;
use crate::models::app::Msg;
use karbon_ui::ui::prelude::*;

/// Loupe, champ, effacement.
pub struct SearchField {
    pub value: String,
    pub placeholder: String,
    pub clear_label: String,
    pub field: Option<Field>,
}

impl Default for SearchField {
    fn default() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            clear_label: String::new(),
            field: None,
        }
    }
}

impl Component<Assets> for SearchField {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        let field = self.field.expect("SearchField needs a field");
        ui_file!("/ui/templates/shared/inputs/SearchField.ui", context = cx, styles = css)
    }
}
