use crate::mapper::settings::*;
use crate::app::music_app::MusicApp;

impl MusicApp {
    /// Garde l'ordre d'origine ; la dernière section reste.
    pub(crate) fn toggle_library_tab(&mut self, key: &str) {
        let kept_items = self.library_tab_keys();
        let next_item: Vec<&str> = if kept_items.contains(&key) {
            kept_items.iter().copied().filter(|c| *c != key).collect()
        } else {
            LIBRARY_TABS.iter().map(|(k, _)| *k).filter(|c| *c == key || kept_items.contains(c)).collect()
        };
        if next_item.is_empty() {
            return;
        }
        self.set_setting("library_tabs", serde_json::to_string(&next_item).unwrap_or_default());
    }
}
