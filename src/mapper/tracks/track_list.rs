//! La liste des morceaux : ce qu'affichent ses gabarits, bâti depuis l'état de la page,
//! et les pochettes à charger.

use crate::models::tracks::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Choice;
use crate::models::app::Msg;

/// Les tris proposés (`SORTS`) : clé, libellé, icône.
pub const SORTS: [(&str, &str, &str); 6] = [
    ("title", "tracks_view.sort_title", "material-symbols:sort-by-alpha-rounded"),
    ("artist", "tracks_view.sort_artist", "material-symbols:person-outline-rounded"),
    ("album", "tracks_view.sort_album", "material-symbols:album-outline-rounded"),
    ("rating", "tracks_view.sort_rating", "material-symbols:star-outline-rounded"),
    ("duration", "tracks_view.sort_duration", "material-symbols:schedule-outline-rounded"),
    ("date", "tracks_view.sort_added", "material-symbols:calendar-month-outline-rounded"),
];

pub const LETTERS: [&str; 27] = [
    "#", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T",
    "U", "V", "W", "X", "Y", "Z",
];

impl TracksPage {
    pub(crate) fn new(library_id: i64) -> Self {
        Self {
            library_id,
            sort: "title".into(),
            asc: true,
            detailed: false,
            search: String::new(),
            sent: String::new(),
            search_gen: 0,
            hires: false,
            lossless: false,
            favorites: false,
            without_cover: false,
            genre: None,
            genres: None,
            tracks: Vec::new(),
            a_venir: Vec::new(),
            upcoming_start: None,
            start: 0,
            total: 0,
            loading_state: true,
            following: false,
            token: 0,
            view: 0,
            anchor: None,
            letters: Vec::new(),
        }
    }

    pub fn active_filters(&self) -> bool {
        !self.search.trim().is_empty()
            || self.hires
            || self.lossless
            || self.favorites
            || self.without_cover
            || self.genre.is_some()
    }

    pub(crate) fn params(&self) -> Params {
        Params {
            library: self.library_id,
            sort_dir: Some(if self.asc { "asc" } else { "desc" }.to_string()),
            filter: (!self.sent.is_empty()).then(|| self.sent.clone()),
            missing_cover: Some(self.without_cover),
            quality: if self.hires {
                Some("hires".into())
            } else if self.lossless {
                Some("lossless".into())
            } else {
                None
            },
            favorites: Some(self.favorites),
            genre: self.genre.clone(),
            sort_by: Some(self.sort.clone()),
        }
    }
}

impl MusicApp {
    /// Un lot reçu attend d'entrer dans la liste.
    pub fn tracks_has_frame_work(&self) -> bool {
        self.views.tracks.as_ref().is_some_and(|p| !p.a_venir.is_empty())
    }

    /// Le modèle de la page.
    pub fn tracks_view(&self) -> Option<TracksView> {
        let page = self.views.tracks.as_ref()?;
        let start = usize::try_from(page.start).unwrap_or(0);
        let mut table = self.track_table_at(&page.tracks, start, page.detailed, Some(&page.sort), page.asc, false);
        // `scrollToIndex(anchor)` : la ligne d'ancre est la cible initiale de la zone.
        table.target = page.anchor.and_then(|i| usize::try_from(i).ok());
        let (sort_label, sort_icon) = SORTS
            .iter()
            .find(|(k, ..)| *k == page.sort)
            .map(|(_, l, i)| (t(l), *i))
            .unwrap_or_else(|| (page.sort.trim_start_matches("tag:").trim_start_matches("custom:").to_string(), "material-symbols:sort-rounded"));
        let sorts = SORTS
            .iter()
            .map(|(k, l, i)| Choice {
                label: t(l),
                icon: (*i).into(),
                active: *k == page.sort,
                msg: Msg::Tracks(TracksMsg::Sort((*k).into())),
                icon_only: false,
            })
            .collect();
        let densities = vec![
            Choice {
                label: t("tracks_view.compact"),
                icon: "material-symbols:density-small-rounded".into(),
                active: !page.detailed,
                msg: Msg::Tracks(TracksMsg::Detailed(false)),
                icon_only: true,
            },
            Choice {
                label: t("tracks_view.detailed"),
                icon: "material-symbols:density-medium-rounded".into(),
                active: page.detailed,
                msg: Msg::Tracks(TracksMsg::Detailed(true)),
                icon_only: true,
            },
        ];
        let genres = page.genres.as_ref().map(|g| {
            g.iter()
                .map(|g| (g.name.clone(), intl::number(g.total_tracks), page.genre.as_deref() == Some(g.name.as_str())))
                .collect()
        });
        let loaded = t_with(
            "tracks_view.loaded",
            &[
                ("n", &intl::number(page.start + page.tracks.len() as i64)),
                ("total", &intl::number(page.total)),
            ],
        );
        let letters = LETTERS
            .iter()
            .map(|l| (*l, page.letters.iter().any(|(x, _)| x == l)))
            .collect();
        Some(TracksView {
            table,
            sort_label,
            sort_icon,
            sorts,
            densities,
            genre_label: page.genre.clone().unwrap_or_else(|| t("tracks_view.genre")),
            genres,
            loaded,
            letters,
        })
    }
}
