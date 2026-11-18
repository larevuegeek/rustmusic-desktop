use crate::models::tags::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::controller::tags::cell_key;
use crate::models::tags::SourceState;
use crate::mapper::tags::SOURCE_FIELDS;
use crate::core::metadata_source::matching::Confidence;
use crate::core::metadata_source::model::{AlbumDetail, AlbumTrack};

pub(crate) fn proposed(album: &AlbumDetail, track: &AlbumTrack, field: &str) -> String {
    let discs = album.tracks.iter().map(|t| t.disc).collect::<std::collections::HashSet<_>>().len();
    match field {
        "title" => track.title.clone(),
        "artist" => track.artist.clone(),
        "album" => album.title.clone(),
        "album_artist" => album.artist.clone(),
        "year" => album.year.clone().unwrap_or_default(),
        "genre" => album.genre.clone().unwrap_or_default(),
        "track_number" => track.position.to_string(),
        "total_tracks" => album.tracks.iter().filter(|t| t.disc == track.disc).count().to_string(),
        "disc_number" => {
            if discs > 1 {
                track.disc.to_string()
            } else {
                String::new()
            }
        }
        "composer" => track.composer.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

pub fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n > 1 { t(many) } else { t(one) })
}

pub fn duration(s: u32) -> String {
    format!("{}:{:02}", s / 60, s % 60)
}

impl MusicApp {
    /// `pairs` : les fichiers appariés (ni rejetés ni exclus) et leurs cellules.
    pub(crate) fn source_pairs(&self) -> Vec<Pair> {
        let Some(s) = self.views.popin.as_ref().and_then(|p| p.source()) else { return Vec::new() };
        let Some(album) = &s.album else { return Vec::new() };
        let Some(page) = self.views.tags.as_ref() else { return Vec::new() };
        s.matches
            .iter()
            .filter(|m| m.confidence != Confidence::Rejected && !s.excluded.contains(&m.local))
            .filter_map(|m| {
                let path = s.targets.get(m.local)?;
                let file = page.files.iter().find(|f| &f.path == path)?;
                let track = album.tracks.iter().find(|t| Some(t.key) == m.remote)?.clone();
                let cells = SOURCE_FIELDS
                    .iter()
                    .map(|f| {
                        let to = proposed(album, &track, f);
                        let from = file.value(crate::mapper::tags::field_index(f)).trim().to_string();
                        let state = if to.is_empty() { 2 } else if to == from { 1 } else { 0 };
                        SourceCell { field: f, from, to, state }
                    })
                    .collect();
                Some(Pair { index: m.local, path: path.clone(), has_cover: file.shown_cover().is_some(), track, doubtful: m.confidence == Confidence::Doubtful, cells })
            })
            .collect()
    }

