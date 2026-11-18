use crate::models::albums::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::layout::ToastKind;
use crate::commands::library::library_command;

impl MusicApp {
    pub(crate) fn cover_search_update(&mut self, msg: CoverMsg) -> bool {
        match msg {
            CoverMsg::Open(album_id, query) => {
                self.views.collection_menu = None;
                self.views.cover_search = Some(CoverSearch { album_id, query, results: Vec::new(), loading: false, applying: false, selected: None, seq: 0 });
                return self.cover_search_run();
            }
            CoverMsg::Close => self.views.cover_search = None,
            CoverMsg::Query(v) => {
                let Some(c) = self.views.cover_search.as_mut() else { return false };
                c.query = v;
            }
            CoverMsg::Search => return self.cover_search_run(),
            CoverMsg::Select(i) => {
                let Some(c) = self.views.cover_search.as_mut() else { return false };
                if c.applying {
                    return false;
                }
                c.selected = if c.selected == Some(i) { None } else { Some(i) };
            }
            CoverMsg::Apply => {
                let Some(c) = self.views.cover_search.as_mut() else { return false };
                let Some(hit) = c.selected.and_then(|i| c.results.get(i)).cloned() else { return false };
                c.applying = true;
                let (album, url) = (c.album_id.clone(), hit.cover_xl.clone());
                let app = self.backend.app.clone();
                self.backend.call(async move { library_command::apply_deezer_cover(app.state(), album, url).await }, move |me, r| {
                    match r {
                        Ok(_) => {
                            me.views.cover_search = None;
                            me.load_library_data();
                            me.load_page();
                            me.toast(ToastKind::Success, t("tags.cover"), Some(t("popins.cover_applied").replace("{title}", &hit.title)));
                        }
                        Err(e) => {
                            if let Some(c) = me.views.cover_search.as_mut() {
                                c.applying = false;
                            }
                            me.toast(ToastKind::Error, t("popins.error_title"), Some(e));
                        }
                    }
                    true
                });
            }
        }
        true
    }

    pub(crate) fn cover_search_run(&mut self) -> bool {
        let Some(c) = self.views.cover_search.as_mut() else { return false };
        let query = c.query.trim().to_string();
        if query.is_empty() {
            return true;
        }
        c.seq += 1;
        c.loading = true;
        c.results.clear();
        c.selected = None;
        let seq = c.seq;
        self.backend.call(async move { library_command::search_deezer_covers(query, Some(12)).await }, move |me, r| {
            let Some(c) = me.views.cover_search.as_mut() else { return false };
            if c.seq != seq {
                return false;
            }
            c.loading = false;
            match r {
                Ok(list) => {
                    let smalls: Vec<String> = list.iter().map(|x| x.cover_small.clone()).collect();
                    c.results = list.into_iter().map(|x| CoverHit { title: x.title, artist: x.artist, cover_xl: x.cover_xl, thumb: None }).collect();
                    for (i, url) in smalls.into_iter().enumerate() {
                        me.backend.call(async move { crate::service::radio::radio_service::logo(&url).await }, move |me, path| {
                            let Some(c) = me.views.cover_search.as_mut() else { return false };
                            if c.seq != seq {
                                return false;
                            }
                            if let Some(h) = c.results.get_mut(i) {
                                h.thumb = path;
                            }
                            true
                        });
                    }
                }
                Err(e) => me.toast(ToastKind::Error, t("popins.error_title"), Some(e)),
            }
            true
        });
        true
    }
}
