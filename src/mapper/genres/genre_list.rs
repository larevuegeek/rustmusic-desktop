//! La liste des genres : ce qu'affichent ses gabarits, bâti depuis l'état de la page.

use crate::models::genres::*;
use karbon_ui::ui::prelude::*;
use crate::models::app::Choice;
use crate::models::app::Msg;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use crate::helper::string::string::hue;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Route;
use crate::mapper::albums::natural_cmp;
use crate::mapper::albums::sort_key;
use crate::mapper::albums::sort_letter;
use crate::models::albums::AlbumsMsg;
use crate::models::albums::ViewMode;
use crate::mapper::tracks::track_list::LETTERS;
use crate::entity::library::genre_view::GenreView;

/// Clé de mémoire d'une carte (`memo={…}`) : ce qu'elle affiche (les dégradés et le
/// message d'ouverture découlent du nom et de la couleur).
impl std::hash::Hash for GenreItem {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        let c = self.color;
        (&self.name, &self.label, &self.covers, [c.red, c.green, c.blue, c.alpha]).hash(h);
        (&self.albums_n, &self.albums_label, &self.tracks_n, &self.tracks_label, &self.artists, &self.part).hash(h);
        (self.bar.to_bits(), self.special).hash(h);
    }
}

pub const SORTS: [(&str, &str, &str); 4] = [
    ("tracks", "genres_view.sort_tracks", "material-symbols:library-music-outline-rounded"),
    ("albums", "genres_view.sort_albums", "material-symbols:album-outline-rounded"),
    ("name", "genres_view.sort_name", "material-symbols:sort-by-alpha-rounded"),
    ("recent", "genres_view.sort_recent", "material-symbols:history-rounded"),
];

pub(crate) fn gradient(angle: f32, from: Color, to: Color) -> Vec<Background> {
    vec![LinearGradient::new(angle, [GradientStop { position: 0., color: from }, GradientStop { position: 1., color: to }]).into()]
}

pub(crate) fn mix_black(c: Color, part: f32) -> Color {
    let m = |v: u8| (f32::from(v) * part).round() as u8;
    Color::rgba(m(c.red), m(c.green), m(c.blue), 255)
}

/// `formatNumber(part, part < 1 ? 1 : 0)` : « 12 », « 0,4 ».
pub(crate) fn share(p: f64) -> String {
    if p < 1. {
        format!("{:.1}", p).trim_end_matches('0').trim_end_matches('.').replace('.', ",")
    } else {
        format!("{:.0}", p)
    }
}

impl GenreItem {
    /// Ancre du « … » de la carte ou de la ligne.
    pub fn more_id(&self) -> String {
        format!("genre.more.{}", self.name)
    }

    pub fn menu(&self) -> Msg {
        Msg::CollOpen(crate::models::tracks::TrackMenuAt::Named(self.more_id()), self.label.clone(), crate::models::library::CollectionSource::Genre(self.name.clone()))
    }

    /// « 120 titres · 8 albums ».
    pub fn summary(&self) -> String {
        format!("{} · {} {}", self.tracks_label, self.albums_n, self.albums_label)
    }
}

impl GenresPage {
    pub fn active_filters(&self) -> bool {
        !self.search.trim().is_empty() || self.main_items || self.without_genre
    }
}

/// Les genres par nom, nombre d'albums, dernière écoute ou nombre de titres (par défaut).
pub(crate) fn sort_genres(list: &mut [&GenreView], sort: &str) {
    let by_name = |x: &&GenreView, y: &&GenreView| natural_cmp(&sort_key(&x.name), &sort_key(&y.name));
    match sort {
        "albums" => list.sort_by(|x, y| y.total_albums.cmp(&x.total_albums).then(by_name(x, y))),
        "name" => list.sort_by(by_name),
        "recent" => list.sort_by(|x, y| y.last_played_at.cmp(&x.last_played_at).then(by_name(x, y))),
        _ => list.sort_by(|x, y| y.total_tracks.cmp(&x.total_tracks).then(by_name(x, y))),
    }
}