    /// Les lignes de champs (seulement ceux où quelque chose diffère).
    pub fn source_rows(&self) -> Vec<FieldRow> {
        let Some(s) = self.views.popin.as_ref().and_then(|p| p.source()) else { return Vec::new() };
        let pairs = self.source_pairs();
        SOURCE_FIELDS
            .iter()
            .filter_map(|f| {
                let diffs: Vec<(&Pair, &SourceCell)> = pairs.iter().filter_map(|p| p.cells.iter().find(|c| c.field == *f && c.state == 0).map(|c| (p, c))).collect();
                if diffs.is_empty() {
                    return None;
                }
                let keys: Vec<String> = diffs.iter().map(|(p, _)| cell_key(&p.path, f)).collect();
                let n = keys.iter().filter(|k| s.chosen.contains(*k)).count();
                let check = if n == 0 { 0 } else if n == keys.len() { 1 } else { 2 };
                let fills = diffs.iter().filter(|(_, c)| c.from.is_empty()).count();
                let replaces = diffs.len() - fills;
                let mut froms: Vec<&str> = diffs.iter().map(|(_, c)| c.from.as_str()).collect();
                froms.dedup();
                let mut tos: Vec<&str> = diffs.iter().map(|(_, c)| c.to.as_str()).collect();
                tos.dedup();
                let from = if froms.len() > 1 {
                    (t("source.n_values").replace("{n}", &froms.len().to_string()), 1)
                } else if froms[0].is_empty() {
                    (t("source.empty"), 2)
                } else {
                    (froms[0].to_string(), 0)
                };
                let effect = if replaces == 0 {
                    (t("source.fills").replace("{n}", &fills.to_string()), true)
                } else if fills == 0 {
                    (t("source.replaces").replace("{n}", &replaces.to_string()), false)
                } else {
                    (t("source.fills_replaces").replace("{f}", &fills.to_string()).replace("{r}", &replaces.to_string()), false)
                };
                let details = pairs
                    .iter()
                    .map(|p| {
                        let cell = p.cells.iter().find(|c| c.field == *f);
                        let d = cell.filter(|c| c.state == 0);
                        let key = d.map(|_| cell_key(&p.path, f));
                        Detail {
                            on: key.as_ref().is_some_and(|k| s.chosen.contains(k)),
                            key,
                            number: format!("{}{}", if p.track.disc > 1 { format!("{}-", p.track.disc) } else { String::new() }, p.track.position),
                            from: cell.map(|c| c.from.clone()).filter(|x| !x.is_empty()).unwrap_or_else(|| "—".into()),
                            to: match cell {
                                Some(c) if c.state == 2 => t("source.not_provided"),
                                Some(c) if c.state == 0 => c.to.clone(),
                                _ => t("source.identical_short"),
                            },
                        }
                    })
                    .collect();
                Some(FieldRow {
                    field: f.to_string(),
                    label: t(&format!("tags.{f}")),
                    check,
                    from,
                    to: if tos.len() > 1 { format!("{} …", tos[0]) } else { tos[0].to_string() },
                    effect,
                    open: s.opened.contains(*f),
                    details,
                })
            })
            .collect()
    }

    /// La ligne de pochette : case, pochette actuelle, comble seulement, effet.
    pub fn source_cover_row(&self) -> Option<(u8, Option<String>, bool, String)> {
        let s = self.views.popin.as_ref().and_then(|p| p.source())?;
        s.cover.as_ref()?;
        let pairs = self.source_pairs();
        let keys: Vec<String> = pairs.iter().map(|p| cell_key(&p.path, "cover")).collect();
        let n = keys.iter().filter(|k| s.chosen.contains(*k)).count();
        let check = if n == 0 { 0 } else if n == keys.len() { 1 } else { 2 };
        let current = pairs.iter().find(|p| p.has_cover).and_then(|p| self.views.tags.as_ref()?.files.iter().find(|f| f.path == p.path)?.shown_cover());
        let fills = pairs.iter().filter(|p| !p.has_cover).count();
        let only = fills == keys.len();
        let effect = if only { t("source.fills").replace("{n}", &fills.to_string()) } else { t("source.replaces").replace("{n}", &(keys.len() - fills).to_string()) };
        Some((check, current, only, effect))
    }

    /// Champs identiques partout, et non fournis par la source.
    pub fn source_same_missing(&self) -> (Vec<String>, Vec<String>) {
        let pairs = self.source_pairs();
        if pairs.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let state = |f: &str| {
            let cells: Vec<u8> = pairs.iter().filter_map(|p| p.cells.iter().find(|c| c.field == f).map(|c| c.state)).collect();
            if cells.iter().all(|s| *s == 2) {
                2
            } else if cells.contains(&0) {
                0
            } else {
                1
            }
        };
        let label = |f: &&str| t(&format!("tags.{f}"));
        (SOURCE_FIELDS.iter().filter(|f| state(f) == 1).map(label).collect(), SOURCE_FIELDS.iter().filter(|f| state(f) == 2).map(label).collect())
    }

