//! L'atelier des tags : ce qu'affichent ses gabarits, bâti depuis l'état de la page.

use crate::models::tags::*;
use crate::mapper::tags::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::controller::tags::tracks_label;
use crate::models::tags::TagsMsg;
use crate::models::tags::WorkshopFile;

pub const FIELDS: [&str; 12] = [
    "track_number", "title", "artist", "album", "album_artist", "year", "genre", "disc_number", "total_tracks", "total_discs", "composer", "comment",
];

/// Indice de chaque champ dans `FIELDS`.
pub const TRACK_NUMBER: usize = 0;

pub const TITLE: usize = 1;

pub const ARTIST: usize = 2;

pub const ALBUM_ARTIST: usize = 4;

pub const GENRE: usize = 6;

/// Titre, artiste, album.
pub const ESSENTIAL: [usize; 3] = [1, 2, 3];

pub const FLAGGED: [usize; 5] = [1, 2, 3, 4, 6];

/// La colonne « Fichier », hors des champs.
pub const FILE_COLUMN: usize = 12;

/// Les actions des menus « Tags » et « Fichiers » , par leur titre.
pub fn workshop_action(key: &str) -> crate::models::app::Msg {
    use crate::models::app::Msg;
    match key {
        "tags.number_tracks" => Msg::Tags(TagsMsg::Number),
        "clean.short" => Msg::Clean(crate::models::tags::CleanMsg::Open),
        "history.title" => Msg::History(crate::models::tags::HistoryMsg::Open),
        "workshop.rename_files" => Msg::Rename(crate::models::tags::RenameMsg::Open),
        "workshop.complete_online" => Msg::Source(crate::models::tags::SourceMsg::Open),
        _ => Msg::Tags(TagsMsg::Menu(None)),
    }
}

/// Largeur minimale, part de la place restante (0 : fixe), nombre.
pub const SPECS: [(f32, f32, bool); 13] = [
    (72., 0., true),
    (200., 1.7, false),
    (140., 1., false),
    (140., 1., false),
    (140., 1., false),
    (84., 0., true),
    (110., 0.8, false),
    (80., 0., true),
    (96., 0., true),
    (96., 0., true),
    (170., 1., false),
    (200., 1., false),
    (240., 1.2, false),
];

pub(crate) fn label(c: usize) -> String {
    if c == FILE_COLUMN {
        t("workshop.file")
    } else {
        t(&format!("tags.{}", FIELDS[c]))
    }
}

pub(crate) fn heading(c: usize) -> String {
    match c {
        0 => t("workshop.col_track"),
        7 => t("tags.disc_short"),
        8 => t("workshop.col_tracks"),
        9 => t("workshop.col_discs"),
        _ => label(c),
    }
}

pub(crate) fn file_name(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
}

pub(crate) const FEAT: [&str; 4] = [" feat", " ft.", " ft ", " featuring "];

pub(crate) fn has_feat(f: &WorkshopFile) -> bool {
    let v = f.value(ARTIST).to_lowercase();
    FEAT.iter().any(|m| v.contains(m))
}

pub(crate) fn has_issue(f: &WorkshopFile) -> bool {
    f.incomplete() || f.value(GENRE).trim().is_empty() || f.value(crate::mapper::tags::ALBUM_ARTIST).trim().is_empty() || has_feat(f)
}

impl WorkshopFile {
    pub fn value(&self, f: usize) -> &str {
        self.edits[f].as_deref().unwrap_or(&self.original[f])
    }

    pub fn dirty(&self) -> bool {
        self.edits.iter().any(Option::is_some) || self.pending_cover.is_some()
    }

    /// La pochette affichée : celle en attente, sinon celle du fichier.
    pub fn shown_cover(&self) -> Option<String> {
        self.pending_cover.as_ref().map(|c| c.1.clone()).or_else(|| self.cover.clone())
    }

    pub fn incomplete(&self) -> bool {
        ESSENTIAL.iter().any(|f| self.value(*f).trim().is_empty())
    }
}

impl WorkshopPage {
    pub fn readable(&self) -> impl Iterator<Item = &WorkshopFile> {
        self.files.iter().filter(|f| f.readable)
    }

    pub fn targets(&self) -> Vec<&WorkshopFile> {
        self.readable().filter(|f| self.selected.contains(&f.path)).collect()
    }
}

/// Le libellé d'un point à vérifier (`essential`, `genre`…).
pub fn issue_label(key: &str) -> String {
    t(&format!("workshop.issue_{key}"))
}

impl crate::models::tags::Column {
    /// Le menu de la colonne.
    pub fn menu_key(&self) -> String {
        format!("col:{}", self.idx)
    }
}