impl MusicApp {
    pub(crate) fn genre_item(&self, g: &GenreView, total: i64, max: i64, special: bool) -> GenreItem {
        let h = if special { 0. } else { hue(&g.name) };
        let chroma = if special { 0. } else { 1. };
        let color = if special { Color::rgba(0x8a, 0x93, 0x8e, 255) } else { Color::oklch(0.7, 0.17, h, 1.) };
        let tiles = std::array::from_fn(|k| {
            let kf = k as f32;
            let hk = (h + kf * 23.) % 360.;
            gradient(45. + kf * 40., Color::oklch(0.55 - kf * 0.06, 0.1 * chroma, hk, 1.), Color::oklch(0.2, 0.04 * chroma, hk, 1.))
        });
        let covers = std::array::from_fn(|k| g.covers.get(k).cloned());
        let fade = vec![
            // CSS 180deg (vers le bas) : l'API compte depuis la droite (CSS − 90°).
            LinearGradient::new(
                90.,
                [
                    GradientStop { position: 0.35, color: Color::rgba(0, 0, 0, 0) },
                    GradientStop { position: 1., color: mix_black(color, 0.35) },
                ],
            )
            .into(),
        ];
        let part = g.total_tracks as f64 / total.max(1) as f64 * 100.;
        let pl = |n: i64, one: &str, many: &str| t(if n == 1 { one } else { many });
        GenreItem {
            name: g.name.clone(),
            label: if special { t("genres_view.untagged") } else { g.name.clone() },
            covers,
            tiles,
            color,
            fade,
            albums_n: intl::number(g.total_albums),
            albums_label: pl(g.total_albums, "library_head.albums_one", "library_head.albums_n"),
            tracks_n: intl::number(g.total_tracks),
            tracks_label: pl(g.total_tracks, "library_head.tracks_one", "library_head.tracks_n"),
            artists: if special { t("genres_view.untagged_hint") } else { g.top_artists.join(", ") },
            part: t_with("genres_view.share", &[("p", &share(part))]),
            bar: (g.total_tracks as f32 / max.max(1) as f32 * 100.).clamp(0., 100.),
            special,
            open: if special {
                Msg::Genres(GenresMsg::OpenUntagged)
            } else {
                Msg::Go(Route::Genre(g.name.clone()))
            },
        }
    }

    /// Les genres filtrés et triés (sans la carte « Sans genre »).
    pub(crate) fn genres_visible(&self) -> Vec<&GenreView> {
        let Some(page) = self.views.genres.as_ref() else { return Vec::new() };
        if page.without_genre {
            return Vec::new();
        }
        let genres = &self.data.genres;
        let total = match self.data.library() {
            Some(l) if l.total_tracks > 0 => l.total_tracks,
            _ => genres.iter().map(|g| g.total_tracks).sum(),
        };
        let threshold = (total as f64 * 0.03).max(50.);
        let q = page.search.trim().to_lowercase();
        let mut visible: Vec<&GenreView> = genres
            .iter()
            .filter(|g| (q.is_empty() || format!("{} {}", g.name, g.top_artists.join(", ")).to_lowercase().contains(&q)) && (!page.main_items || g.total_tracks as f64 >= threshold))
            .collect();
        sort_genres(&mut visible, &page.sort);
        visible
    }


    pub fn genres_view(&self) -> Option<GenresView> {
        let page = self.views.genres.as_ref()?;
        let library = self.data.library()?;
        let genres = &self.data.genres;
        let total = if library.total_tracks > 0 { library.total_tracks } else { genres.iter().map(|g| g.total_tracks).sum() };
        let max = genres.iter().map(|g| g.total_tracks).max().unwrap_or(1);
        let without: Vec<_> = self.data.albums.iter().filter(|a| a.genre.is_none()).collect();
        let pseudo = GenreView {
            name: String::new(),
            total_albums: without.len() as i64,
            total_tracks: without.iter().map(|a| a.total_tracks).sum(),
            covers: without.iter().filter_map(|a| a.cover_url.clone()).take(4).collect(),
            top_artists: Vec::new(),
            last_played_at: None,
        };
        let visible = self.genres_visible();
        let letters = LETTERS.iter().map(|l| (*l, visible.iter().any(|g| sort_letter(&g.name) == *l))).collect();
        let items: Vec<GenreItem> = if page.without_genre {
            vec![self.genre_item(&pseudo, total, max, true)]
        } else {
            visible[page.window.range(visible.len())].iter().map(|g| self.genre_item(g, total, max, false)).collect()
        };
        let target = page.window.target_slot().and_then(|k| items.get(k)).map(|g| g.name.clone());
        let (sort_label, sort_icon) = SORTS
            .iter()
            .find(|(k, ..)| *k == page.sort)
            .map(|(_, l, i)| (t(l), *i))
            .unwrap_or_else(|| (t(SORTS[0].1), SORTS[0].2));
        let sorts = SORTS
            .iter()
            .map(|(k, l, i)| Choice { label: t(l), icon: (*i).into(), active: *k == page.sort, msg: Msg::Genres(GenresMsg::Sort((*k).into())), icon_only: false })
            .collect();
        let views = [(ViewMode::Grid, "albums_view.grid", "material-symbols:grid-view-outline-rounded"), (ViewMode::List, "albums_view.list", "material-symbols:view-list-outline-rounded")]
            .into_iter()
            .map(|(mode, l, i)| Choice { label: t(l), icon: i.into(), active: self.views.view_mode == mode, msg: Msg::Albums(AlbumsMsg::View(mode)), icon_only: true })
            .collect();
        Some(GenresView {
            items,
            target,
            untagged: pseudo.total_albums,
            untagged_n: intl::number(pseudo.total_albums),
            sort_label,
            sort_icon,
            sorts,
            views,
            placeholder: t_with("genres_view.filter", &[("n", &intl::number(genres.len() as i64))]),
            letters,
        })
    }

    pub(crate) fn genres_covers(&self) -> Vec<String> {
        self.genres_view()
            .map(|v| v.items.into_iter().flat_map(|i| i.covers.into_iter().flatten()).map(|p| crate::app::resources::assets::small_key(&p)).collect())
            .unwrap_or_default()
    }
}