    /// L'appariement des pistes.
    pub fn source_associations(&self) -> Vec<Association> {
        let Some(s) = self.views.popin.as_ref().and_then(|p| p.source()) else { return Vec::new() };
        let Some(album) = &s.album else { return Vec::new() };
        s.matches
            .iter()
            .map(|m| {
                let track = album.tracks.iter().find(|t| Some(t.key) == m.remote);
                let title = s.targets.get(m.local).map(|p| self.workshop_value(p, "title")).filter(|x| !x.is_empty()).unwrap_or_else(|| "—".into());
                Association {
                    local: m.local,
                    title,
                    confidence: if m.confidence == Confidence::Rejected || track.is_none() { 2 } else if m.confidence == Confidence::Doubtful { 1 } else { 0 },
                    remote: track.map(|t| format!("{}{}. {}", if t.disc > 1 { format!("{}-", t.disc) } else { String::new() }, t.position, t.title)).unwrap_or_else(|| "—".into()),
                    duration: track.map(|t| duration(t.duration)).unwrap_or_default(),
                    excluded: s.excluded.contains(&m.local),
                }
            })
            .collect()
    }

    /// Le pied : changements et fichiers.
    pub fn source_counts(&self) -> (usize, usize) {
        let Some(s) = self.views.popin.as_ref().and_then(|p| p.source()) else { return (0, 0) };
        let pairs = self.source_pairs();
        let mut files = std::collections::HashSet::new();
        let mut n = 0;
        for p in &pairs {
            for c in p.cells.iter().filter(|c| c.state == 0) {
                if s.chosen.contains(&cell_key(&p.path, c.field)) {
                    n += 1;
                    files.insert(p.path.clone());
                }
            }
            if s.cover.is_some() && s.chosen.contains(&cell_key(&p.path, "cover")) {
                n += 1;
                files.insert(p.path.clone());
            }
        }
        (n, files.len())
    }

    /// Score d'accord, nombre de paires, tout sûr.
    pub fn source_fit(&self) -> (i64, usize, bool) {
        let Some(s) = self.views.popin.as_ref().and_then(|p| p.source()) else { return (0, 0, false) };
        let kept: Vec<f32> = s.matches.iter().filter(|m| m.confidence != Confidence::Rejected).map(|m| m.score).collect();
        let fit = if kept.is_empty() { 0 } else { (kept.iter().sum::<f32>() / kept.len() as f32 * 100.).round() as i64 };
        let pairs = self.source_pairs();
        let sure = pairs.len() == s.targets.len() && pairs.iter().all(|p| !p.doubtful);
        (fit, pairs.len(), sure)
    }

    /// « Vos fichiers » : nombre de pistes, année commune, champs vides partout.
    pub fn source_files_line(&self) -> (String, Option<String>, Option<String>) {
        let Some(s) = self.views.popin.as_ref().and_then(|p| p.source()) else { return (String::new(), None, None) };
        let years: std::collections::HashSet<String> = s.targets.iter().map(|p| self.workshop_value(p, "year")).filter(|y| !y.is_empty()).collect();
        let year = (years.len() == 1).then(|| years.into_iter().next().unwrap_or_default());
        let empty: Vec<String> = SOURCE_FIELDS
            .iter()
            .filter(|f| !s.targets.is_empty() && s.targets.iter().all(|p| self.workshop_value(p, f).is_empty()))
            .map(|f| t(&format!("tags.{f}")).to_lowercase())
            .collect();
        let empty = (!empty.is_empty()).then(|| t("source.empty_fields").replace("{f}", &empty.join(", ")));
        (plural(s.targets.len(), "source.track_one", "source.track_many"), year, empty)
    }
}

impl crate::core::metadata_source::model::AlbumHit {
    /// L'édition (« Deluxe, 2011 »), sinon le titre.
    pub fn display_title(&self) -> String {
        self.detail.clone().unwrap_or_else(|| self.title.clone())
    }

