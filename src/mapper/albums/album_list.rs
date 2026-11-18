//! La liste des albums : ce qu'affichent ses gabarits, bâti depuis l'état de la page.

use crate::models::albums::*;
use crate::mapper::albums::*;
use crate::models::app::Choice;
use crate::models::app::Msg;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Route;
use crate::mapper::tracks::track_list::LETTERS;
use crate::query::album::album_list_row::AlbumListRow;
use crate::models::shared::ListWindow;

/// Albums affichés au plus : leurs pochettes décodées (448 px) tiennent dans le cache
/// (`ImageLoader::budget` dans `assets.rs`) ; au-delà, le cache évince ce que la vue redemande aussitôt.
/// « Sans genre » dans le filtre de genre.
pub const NO_GENRE: &str = "__sans__";

pub const SORTS: [(&str, &str, &str); 4] = [
    ("title", "albums_view.sort_title", "material-symbols:sort-by-alpha-rounded"),
    ("artist", "albums_view.sort_artist", "material-symbols:person-outline-rounded"),
    ("year", "albums_view.sort_year", "material-symbols:calendar-month-outline-rounded"),
    ("added", "albums_view.sort_added", "material-symbols:schedule-outline-rounded"),
];

/// « 24/96 », « DSD64 », sinon rien.
pub fn hires_label(bits: Option<i64>, rate: Option<i64>) -> Option<String> {
    if bits == Some(1) {
        return Some(crate::helper::audio::dsd_label(rate.map_or(0, |r| r as u32)));
    }
    let (b, r) = (bits?, rate?);
    if b <= 16 && r <= 48_000 {
        return None;
    }
    Some(format!("{b}/{}", r / 1000))
}

impl AlbumsPage {
    /// Le filtre d'année : l'année choisie, ou « Année ».
    pub fn year_label(&self) -> String {
        self.year.map(|y| y.to_string()).unwrap_or_else(|| karbon_ui::intl::t("albums_view.year"))
    }

    pub(crate) fn new(library_id: i64) -> Self {
        Self {
            library_id,
            sort: "title".into(),
            search: String::new(),
            search_term: String::new(),
            search_gen: 0,
            hires: false,
            compilations: false,
            without_cover: false,
            genre: None,
            year: None,
            window: ListWindow::default(),
        }
    }

    pub fn active_filters(&self) -> bool {
        !self.search.trim().is_empty()
            || self.hires
            || self.compilations
            || self.without_cover
            || self.genre.is_some()
            || self.year.is_some()
    }
}

/// Les albums que montrent les filtres de la page, dans l'ordre choisi, avec leur lettre.
pub(crate) fn filter_albums<'a>(page: &AlbumsPage, albums: &'a [AlbumListRow]) -> Vec<(&'a AlbumListRow, &'static str)> {
    let q = page.search_term.trim().to_lowercase();
    let mut list: Vec<(&AlbumListRow, String, &'static str)> = albums
        .iter()
        .map(|a| (a, sort_key(&a.title), sort_letter(&a.title)))
        .filter(|(a, ..)| {
            let text = format!("{} {} {}", a.title, a.artist, a.genre.clone().unwrap_or_default()).to_lowercase();
            (q.is_empty() || text.contains(&q))
                && (!page.hires || hires_label(a.max_bits, a.max_sample_rate).is_some())
                && (!page.compilations || a.album_type == "compilation")
                && (!page.without_cover || a.cover_url.is_none())
                && match page.genre.as_deref() {
                    None => true,
                    Some(NO_GENRE) => a.genre.is_none(),
                    Some(g) => a.genre.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(g)),
                }
                && page.year.is_none_or(|y| a.year == Some(y))
        })
        .collect();
    match page.sort.as_str() {
        "artist" => list.sort_by(|x, y| natural_cmp(&x.0.artist, &y.0.artist).then(natural_cmp(&x.1, &y.1))),
        "year" => list.sort_by(|x, y| y.0.year.unwrap_or(0).cmp(&x.0.year.unwrap_or(0)).then(natural_cmp(&x.1, &y.1))),
        "added" => list.sort_by(|x, y| y.0.created_at.cmp(&x.0.created_at)),
        _ => list.sort_by(|x, y| natural_cmp(&x.1, &y.1)),
    }
    list.into_iter().map(|(a, _, l)| (a, l)).collect()
}

impl MusicApp {
    /// Les albums filtrés et triés, avec leur lettre.
    pub(crate) fn albums_filtered(&self) -> Vec<(&AlbumListRow, &'static str)> {
        let Some(page) = self.views.albums.as_ref() else { return Vec::new() };
        filter_albums(page, &self.data.albums)
    }

