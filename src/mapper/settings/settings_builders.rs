//! Les briques des pages de réglages : variantes des lignes (`OptionRow`), des boutons
//! discrets (`Ghost`) et des groupes (`OptionGroup`).

use crate::models::settings::*;

impl Ghost {
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }
}

impl OptionRow {
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn value(mut self, value: String) -> Self {
        self.value = Some(value);
        self
    }

    pub fn lead(mut self, icon: &'static str, hue: i32) -> Self {
        self.lead = Some((icon, hue));
        self
    }
}

impl OptionGroup {
    pub fn with_action(mut self, action: GroupAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn with_sidebar(mut self, sidebar: [bool; 4]) -> Self {
        self.sidebar = Some(sidebar);
        self
    }
}
