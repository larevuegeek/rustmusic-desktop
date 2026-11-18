use crate::models::player::*;
use crate::app::music_app::MusicApp;
use crate::helper::lrc::parse_lrc;
use crate::models::app::Menu;
use crate::commands::tags::lyrics_command;

impl MusicApp {
    /// Charge les paroles du morceau en cours si le volet est ouvert et qu'il a changé.
    pub(crate) fn lyrics_sync(&mut self) -> bool {
        if self.menu != Some(Menu::Lyrics) && !self.views.mini.lyrics_open() {
            return false;
        }
        let path = self.data.player.path.clone();
        if path == self.views.lyrics.path {
            return false;
        }
        self.views.lyrics = LyricsState { path: path.clone(), ..Default::default() };
        let Some(path) = path else { return true };
        self.lyrics_fetch(path, false);
        true
    }

    pub(crate) fn lyrics_fetch(&mut self, path: String, refresh: bool) {
        let l = &mut self.views.lyrics;
        if refresh {
            l.refreshing = true;
        } else {
            l.status = LyricsStatus::Loading;
        }
        l.error = None;
        let app = self.backend.app.clone();
        let request = path.clone();
        self.backend.call(
            async move {
                if refresh {
                    lyrics_command::refresh_lyrics(app.state(), request).await
                } else {
                    lyrics_command::get_lyrics(app.state(), request).await
                }
            },
            move |me, r| {
                let l = &mut me.views.lyrics;
                if l.path.as_deref() != Some(path.as_str()) {
                    return false;
                }
                l.refreshing = false;
                match r {
                    Ok(Some(p)) => {
                        let source = serde_json::to_value(&p.source).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                        if source == "none" || (p.plain.is_none() && p.synced.is_none()) {
                            l.status = LyricsStatus::Empty;
                        } else {
                            l.lines = p.synced.as_deref().map(parse_lrc).unwrap_or_default();
                            l.plain = p.plain;
                            l.source = source;
                            l.status = LyricsStatus::Ready;
                        }
                    }
                    Ok(None) => l.status = LyricsStatus::Empty,
                    Err(e) => {
                        l.status = LyricsStatus::Error;
                        l.error = Some(e);
                    }
                }
                true
            },
        );
    }
}