impl MusicApp {
    pub fn workshop_view(&self) -> Option<WorkshopView> {
        let p = self.views.tags.as_ref()?;
        let readable: Vec<&WorkshopFile> = p.readable().collect();
        let pending: Vec<&&WorkshopFile> = readable.iter().filter(|f| f.dirty()).collect();
        let changes: usize = pending.iter().map(|f| f.edits.iter().filter(|e| e.is_some()).count()).sum();
        let targets = p.targets();
        let common = |f: usize| {
            let values: std::collections::BTreeSet<&str> = readable.iter().map(|x| x.value(f)).filter(|v| !v.is_empty()).collect();
            (values.len() == 1).then(|| values.into_iter().next().unwrap_or_default().to_string())
        };
        let mut formats: Vec<String> = Vec::new();
        for f in &p.files {
            let e = f.path.rsplit('.').next().unwrap_or("").to_uppercase();
            if !e.is_empty() && !formats.contains(&e) {
                formats.push(e);
            }
        }
        let empty = |x: &WorkshopFile, f: usize| x.value(f).trim().is_empty();
        let issues: Vec<(&'static str, usize, &'static str)> = [
            ("essential", readable.iter().filter(|f| f.incomplete()).count(), "online"),
            ("genre", readable.iter().filter(|f| empty(f, GENRE)).count(), "online"),
            ("album_artist", readable.iter().filter(|f| empty(f, crate::mapper::tags::ALBUM_ARTIST)).count(), "fill_album_artist"),
            ("featuring", readable.iter().filter(|f| has_feat(f)).count(), "clean"),
        ]
        .into_iter()
        .filter(|i| i.1 > 0)
        .collect();

        let scope = karbon_ui::intl::t_with("workshop.scope", &[("total", &readable.len().to_string())]);
        let (scope_before, scope_after) = match scope.split_once("{n}") {
            Some((a, b)) => (a.trim_end().to_string(), b.trim_start().to_string()),
            None => (scope.trim_end().to_string(), String::new()),
        };
        let column_ids: Vec<usize> = (0..13).filter(|c| *c == crate::mapper::tags::TITLE || p.shown.contains(c)).collect();
        let first = |f: usize| targets.first().map(|x| x.value(f).to_string()).unwrap_or_default();
        let short = |v: String| if v.chars().count() > 18 { format!("{}…", v.chars().take(18).collect::<String>()) } else { v };
        let columns: Vec<Column> = column_ids
            .iter()
            .map(|&c| {
                let (width, grow, numeric) = SPECS[c];
                let v = if c < 12 { first(c) } else { String::new() };
                Column {
                    idx: c,
                    label: label(c),
                    heading: heading(c),
                    width,
                    grow,
                    numeric,
                    sorted: p.sort.filter(|(f, _)| *f == c).map(|(_, a)| a),
                    menu: p.menu.as_deref() == Some(&format!("col:{c}")),
                    fill: (t("workshop.copy_value").replace("{v}", &short(if v.is_empty() { "—".into() } else { v.clone() })), targets.len() >= 2 && !v.is_empty()),
                    case_ok: !numeric && !targets.is_empty(),
                    clear: (t("workshop.clear_n").replace("{n}", &targets.len().to_string()), !targets.is_empty()),
                }
            })
            .collect();
        let lines: Vec<&WorkshopFile> = if p.only_issues { p.files.iter().filter(|f| !f.readable || has_issue(f)).collect() } else { p.files.iter().collect() };
        let rows = lines
            .iter()
            .map(|f| Row {
                path: f.path.clone(),
                file_name: file_name(&f.path),
                on: p.selected.contains(&f.path),
                readable: f.readable,
                cover: f.shown_cover(),
                modified: f.readable && f.dirty(),
                cells: column_ids
                    .iter()
                    .map(|&c| {
                        let (width, grow, numeric) = SPECS[c];
                        let value = if c == FILE_COLUMN { file_name(&f.path) } else { f.value(c).to_string() };
                        let dirty = c < 12 && f.edits[c].is_some();
                        Cell {
                            idx: c,
                            missing: value.is_empty() && FLAGGED.contains(&c) && f.readable,
                            title: if dirty {
                                format!("{} : {}", t("workshop.before"), if f.original[c].is_empty() { t("workshop.was_empty") } else { f.original[c].clone() })
                            } else if c == FILE_COLUMN {
                                f.path.clone()
                            } else {
                                value.clone()
                            },
                            editing: p.editing.as_ref().is_some_and(|(path, x)| *path == f.path && *x == c),
                            value,
                            dirty,
                            width,
                            grow,
                            numeric,
                        }
                    })
                    .collect(),
            })
            .collect();
        let readable_shown: Vec<&&WorkshopFile> = lines.iter().filter(|f| f.readable).collect();
        let all_selected = !readable_shown.is_empty() && readable_shown.iter().all(|f| p.selected.contains(&f.path));
        let some_selected = readable_shown.iter().any(|f| p.selected.contains(&f.path));

        Some(WorkshopView {
            head_cover: p.files.iter().find_map(|f| f.shown_cover()),
            source: p.source.clone(),
            artist: common(crate::mapper::tags::ALBUM_ARTIST).or_else(|| common(ARTIST)),
            year: common(5),
            files_label: format!("{} {}", plural(p.files.len(), "workshop.file_one", "workshop.file_many"), formats.join(", ")),
            changes,
            changes_label: plural(changes, "workshop.change_one", "workshop.change_many"),
            undo: p.undo.len(),
            pending: pending.len(),
            write_label: if pending.is_empty() {
                t("workshop.write")
            } else {
                format!("{} {}", t("workshop.write"), plural(pending.len(), "workshop.file_one", "workshop.file_many"))
            },
            launching: p.launching,
            panel: p.panel,
            leave: p.confirm_leave.then(|| format!("{} {}", pending.len(), t(if pending.len() > 1 { "workshop.pending_files" } else { "workshop.pending_file" }))),
            error: p.error.clone(),
            selected: targets.len(),
            total: readable.len(),
            issue_count: readable.iter().filter(|f| has_issue(f)).count(),
            issues_label: format!("{} {}", issues.len(), t(if issues.len() > 1 { "workshop.issue_many" } else { "workshop.issue_one" })),
            scope_before,
            scope_after,
            issues,
            only_issues: p.only_issues,
            menu: p.menu.clone(),
            min_width: 48. + 56. + 32. + column_ids.iter().map(|c| SPECS[*c].0).sum::<f32>(),
            all_columns: (0..13).map(|c| (c, label(c), c == crate::mapper::tags::TITLE || p.shown.contains(&c))).collect(),
            columns,
            rows,
            all_selected,
            some_selected,
            editing: p.editing.is_some(),
            draft: p.draft.clone(),
            inspector: self.workshop_inspector(),
            notice: p.notice.as_ref().map(|n| (n.text.clone(), n.undoable)),
        })
    }

