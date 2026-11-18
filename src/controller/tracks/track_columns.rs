use crate::models::tracks::*;
use crate::app::music_app::MusicApp;
use crate::mapper::tracks::fixed;
use crate::mapper::tracks::shown_keys;
use crate::mapper::tracks::tag_title;
use crate::commands::library::library_command;

/// Tags déjà servis par une colonne bâtie.
pub(crate) const COVERED: [&str; 9] = ["title", "artist", "album", "album_artist", "year", "genre", "track_number", "disc_number", "duration"];

pub(crate) const DEFAULT: [&str; 7] = ["index", "cover", "title", "artist", "album", "rating", "duration"];

/// Intitulé d'une clé (colonne bâtie ou tag).
pub fn title(key: &str) -> String {
    if let Some(tag) = key.strip_prefix("tag:") {
        return tag_title(tag);
    }
    fixed().into_iter().find(|c| c.key == key).map(|c| c.label()).unwrap_or_else(|| key.to_string())
}

impl MusicApp {
    pub(crate) fn columns_update(&mut self, msg: ColumnsMsg) -> bool {
        match msg {
            ColumnsMsg::Resize(key, event) => return self.column_resize(key, event),
            ColumnsMsg::Fit(key) => return self.column_fit(&key),
            ColumnsMsg::Original(key) => return self.column_original(&key),
            ColumnsMsg::Menu(key, at) => return self.column_menu(key, at),
            ColumnsMsg::CloseMenu => return self.views.column_menu.take().is_some(),
            ColumnsMsg::ResetWidths => {
                self.views.column_menu = None;
                self.set_setting("track_column_widths", "{}");
                return true;
            }
            _ => {}
        }
        if msg == ColumnsMsg::Open {
            let choice = shown_keys(self.data.setting("track_columns"));
            self.menu = None;
            self.views.columns = Some(ColumnsPopin { choice, tags: None, filter: String::new() });
            let library = self.data.library_id;
            let app = self.backend.app.clone();
            self.backend.call(
                async move {
                    match library {
                        Some(l) => library_command::get_library_tag_keys(app.state(), l).await.map(|v| v.into_iter().map(|c| (c.key, c.filled)).collect()).unwrap_or_default(),
                        None => Vec::new(),
                    }
                },
                |me, tags: Vec<(String, i64)>| {
                    let Some(c) = me.views.columns.as_mut() else { return false };
                    c.tags = Some(tags.into_iter().filter(|(k, _)| !COVERED.contains(&k.as_str())).collect());
                    true
                },
            );
            return true;
        }
        let Some(c) = self.views.columns.as_mut() else { return false };
        match msg {
            ColumnsMsg::Close => self.views.columns = None,
            ColumnsMsg::Filter(v) => c.filter = v,
            ColumnsMsg::Toggle(k) => {
                if let Some(i) = c.choice.iter().position(|x| *x == k) {
                    c.choice.remove(i);
                } else {
                    c.choice.push(k);
                }
            }
            ColumnsMsg::Move(i, down) => {
                let j = if down { i + 1 } else { i.wrapping_sub(1) };
                if i < c.choice.len() && j < c.choice.len() {
                    c.choice.swap(i, j);
                }
            }
            ColumnsMsg::Reset => c.choice = DEFAULT.iter().map(|s| s.to_string()).collect(),
            ColumnsMsg::Apply => {
                let json = serde_json::to_string(&c.choice).unwrap_or_default();
                self.views.columns = None;
                self.set_setting("track_columns", json);
            }
            ColumnsMsg::Open
            | ColumnsMsg::Resize(..)
            | ColumnsMsg::Fit(_)
            | ColumnsMsg::Original(_)
            | ColumnsMsg::Menu(..)
            | ColumnsMsg::CloseMenu
            | ColumnsMsg::ResetWidths => {}
        }
        true
    }
}