    /// « 12 pistes ».
    pub fn tracks_label(&self) -> String {
        plural(self.track_count as usize, "source.track_one", "source.track_many")
    }
}

impl RemoteCover {
    /// « 1200 × 1200 ».
    pub fn size_label(&self) -> String {
        format!("{} × {}", self.size.0, self.size.1)
    }
}

impl MusicApp {
    /// Sous le titre : la page de l'atelier et le nombre de fichiers visés.
    pub fn source_subtitle(&self, so: &SourceState) -> String {
        let page = self.views.tags.as_ref().map(|p| p.source.clone()).unwrap_or_default();
        format!("{page} · {}", plural(so.targets.len(), "workshop.file_one", "workshop.file_many"))
    }
}

impl SourceState {
    /// Plusieurs fichiers : le détail par piste.
    pub fn has_many_targets(&self) -> bool {
        self.targets.len() > 1
    }

    /// « Pour vos 12 fichiers ».
    pub fn fit_your_label(&self) -> String {
        karbon_ui::intl::t_with("source.fit_your", &[("n", &self.targets.len().to_string())])
    }

    /// « 3 éditions » qui ont autant de pistes que vos fichiers.
    pub fn fitting_label(&self) -> String {
        plural(self.groups().0.len(), "source.edition_one", "source.edition_many")
    }

    /// La vignette de l'édition retenue.
    pub fn picked_thumb(&self) -> Option<&String> {
        self.picked.as_ref().and_then(|id| self.thumbs.get(id))
    }

    /// « MusicBrainz · Deluxe · 12 pistes · 92 % de concordance ».
    pub fn album_subtitle(&self, album: &AlbumDetail, fit: i64) -> String {
        [
            Some(self.source_name().to_string()),
            self.picked_hit().and_then(|h| h.detail.clone()),
            Some(plural(album.tracks.len(), "source.track_one", "source.track_many")),
            Some(karbon_ui::intl::t_with("source.fit", &[("n", &fit.to_string())])),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
    }

    /// L'aide du mode choisi (compléter, tout remplacer, au choix).
    pub fn mode_hint(&self) -> String {
        t(&format!("source.mode_{}_hint", ["fill", "all", "custom"][self.mode as usize]))
    }

    /// « 9 / 12 » pistes appariées.
    pub fn matched_label(&self, matched: usize) -> String {
        format!("{matched} / {}", self.targets.len())
    }

    pub fn source_name(&self) -> &'static str {
        if self.musicbrainz {
            "MusicBrainz"
        } else {
            "Deezer"
        }
    }

    pub fn picked_hit(&self) -> Option<&crate::core::metadata_source::model::AlbumHit> {
        self.hits.iter().find(|h| Some(&h.id) == self.picked.as_ref())
    }

    /// Éditions au bon nombre de pistes, puis les autres.
    pub fn groups(&self) -> (Vec<&crate::core::metadata_source::model::AlbumHit>, Vec<&crate::core::metadata_source::model::AlbumHit>) {
        let n = self.targets.len() as u32;
        (self.hits.iter().filter(|h| h.track_count == n).collect(), self.hits.iter().filter(|h| h.track_count != n).collect())
    }

    /// La pastille d'une édition : (texte, ton 0 ok, 1 ambre, 2 neutre).
    pub fn pill(&self, count: u32) -> (String, u8) {
        let n = self.targets.len() as i64;
        let diff = count as i64 - n;
        if diff == 0 {
            (format!("{count} / {n} {}", t("source.tracks")), 0)
        } else if diff > 0 {
            (t("source.extra").replace("{n}", &count.to_string()).replace("{d}", &diff.to_string()), 1)
        } else {
            (t("source.short_by").replace("{n}", &count.to_string()).replace("{d}", &(-diff).to_string()), 2)
        }
    }
}
