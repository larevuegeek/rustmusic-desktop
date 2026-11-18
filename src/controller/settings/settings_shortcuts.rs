use crate::app::music_app::MusicApp;
use crate::models::settings::SettingsMsg;

impl MusicApp {
    pub(crate) fn settings_shortcut_update(&mut self, msg: SettingsMsg) -> bool {
        match msg {
            SettingsMsg::ShortcutEdit(action) => {
                let s = &mut self.views.settings.shortcut_editing;
                *s = if s.as_deref() == Some(action) { None } else { Some(action.to_string()) };
            }
            SettingsMsg::ShortcutReset(action) => {
                let mut m = self.shortcut_overrides();
                m.remove(action);
                let v = if m.is_empty() { String::new() } else { serde_json::Value::Object(m).to_string() };
                self.set_setting("shortcuts", v);
            }
            SettingsMsg::ShortcutResetAll => {
                self.views.settings.shortcut_editing = None;
                self.set_setting("shortcuts", "");
            }
            _ => return false,
        }
        true
    }
}