    pub(crate) fn workshop_inspector(&self) -> Option<Inspector> {
        let p = self.views.tags.as_ref()?;
        let targets = p.targets();
        if targets.is_empty() {
            return None;
        }
        let un = targets.len() == 1;
        let common = |f: usize| {
            let v = targets[0].value(f);
            targets.iter().all(|x| x.value(f) == v).then(|| v.to_string())
        };
        let field = |idx: usize, half: bool, per_file: bool| {
            let c = common(idx);
            let locked = per_file && !un;
            InspectorField {
                idx,
                label: t(&format!("tags.{}", FIELDS[idx])),
                value: p.drafts.get(&idx).cloned().or_else(|| c.clone()).unwrap_or_default(),
                placeholder: if c.is_none() { t("workshop.mixed_keep") } else { "—".into() },
                half: half && !locked,
                locked,
                missing: c.as_deref() == Some("") && FLAGGED.contains(&idx),
                dirty: targets.iter().any(|x| x.edits[idx].is_some()),
            }
        };
        let mut genres: Vec<(String, usize)> = Vec::new();
        for f in &p.files {
            let g = f.value(GENRE).trim().to_string();
            if g.is_empty() {
                continue;
            }
            match genres.iter_mut().find(|(x, _)| *x == g) {
                Some((_, n)) => *n += 1,
                None => genres.push((g, 1)),
            }
        }
        genres.sort_by(|a, b| b.1.cmp(&a.1));
        let current = common(GENRE);
        let plural = tracks_label(targets.len());
        let applies = t("workshop.applies_many");
        let (before, after) = applies.split_once("{n}").map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or((applies, String::new()));
        Some(Inspector {
            title: if un {
                Some(targets[0].value(crate::mapper::tags::TITLE)).filter(|v| !v.is_empty()).map(String::from).unwrap_or_else(|| file_name(&targets[0].path))
            } else {
                plural.clone()
            },
            sub: if un { file_name(&targets[0].path) } else { p.source.clone() },
            cover_state: if targets.iter().all(|f| f.cover.is_some()) {
                t("workshop.cover_embedded")
            } else if targets.iter().any(|f| f.cover.is_some()) {
                t("workshop.cover_partial")
            } else {
                t("tags.no_cover")
            },
            cover: targets.iter().find_map(|f| f.shown_cover()),
            groups: vec![
                (t("tags.section_track"), vec![field(1, false, true), field(2, false, false), field(0, true, true), field(7, true, false)]),
                (
                    t("tags.section_album"),
                    vec![field(3, false, false), field(4, false, false), field(5, true, false), field(6, true, false), field(8, true, false), field(9, true, false)],
                ),
                (t("workshop.credits"), vec![field(10, false, false), field(11, false, false)]),
            ],
            genres: genres.into_iter().map(|(g, _)| g).filter(|g| Some(g) != current.as_ref()).take(5).collect(),
            applies: if un { (t("workshop.applies_one"), String::new(), String::new()) } else { (before, plural, after) },
        })
    }

    /// Pour les gabarits : le message d'une case.
    pub fn workshop_edit_msg(&self, path: &str, idx: usize) -> Msg {
        Msg::Tags(TagsMsg::Edit(path.to_string(), idx))
    }

    /// Les vignettes affichées par l'atelier.
    pub(crate) fn workshop_covers(&self) -> Vec<String> {
        let Some(p) = self.views.tags.as_ref() else { return Vec::new() };
        let mut v: Vec<String> = Vec::new();
        for c in p.files.iter().filter_map(|f| f.shown_cover()) {
            if !v.contains(&c) {
                v.push(c);
            }
        }
        v
    }
}
