use crate::models::tags::*;
use crate::mapper::tags::*;
use std::collections::HashSet;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use karbon_ui::storage;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::models::layout::PopinSize;
use crate::models::tags::TagsMsg;
use crate::commands::tags::metadata_command::{self, LocalTrackPayload};
use crate::commands::tags::tag_command;
use crate::core::metadata_source::model::Source;

pub(crate) const SOURCE_KEY: &str = "tags:source";

pub fn cell_key(path: &str, field: &str) -> String {
    format!("{path} {field}")
}

impl MusicApp {
    pub(crate) fn source_state(&mut self) -> Option<&mut SourceState> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::Source(s)) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn source_update(&mut self, msg: SourceMsg) -> bool {
        if msg == SourceMsg::Open {
            let Some(p) = self.views.tags.as_mut() else { return false };
            p.menu = None;
            let targets: Vec<String> = p.targets().iter().map(|f| f.path.clone()).collect();
            let query = targets.first().map(|f| [self.workshop_value(f, "artist"), self.workshop_value(f, "album")].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ")).unwrap_or_default();
            let st = SourceState { step: 1, musicbrainz: storage::local().get(SOURCE_KEY).as_deref() == Some("musicbrainz"), query, targets, ..Default::default() };
            self.open_popin(Popin { title: t("workshop.complete_online"), icon: None, body: PopinBody::Source(Box::new(st)), size: PopinSize::Xxl, bare: true, flush: true });
            return self.source_search();
        }
        let Some(s) = self.source_state() else { return false };
        match msg {
            SourceMsg::Source(mb) => {
                if s.musicbrainz == mb {
                    return false;
                }
                s.musicbrainz = mb;
                if let Err(e) = storage::local().set(SOURCE_KEY, if mb { "musicbrainz" } else { "deezer" }) {
                log::warn!("Préférences locales : {e}");
            }
                return self.source_search();
            }
            SourceMsg::Query(v) => s.query = v,
            SourceMsg::Search => return self.source_search(),
            SourceMsg::Pick(id) => s.picked = Some(id),
            SourceMsg::Step1 => s.step = 1,
            SourceMsg::Next => return self.source_next(),
            SourceMsg::Mode(m) => {
                s.mode = m;
                if m < 2 {
                    let all = m == 1;
                    return self.source_select_all(all);
                }
            }
            SourceMsg::ToggleField(f) => {
                let keys = self.source_keys(Some(&f));
                return self.source_toggle_keys(keys);
            }
            SourceMsg::ToggleCover => {
                let keys = self.source_keys(None);
                return self.source_toggle_keys(keys);
            }
            SourceMsg::ToggleCell(k) => return self.source_toggle_keys(vec![k]),
            SourceMsg::Open1(f) => {
                if !s.opened.remove(&f) {
                    s.opened.insert(f);
                }
            }
            SourceMsg::MapOpen => s.map_open = !s.map_open,
            SourceMsg::Exclude(i) => {
                if !s.excluded.remove(&i) {
                    s.excluded.insert(i);
                }
            }
            SourceMsg::Apply => return self.source_apply(),
            SourceMsg::Auto => s.auto = true,
            SourceMsg::Open => {}
        }
        true
    }

    pub(crate) fn source_search(&mut self) -> bool {
        let Some(s) = self.source_state() else { return false };
        s.seq += 1;
        s.picked = None;
        s.hits.clear();
        if s.query.trim().is_empty() {
            s.searching = false;
            return true;
        }
        s.searching = true;
        s.error = None;
        let (seq, source, query, count) = (s.seq, if s.musicbrainz { Source::Musicbrainz } else { Source::Deezer }, s.query.clone(), s.targets.len() as u32);
        self.backend.call(async move { metadata_command::metadata_search_albums(source, query, Some(12)).await }, move |me, r| {
            let Some(s) = me.source_state() else { return false };
            if s.seq != seq {
                return false;
            }
            s.searching = false;
            s.searched = true;
            match r {
                Ok(found) => {
                    s.picked = found.iter().find(|h| h.track_count == count).or(found.first()).map(|h| h.id.clone());
                    let covers: Vec<(String, String)> = found.iter().filter_map(|h| h.cover.clone().map(|c| (h.id.clone(), c))).collect();
                    s.hits = found;
                    if s.auto && s.picked.is_some() {
                        me.source_next();
                    }
                    for (id, url) in covers {
                        me.backend.call(async move { crate::service::radio::radio_service::logo(&url).await }, move |me, path| {
                            let Some(s) = me.source_state() else { return false };
                            if let Some(p) = path {
                                s.thumbs.insert(id, p);
                            }
                            true
                        });
                    }
                }
                Err(e) => s.error = Some(e),
            }
            true
        });
        true
    }

    pub(crate) fn source_next(&mut self) -> bool {
        let Some(s) = self.source_state() else { return false };
        let Some(id) = s.picked.clone() else { return false };
        if s.matching {
            return false;
        }
        s.matching = true;
        s.error = None;
        let targets = s.targets.clone();
        let source = if s.musicbrainz { Source::Musicbrainz } else { Source::Deezer };
        let tracks: Vec<LocalTrackPayload> = targets
            .iter()
            .enumerate()
            .map(|(index, p)| LocalTrackPayload { index, title: self.workshop_value(p, "title"), track_number: self.workshop_value(p, "track_number").parse().ok(), duration: None })
            .collect();
        self.backend.call(async move { metadata_command::metadata_match_album(source, id, tracks).await }, |me, r| {
            let Some(s) = me.source_state() else { return false };
            s.matching = false;
            match r {
                Ok(p) => {
                    s.matches = p.matches.into_iter().map(|m| Match { local: m.local_index, remote: m.remote_key, score: m.score, confidence: m.confidence }).collect();
                    let cover = p.album.cover.clone();
                    s.album = Some(p.album);
                    s.excluded.clear();
                    s.opened.clear();
                    s.cover = None;
                    s.mode = 0;
                    s.step = 2;
                    me.source_select_all(false);
                    if let Some(url) = cover {
                        if let Some(s) = me.source_state() {
                            s.cover_loading = true;
                        }
                        me.backend.call(async move { tag_command::prepare_image_from_url(url).await }, |me, r| {
                            let Some(s) = me.source_state() else { return false };
                            s.cover_loading = false;
                            if let Ok(img) = r {
                                s.cover = Some(RemoteCover { path: img.path, src: img.src, size: (img.width, img.height) });
                                let mode = s.mode;
                                if mode < 2 {
                                    me.source_select_all(mode == 1);
                                }
                            }
                            true
                        });
                    }
                }
                Err(e) => s.error = Some(e),
            }
            true
        });
        true
    }

    /// Les valeurs qui comblent un vide (ou toutes), et la pochette.
    pub(crate) fn source_select_all(&mut self, all: bool) -> bool {
        let pairs = self.source_pairs();
        let has_cover = self.source_state().is_some_and(|s| s.cover.is_some());
        let mut next = HashSet::new();
        for p in &pairs {
            for c in p.cells.iter().filter(|c| c.state == 0) {
                if all || c.from.is_empty() {
                    next.insert(cell_key(&p.path, c.field));
                }
            }
            if has_cover && (all || !p.has_cover) {
                next.insert(cell_key(&p.path, "cover"));
            }
        }
        let Some(s) = self.source_state() else { return false };
        s.chosen = next;
        true
    }

    /// Les clés d'un champ, ou de la pochette (`None`).
    pub(crate) fn source_keys(&mut self, field: Option<&str>) -> Vec<String> {
        let pairs = self.source_pairs();
        match field {
            Some(f) => pairs.iter().filter(|p| p.cells.iter().any(|c| c.field == f && c.state == 0)).map(|p| cell_key(&p.path, f)).collect(),
            None => pairs.iter().map(|p| cell_key(&p.path, "cover")).collect(),
        }
    }

    pub(crate) fn source_toggle_keys(&mut self, keys: Vec<String>) -> bool {
        let Some(s) = self.source_state() else { return false };
        if keys.is_empty() {
            return false;
        }
        if keys.iter().all(|k| s.chosen.contains(k)) {
            for k in &keys {
                s.chosen.remove(k);
            }
        } else {
            s.chosen.extend(keys);
        }
        s.mode = 2;
        true
    }

    /// `apply` : verse les valeurs cochées (et la pochette) dans l'atelier, en une étape.
    pub(crate) fn source_apply(&mut self) -> bool {
        let pairs = self.source_pairs();
        let Some(s) = self.source_state() else { return false };
        let chosen = s.chosen.clone();
        let cover = s.cover.clone();
        let mut values = Vec::new();
        let mut covers = Vec::new();
        for p in &pairs {
            for c in p.cells.iter().filter(|c| c.state == 0 && chosen.contains(&cell_key(&p.path, c.field))) {
                values.push((p.path.clone(), field_index(c.field), c.to.clone()));
            }
            if cover.is_some() && chosen.contains(&cell_key(&p.path, "cover")) {
                covers.push(p.path.clone());
            }
        }
        let files: HashSet<&String> = values.iter().map(|v| &v.0).chain(covers.iter()).collect();
        let (nv, nf) = (values.len() + covers.len(), files.len());
        if nv == 0 {
            return false;
        }
        self.views.popin = None;
        let Some(p) = self.views.tags.as_mut() else { return false };
        p.snapshot();
        for (path, f, v) in values {
            if let Some(file) = p.files.iter_mut().find(|x| x.path == path) {
                file.apply(f, v);
            }
        }
        if let Some(c) = cover {
            for path in covers {
                if let Some(file) = p.files.iter_mut().find(|x| x.path == path) {
                    file.pending_cover = Some((c.path.clone(), c.src.clone()));
                }
            }
        }
        p.settle();
        self.tags_update(TagsMsg::Completed(nv, nf))
    }
}
