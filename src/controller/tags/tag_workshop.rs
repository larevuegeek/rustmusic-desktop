//! L'atelier des tags : chargement des données et réponse aux messages.

use crate::models::tags::*;
use crate::mapper::tags::*;
use std::collections::HashSet;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::commands::library::library_command;
use crate::commands::tags::tag_command;

/// « 3 morceaux ».
pub fn tracks_label(n: usize) -> String {
    format!("{n} {}", t(if n > 1 { "workshop.track_many" } else { "workshop.track_one" }))
}

/// Majuscule en tête de chaque mot.
pub(crate) fn title_case(v: &str) -> String {
    v.split(' ')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|p| p.to_uppercase().collect::<String>() + &c.as_str().to_lowercase()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl WorkshopFile {
    /// La modification disparaît si la valeur revient à l'original.
    pub(crate) fn apply(&mut self, f: usize, value: String) {
        self.edits[f] = (value != self.original[f]).then_some(value);
    }
}

impl WorkshopPage {
    /// Une étape d'annulation : l'état d'avant.
    pub(crate) fn snapshot(&mut self) {
        self.undo.push(self.files.clone());
    }

    /// Annule l'étape si rien n'a changé ; dit si quelque chose a changé.
    pub(crate) fn settle(&mut self) -> bool {
        let changed = self.undo.last().is_some_and(|before| before.iter().zip(&self.files).any(|(a, b)| a.edits != b.edits || a.pending_cover != b.pending_cover));
        if !changed {
            self.undo.pop();
        }
        changed
    }
}

impl MusicApp {
    /// Charge les titres d'un album, puis leurs tags lus dans les fichiers.
    pub(crate) fn tags_open(&mut self, album: Option<String>) {
        // Sélection envoyée à l'atelier (`tagWorkshop.load(paths)`) : la barre de sélection.
        let paths = self.views.workshop_paths.take();
        if paths.is_none() && self.views.tags.as_ref().is_some_and(|p| p.album == album && !p.files.is_empty()) {
            return;
        }
        self.views.tags = Some(WorkshopPage {
            album: album.clone(),
            loading: album.is_some() || paths.is_some(),
            shown: vec![0, 1, 2, 3, 5, 6],
            panel: true,
            ..Default::default()
        });
        if album.is_none() && paths.is_none() {
            return;
        }
        let app = self.backend.app.clone();
        let library = self.data.library_id.unwrap_or(0);
        let from_selection = self.views.workshop_source.take().unwrap_or_else(|| t("workshop.from_selection"));
        self.backend.call(
            async move {
                let (source, paths) = match (album, paths) {
                    (_, Some(paths)) => (from_selection, paths),
                    (Some(album), None) => {
                        let tracks_list = library_command::get_tracks_by_album(app.state(), library, album).await.unwrap_or_default();
                        let source = tracks_list.first().and_then(|p| p.album.clone()).unwrap_or_else(|| t("workshop.title"));
                        (source, tracks_list.into_iter().map(|p| p.path).collect())
                    }
                    (None, None) => (String::new(), Vec::new()),
                };
                (source, tag_command::read_workshop_tracks(paths).await.unwrap_or_default())
            },
            |me, (source, read_count)| {
                let Some(p) = me.views.tags.as_mut() else { return false };
                p.source = source;
                p.loading = false;
                p.files = read_count
                    .into_iter()
                    .map(|w| {
                        let tags = serde_json::to_value(&w.tags).unwrap_or_default();
                        let original = FIELDS.map(|f| match &tags[f] {
                            serde_json::Value::Null => String::new(),
                            serde_json::Value::String(s) => s.clone(),
                            v => v.to_string(),
                        });
                        WorkshopFile { path: w.path, original, edits: Default::default(), readable: w.readable && w.writable, cover: w.cover, pending_cover: None }
                    })
                    .collect();
                p.selected = p.files.iter().filter(|f| f.readable).map(|f| f.path.clone()).collect();
                true
            },
        );
    }

    /// `say` : l'avis s'efface après 2,6 s (4,5 s s'il propose « Annuler »).
    pub(crate) fn tags_say(&mut self, text: String, undoable: bool) {
        let Some(p) = self.views.tags.as_mut() else { return };
        p.next_notice += 1;
        let id = p.next_notice;
        p.notice = Some(Notice { text, undoable, id });
        let ms = if undoable { 4500 } else { 2600 };
        self.backend.call(async move { tokio::time::sleep(std::time::Duration::from_millis(ms)).await }, move |me, ()| {
            me.tags_update(TagsMsg::NoticeEnd(id))
        });
    }

    pub(crate) fn tags_update(&mut self, msg: TagsMsg) -> bool {
        let Some(p) = self.views.tags.as_mut() else { return false };
        match msg {
            TagsMsg::Pick(path) => {
                if p.files.iter().any(|f| f.path == path && f.readable) {
                    p.selected = [path].into_iter().collect();
                }
            }
            TagsMsg::Toggle(path) => {
                if !p.selected.remove(&path) {
                    p.selected.insert(path);
                }
            }
            TagsMsg::ToggleAll => {
                let readable: Vec<String> = p.readable().map(|f| f.path.clone()).collect();
                if readable.iter().all(|x| p.selected.contains(x)) {
                    p.selected.clear();
                } else {
                    p.selected.extend(readable);
                }
            }
            TagsMsg::Edit(path, f) => {
                let Some(file) = p.files.iter().find(|x| x.path == path && x.readable) else { return false };
                p.draft = file.value(f).to_string();
                p.editing = Some((path, f));
            }
            TagsMsg::Draft(v) => p.draft = v,
            TagsMsg::Commit => {
                let Some((path, f)) = p.editing.take() else { return false };
                let value = p.draft.split_whitespace().collect::<Vec<_>>().join(" ");
                p.snapshot();
                if let Some(file) = p.files.iter_mut().find(|x| x.path == path) {
                    file.apply(f, value);
                }
                p.settle();
            }
            TagsMsg::Sort(f) => {
                let asc = p.sort.is_none_or(|(g, a)| g != f || !a);
                p.sort = Some((f, asc));
                let num = [0usize, 5, 7, 8, 9].contains(&f);
                p.files.sort_by(|a, b| {
                    let o = if num {
                        a.value(f).parse::<f64>().unwrap_or(f64::MAX).total_cmp(&b.value(f).parse::<f64>().unwrap_or(f64::MAX))
                    } else {
                        a.value(f).to_lowercase().cmp(&b.value(f).to_lowercase())
                    };
                    if asc { o } else { o.reverse() }
                });
            }
            TagsMsg::Menu(m) => p.menu = if p.menu == m { None } else { m },
            TagsMsg::Column(c) => {
                if c != TITLE && !p.shown.contains(&c) {
                    p.shown.push(c);
                } else {
                    p.shown.retain(|x| *x != c);
                }
            }
            TagsMsg::ColumnAction(f, action) => {
                p.menu = None;
                if action == 3 {
                    p.shown.retain(|x| *x != f);
                    return true;
                }
                let n = p.targets().len();
                let first = p.targets().first().map(|x| x.value(f).to_string()).unwrap_or_default();
                let targets: HashSet<String> = p.targets().iter().map(|x| x.path.clone()).collect();
                p.snapshot();
                for file in p.files.iter_mut().filter(|x| targets.contains(&x.path)) {
                    let v = match action {
                        0 => first.clone(),
                        1 => title_case(file.value(f)),
                        _ => String::new(),
                    };
                    file.apply(f, v);
                }
                let text = if p.settle() {
                    let what = ["done_fill", "done_case", "done_clear"][action as usize];
                    (format!("{} · {} · {}", t(&format!("tags.{}", FIELDS[f])), t(&format!("workshop.{what}")), tracks_label(n)), true)
                } else {
                    (t("workshop.nothing_changed"), false)
                };
                self.tags_say(text.0, text.1);
            }
            TagsMsg::OnlyIssues => p.only_issues = !p.only_issues,
            TagsMsg::Panel => p.panel = !p.panel,
            TagsMsg::Undo => {
                let Some(before) = p.undo.pop() else { return false };
                p.files = before;
                p.editing = None;
                self.tags_say(t("workshop.undone"), false);
            }
            TagsMsg::RevertAll => {
                p.snapshot();
                for f in &mut p.files {
                    f.edits = Default::default();
                }
                p.settle();
                self.tags_say(t("workshop.all_reverted"), true);
            }
            TagsMsg::InspectorDraft(f, v) => {
                p.drafts.insert(f, v);
            }
            TagsMsg::InspectorApply(f) => {
                let Some(raw) = p.drafts.remove(&f) else { return false };
                let value = raw.split_whitespace().collect::<Vec<_>>().join(" ");
                return self.tags_set_on_selected(f, value);
            }
            TagsMsg::Genre(g) => return self.tags_set_on_selected(GENRE, g),
            TagsMsg::Revert(f) => {
                let targets: HashSet<String> = p.targets().iter().map(|x| x.path.clone()).collect();
                p.snapshot();
                for file in p.files.iter_mut().filter(|x| targets.contains(&x.path)) {
                    file.edits[f] = None;
                }
                p.settle();
            }
            TagsMsg::Number => {
                p.menu = None;
                let targets: Vec<String> = p.targets().iter().map(|x| x.path.clone()).collect();
                p.snapshot();
                for (i, path) in targets.iter().enumerate() {
                    if let Some(file) = p.files.iter_mut().find(|x| x.path == *path) {
                        file.apply(TRACK_NUMBER, (i + 1).to_string());
                    }
                }
                let text = if p.settle() { (t("workshop.numbered").replace("{n}", &targets.len().to_string()), true) } else { (t("workshop.already_numbered"), false) };
                self.tags_say(text.0, text.1);
            }
            TagsMsg::FillAlbumArtist => return self.tags_fill_album_artist(),
            TagsMsg::Write => return self.tags_write(),
            TagsMsg::Leave => {
                let pending = p.readable().any(WorkshopFile::dirty);
                if pending && !p.confirm_leave {
                    p.confirm_leave = true;
                    return true;
                }
                let album = p.album.clone();
                self.views.tags = None;
                return self.handle(match album {
                    Some(a) => Msg::Go(crate::models::app::Route::Album(a)),
                    None => Msg::Back,
                });
            }
            TagsMsg::KeepEditing => p.confirm_leave = false,
            TagsMsg::Cleaned(n) => {
                if n > 0 {
                    self.tags_say(t("workshop.cleaned").replace("{n}", &n.to_string()), true);
                }
                return true;
            }
            TagsMsg::Completed(v, n) => {
                if v > 0 {
                    let files = crate::mapper::tags::plural(n, "workshop.file_one", "workshop.file_many");
                    self.tags_say(t("workshop.completed").replace("{v}", &v.to_string()).replace("{n}", &files), true);
                }
                return true;
            }
            TagsMsg::NoticeEnd(id) => {
                if p.notice.as_ref().is_some_and(|n| n.id == id) {
                    p.notice = None;
                } else {
                    return false;
                }
            }
        }
        true
    }

    /// La même valeur sur toute la sélection.
    pub(crate) fn tags_set_on_selected(&mut self, f: usize, value: String) -> bool {
        let Some(p) = self.views.tags.as_mut() else { return false };
        let targets: HashSet<String> = p.targets().iter().map(|x| x.path.clone()).collect();
        let n = targets.len();
        p.snapshot();
        for file in p.files.iter_mut().filter(|x| targets.contains(&x.path)) {
            file.apply(f, value.clone());
        }
        if p.settle() && n > 1 {
            self.tags_say(format!("{} · {}", t(&format!("tags.{}", FIELDS[f])), tracks_label(n)), true);
        }
        true
    }

    /// Artiste de l'album vide : l'artiste le plus fréquent, sans ses invités.
    pub(crate) fn tags_fill_album_artist(&mut self) -> bool {
        let Some(p) = self.views.tags.as_mut() else { return false };
        let mut counts: Vec<(String, usize)> = Vec::new();
        for f in p.readable() {
            let v = f.value(ARTIST);
            let main = [" feat. ", " feat ", " ft. ", " ft ", " featuring "]
                .iter()
                .filter_map(|s| v.to_lowercase().find(s))
                .min()
                .map_or(v, |i| &v[..i])
                .trim()
                .to_string();
            if main.is_empty() {
                continue;
            }
            match counts.iter_mut().find(|(a, _)| *a == main) {
                Some((_, n)) => *n += 1,
                None => counts.push((main, 1)),
            }
        }
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        let Some((best, _)) = counts.into_iter().next() else { return false };
        p.snapshot();
        let mut n = 0;
        for f in p.files.iter_mut().filter(|f| f.readable && f.value(ALBUM_ARTIST).trim().is_empty()) {
            f.apply(ALBUM_ARTIST, best.clone());
            n += 1;
        }
        p.settle();
        self.tags_say(t("workshop.album_artist_filled").replace("{v}", &best).replace("{n}", &tracks_label(n)), true);
        true
    }

    /// `write` : chaque fichier porte ses modifications ; le lot part en tâche de fond.
    pub(crate) fn tags_write(&mut self) -> bool {
        let Some(p) = self.views.tags.as_mut() else { return false };
        if p.launching {
            return false;
        }
        let items: Vec<serde_json::Value> = p
            .readable()
            .filter(|f| f.dirty())
            .map(|f| {
                let mut edit: serde_json::Map<String, serde_json::Value> =
                    FIELDS.iter().enumerate().filter_map(|(i, k)| f.edits[i].clone().map(|v| (k.to_string(), serde_json::Value::String(v)))).collect();
                if let Some((path, _)) = &f.pending_cover {
                    edit.insert("cover".into(), serde_json::json!({ "path": path, "picture_type": 3 }));
                }
                serde_json::json!({ "path": f.path, "edit": edit })
            })
            .collect();
        let Ok(items) = serde_json::from_value::<Vec<tag_command::TagEditItem>>(serde_json::Value::Array(items)) else { return false };
        if items.is_empty() {
            return false;
        }
        let written_items: HashSet<String> = items.iter().map(|i| i.path.clone()).collect();
        p.launching = true;
        p.error = None;
        let app = self.backend.app.clone();
        self.backend.call(async move { tag_command::write_tags_each(app.clone(), app.state(), items).await }, move |me, r| {
            let Some(p) = me.views.tags.as_mut() else { return false };
            p.launching = false;
            match r {
                Ok(job) => {
                    me.batch_begin(job, t("batch.writing_tags"));
                    let Some(p) = me.views.tags.as_mut() else { return true };
                    // Ce qui est écrit ne se défait plus ici.
                    p.undo.clear();
                    for f in p.files.iter_mut().filter(|f| written_items.contains(&f.path)) {
                        if let Some((_, src)) = f.pending_cover.take() {
                            f.cover = Some(src);
                        }
                        for (i, e) in f.edits.iter_mut().enumerate() {
                            if let Some(v) = e.take() {
                                f.original[i] = v;
                            }
                        }
                    }
                }
                Err(e) => p.error = Some(e),
            }
            true
        });
        true
    }
}