    pub fn albums_view(&self) -> Option<AlbumsView> {
        let page = self.views.albums.as_ref()?;
        let filtered = self.albums_filtered();
        let total = filtered.len();
        let range = page.window.range(total);
        let (start, end) = (range.start, range.end);
        let items: Vec<AlbumItem> = filtered[range]
            .iter()
            .map(|(a, _)| {
                let n = a.total_tracks;
                AlbumItem {
                    id: a.id.clone(),
                    title: a.title.clone(),
                    artist: if a.artist.is_empty() { t("albums_view.unknown_artist") } else { a.artist.clone() },
                    year: a.year.map(|y| y.to_string()).unwrap_or_default(),
                    cover: a.cover_url.clone(),
                    hires: hires_label(a.max_bits, a.max_sample_rate),
                    tracks: if n == 1 { t("home.track_one") } else { t_with("home.tracks_n", &[("n", &n.to_string())]) },
                    tracks_n: n.to_string(),
                    tracks_label: t(if n == 1 { "library_head.tracks_one" } else { "library_head.tracks_n" }),
                    duration: intl::duration(a.total_duration),
                    compilation: a.album_type == "compilation",
                    open: Msg::Go(Route::Album(a.id.clone())),
                }
            })
            .collect();
        // Un genre par nom, sans la casse : l'orthographe la plus fréquente.
        let mut spellings: Vec<(String, std::collections::HashMap<String, usize>)> = Vec::new();
        for a in &self.data.albums {
            let Some(g) = a.genre.as_ref() else { continue };
            let key = g.to_lowercase();
            match spellings.iter_mut().find(|(k, _)| *k == key) {
                Some((_, m)) => *m.entry(g.clone()).or_default() += 1,
                None => spellings.push((key, [(g.clone(), 1)].into_iter().collect())),
            }
        }
        let mut genres: Vec<(String, usize)> = spellings
            .into_iter()
            .map(|(_, m)| {
                let total = m.values().sum();
                let name = m.into_iter().max_by(|x, y| x.1.cmp(&y.1).then(y.0.cmp(&x.0))).map(|(n, _)| n).unwrap_or_default();
                (name, total)
            })
            .collect();
        genres.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let genres = genres
            .into_iter()
            .map(|(g, n)| {
                let active = page.genre.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(&g));
                (g, n.to_string(), active)
            })
            .collect();
        let mut by_year: std::collections::BTreeMap<i32, usize> = Default::default();
        for a in &self.data.albums {
            if let Some(y) = a.year {
                *by_year.entry(y).or_default() += 1;
            }
        }
        let years = by_year.into_iter().rev().map(|(y, n)| (y, n.to_string(), page.year == Some(y))).collect();
        let (sort_label, sort_icon) = SORTS
            .iter()
            .find(|(k, ..)| *k == page.sort)
            .map(|(_, l, i)| (t(l), *i))
            .unwrap_or_else(|| (t("albums_view.sort_title"), "material-symbols:sort-by-alpha-rounded"));
        let sorts = SORTS
            .iter()
            .map(|(k, l, i)| Choice {
                label: t(l),
                icon: (*i).into(),
                active: *k == page.sort,
                msg: Msg::Albums(AlbumsMsg::Sort((*k).into())),
                icon_only: false,
            })
            .collect();
        let views = vec![
            Choice {
                label: t("albums_view.grid"),
                icon: "material-symbols:grid-view-outline-rounded".into(),
                active: self.views.view_mode == ViewMode::Grid,
                msg: Msg::Albums(AlbumsMsg::View(ViewMode::Grid)),
                icon_only: true,
            },
            Choice {
                label: t("albums_view.list"),
                icon: "material-symbols:view-list-outline-rounded".into(),
                active: self.views.view_mode == ViewMode::List,
                msg: Msg::Albums(AlbumsMsg::View(ViewMode::List)),
                icon_only: true,
            },
        ];
        let genre_label = match page.genre.as_deref() {
            Some(NO_GENRE) => t("albums_view.untagged"),
            Some(g) => g.to_string(),
            None => t("albums_view.genre"),
        };
        let year_only = page.year.is_some()
            && page.search.trim().is_empty()
            && !page.hires
            && !page.compilations
            && !page.without_cover
            && page.genre.is_none();
        let empty = match page.year {
            Some(y) if year_only => t_with("library.year_none", &[("y", &y.to_string())]),
            _ => t("albums_view.no_match"),
        };
        let letters = LETTERS
            .iter()
            .map(|l| (*l, filtered.iter().any(|(_, x)| x == l)))
            .collect();
        Some(AlbumsView {
            target: page.window.target_slot().and_then(|k| items.get(k)).map(|it| it.id.clone()),
            items,
            total,
            shown_from: start,
            shown_to: end,
            sort_label,
            sort_icon,
            sorts,
            views,
            genres,
            genre_label,
            years,
            placeholder: t_with("albums_view.filter", &[("n", &intl::number(self.data.albums.len() as i64))]),
            empty,
            letters,
        })
    }


    /// Les pochettes de la tranche affichée.
    pub(crate) fn albums_covers(&self) -> Vec<String> {
        self.albums_view()
            .map(|v| v.items.into_iter().filter_map(|i| i.cover).collect())
            .unwrap_or_default()
    }
}
