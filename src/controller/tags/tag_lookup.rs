use crate::models::tags::*;
use crate::mapper::tags::*;
use crate::app::music_app::MusicApp;
use karbon_ui::storage;
use crate::models::tags::Slot;
use crate::mapper::tags::COVER;
use crate::mapper::tags::EDITOR_FIELDS;
use crate::commands::tags::{metadata_command, tag_command};
use crate::core::metadata_source::model::TrackValues;

/// Les champs comparés entre le fichier et la source.
pub const COMPARED: [&str; 10] = ["title", "artist", "album", "album_artist", "year", "genre", "track_number", "total_tracks", "disc_number", "composer"];

pub(crate) fn proposed(v: &TrackValues, key: &str) -> String {
    let n = |x: Option<u32>| x.filter(|n| *n > 0).map(|n| n.to_string()).unwrap_or_default();
    match key {
        "title" => v.title.clone(),
        "artist" => v.artist.clone(),
        "album" => v.album.clone(),
        "album_artist" => v.album_artist.clone().unwrap_or_default(),
        "year" => v.year.clone().unwrap_or_default(),
        "genre" => v.genre.clone().unwrap_or_default(),
        "track_number" => n(v.track_number.map(u32::from)),
        "total_tracks" => n(v.total_tracks),
        "disc_number" => n(v.disc_number.map(u32::from)),
        "composer" => v.composer.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

impl MusicApp {
    pub(crate) fn lookup(&mut self) -> Option<&mut Lookup> {
        self.tag_editor()?.lookup.as_mut()
    }

    pub(crate) fn lookup_search(&mut self) -> bool {
        let Some(l) = self.lookup() else { return false };
        if l.query.trim().is_empty() {
            return false;
        }
        l.seq += 1;
        l.searching = true;
        l.error = None;
        let (seq, source, query) = (l.seq, l.source(), l.query.clone());
        self.backend.call(async move { metadata_command::metadata_search_tracks(source, query, Some(12)).await }, move |me, r| {
            let Some(l) = me.lookup() else { return false };
            if l.seq != seq {
                return false;
            }
            l.searching = false;
            match r {
                Ok(h) => {
                    l.hits = h;
                    l.searched = true;
                }
                Err(e) => l.error = Some(e),
            }
            true
        });
        true
    }

    pub(crate) fn lookup_update(&mut self, msg: LookupMsg) -> bool {
        let Some(l) = self.lookup() else { return false };
        match msg {
            LookupMsg::Query(v) => l.query = v,
            LookupMsg::Search => return self.lookup_search(),
            LookupMsg::Source(mb) => {
                if l.musicbrainz == mb {
                    return false;
                }
                l.musicbrainz = mb;
                l.hits.clear();
                l.searched = false;
                if let Err(e) = storage::local().set(SOURCE_KEY, if mb { "musicbrainz" } else { "deezer" }) {
                log::warn!("Préférences locales : {e}");
            }
                return self.lookup_search();
            }
            LookupMsg::Pick(i) => {
                let Some(hit) = l.hits.get(i).cloned() else { return false };
                l.loading = Some(i);
                l.error = None;
                let source = l.source();
                let id = hit.id.clone();
                self.backend.call(async move { metadata_command::metadata_track_values(source, id).await }, move |me, r| {
                    let Some(e) = me.tag_editor() else { return false };
                    let current: Vec<(String, String)> = EDITOR_FIELDS.iter().zip(&e.form).map(|(k, v)| ((*k).to_string(), v.clone())).collect();
                    let has_cover = e.cover().is_some();
                    let Some(l) = e.lookup.as_mut() else { return false };
                    l.loading = None;
                    let mut cover_url = None;
                    match r {
                        Ok(v) => {
                            let rows: Vec<CompareRow> = COMPARED
                                .iter()
                                .map(|k| {
                                    let from = current.iter().find(|c| c.0 == *k).map(|c| c.1.clone()).unwrap_or_default();
                                    let to = proposed(&v, k);
                                    let state = if to.is_empty() { 2 } else if to == from { 1 } else { 0 };
                                    CompareRow { key: k, from, to, state }
                                })
                                .collect();
                            let checked = rows.iter().filter(|r| r.state == 0).map(|r| r.key).collect();
                            l.picked = Some((hit, rows, checked));
                            l.cover = None;
                            l.cover_error = None;
                            l.take_cover = false;
                            l.cover_loading = v.cover.is_some();
                            cover_url = v.cover.clone();
                        }
                        Err(err) => l.error = Some(err),
                    }
                    if let Some(url) = cover_url {
                        me.backend.call(async move { tag_command::prepare_image_from_url(url).await }, move |me, r| {
                            let Some(l) = me.lookup() else { return false };
                            l.cover_loading = false;
                            match r {
                                Ok(img) => {
                                    l.cover = Some(Remote {
                                        path: img.path,
                                        src: img.src,
                                        mime: img.mime_type,
                                        size: format!("{}×{}", img.width, img.height),
                                        bytes: img.bytes,
                                        recompressed: img.recompressed,
                                    });
                                    l.take_cover = !has_cover;
                                }
                                Err(e) => l.cover_error = Some(e),
                            }
                            true
                        });
                    }
                    true
                });
            }
            LookupMsg::Back => l.picked = None,
            LookupMsg::Close => {
                if let Some(e) = self.tag_editor() {
                    e.lookup = None;
                }
            }
            LookupMsg::Toggle(k) => {
                if let Some(p) = l.picked.as_mut() {
                    if let Some(i) = p.2.iter().position(|x| *x == k) {
                        p.2.remove(i);
                    } else {
                        p.2.push(k);
                    }
                }
            }
            LookupMsg::ToggleAll => {
                let all = l.all_checked();
                let s = l.selectable();
                if let Some(p) = l.picked.as_mut() {
                    p.2 = if all { Vec::new() } else { s };
                }
            }
            LookupMsg::TakeCover => {
                if l.cover.is_some() {
                    l.take_cover = !l.take_cover;
                }
            }
            LookupMsg::Apply => return self.lookup_apply(),
        }
        true
    }

    /// Verse les champs cochés, puis la pochette à la place de l'ancienne.
    pub(crate) fn lookup_apply(&mut self) -> bool {
        let Some(e) = self.tag_editor() else { return false };
        let Some(l) = e.lookup.take() else { return false };
        let Some((_, rows, checked)) = &l.picked else { return false };
        let mut touched = Vec::new();
        for r in rows.iter().filter(|r| r.state == 0 && checked.contains(&r.key)) {
            if let Some(i) = EDITOR_FIELDS.iter().position(|f| *f == r.key) {
                if e.form[i] != r.to {
                    e.form[i] = r.to.clone();
                    touched.push(r.key.to_string());
                }
            }
        }
        if let (true, Some(c)) = (l.take_cover, l.cover.clone()) {
            let slot = Slot { id: None, path: Some(c.path), src: c.src, picture_type: COVER, description: String::new(), mime: c.mime, bytes: c.bytes };
            match e.slots.iter().position(|s| s.picture_type == COVER) {
                Some(i) => e.slots[i] = slot,
                None => e.slots.insert(0, slot),
            }
            touched.push("cover".into());
        }
        e.filled = touched;
        true
    }
}
