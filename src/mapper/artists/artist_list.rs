//! La liste des artistes : ce qu'affichent ses gabarits, bâti depuis l'état de la page,
//! et les pochettes à charger.

use crate::models::artists::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Choice;
use crate::models::app::Msg;
use crate::mapper::albums::natural_cmp;
use crate::mapper::albums::sort_key;
use crate::mapper::albums::sort_letter;
use crate::models::albums::AlbumsMsg;
use crate::models::albums::ViewMode;
use crate::mapper::tracks::track_list::LETTERS;
use crate::models::artists::ArtistItem;

pub const SORTS: [(&str, &str, &str); 4] = [
    ("name", "artists_view.sort_name", "material-symbols:sort-by-alpha-rounded"),
    ("albums", "artists_view.sort_albums", "material-symbols:album-outline-rounded"),
    ("tracks", "artists_view.sort_tracks", "material-symbols:music-note-rounded"),
    ("duration", "artists_view.sort_duration", "material-symbols:schedule-outline-rounded"),
];

impl ArtistsPage {
    pub fn active_filters(&self) -> bool {
        !self.search.trim().is_empty() || self.without_portrait
    }
}

impl MusicApp {
    pub(crate) fn by_composer(&self) -> bool {
        self.data.setting("artists_tab") == Some("composers")
            && self.views.artists.as_ref().is_some_and(|p| !p.composers.is_empty())
    }

    pub(crate) fn artists_filtered(&self) -> Vec<Entry> {
        let Some(page) = self.views.artists.as_ref() else { return Vec::new() };
        let mut list: Vec<Entry> = if self.by_composer() {
            page.composers
                .iter()
                .map(|c| Entry {
                    item: ArtistItem::from_composer(c),
                    key: sort_key(&c.name),
                    letter: sort_letter(&c.name),
                    name: c.name.to_lowercase(),
                    albums: c.total_albums as i64,
                    tracks: c.total_tracks as i64,
                    duration: c.total_duration,
                    portrait: c.thumbnail_path.is_some(),
                })
                .collect()
        } else {
            self.data
                .artists
                .iter()
                .map(|a| Entry {
                    item: ArtistItem::from_view(a),
                    key: sort_key(&a.name),
                    letter: sort_letter(&a.name),
                    name: a.name.to_lowercase(),
                    albums: a.total_albums,
                    tracks: a.total_tracks,
                    duration: a.total_duration,
                    portrait: a.thumbnail_path.is_some(),
                })
                .collect()
        };
        let by_name = |x: &Entry, y: &Entry| natural_cmp(&x.key, &y.key);
        match page.sort.as_str() {
            "albums" => list.sort_by(|x, y| y.albums.cmp(&x.albums).then(by_name(x, y))),
            "tracks" => list.sort_by(|x, y| y.tracks.cmp(&x.tracks).then(by_name(x, y))),
            "duration" => list.sort_by(|x, y| y.duration.partial_cmp(&x.duration).unwrap_or(std::cmp::Ordering::Equal).then(by_name(x, y))),
            _ => list.sort_by(by_name),
        }
        let q = page.search_term.trim().to_lowercase();
        list.retain(|e| (q.is_empty() || e.name.contains(&q)) && (!page.without_portrait || !e.portrait));
        list
    }

    pub fn artists_view(&self) -> Option<ArtistsView> {
        let page = self.views.artists.as_ref()?;
        let by_composer = self.by_composer();
        let filtered = self.artists_filtered();
        let total = filtered.len();
        let letters = LETTERS.iter().map(|l| (*l, filtered.iter().any(|e| e.letter == *l))).collect();
        let range = page.window.range(total);
        let (start, end) = (range.start, range.end);
        let items: Vec<ArtistItem> = filtered.into_iter().skip(start).take(end - start).map(|e| e.item).collect();
        let (sort_label, sort_icon) = SORTS
            .iter()
            .find(|(k, ..)| *k == page.sort)
            .map(|(_, l, i)| (t(l), *i))
            .unwrap_or_else(|| (t("artists_view.sort_name"), SORTS[0].2));
        let sorts = SORTS
            .iter()
            .map(|(k, l, i)| Choice {
                label: t(l),
                icon: (*i).into(),
                active: *k == page.sort,
                msg: Msg::Artists(ArtistsMsg::Sort((*k).into())),
                icon_only: false,
            })
            .collect();
        let views = [(ViewMode::Grid, "albums_view.grid", "material-symbols:grid-view-outline-rounded"), (ViewMode::List, "albums_view.list", "material-symbols:view-list-outline-rounded")]
            .into_iter()
            .map(|(mode, l, i)| Choice {
                label: t(l),
                icon: i.into(),
                active: self.views.view_mode == mode,
                msg: Msg::Albums(AlbumsMsg::View(mode)),
                icon_only: true,
            })
            .collect();
        let n = if by_composer { page.composers.len() } else { self.data.artists.len() };
        let placeholder = t_with(if by_composer { "composers_view.filter" } else { "artists_view.filter" }, &[("n", &intl::number(n as i64))]);
        let modes = (!page.composers.is_empty()).then(|| (intl::number(self.data.artists.len() as i64), intl::number(page.composers.len() as i64)));
        Some(ArtistsView {
            target: page.window.target_slot().and_then(|k| items.get(k)).map(|it| it.id.clone()),
            items,
            total,
            shown_from: start,
            shown_to: end,
            sort_label,
            sort_icon,
            sorts,
            views,
            placeholder,
            letters,
            modes,
            by_composer,
        })
    }


    /// Les portraits de la tranche affichée.
    pub(crate) fn artists_covers(&self) -> Vec<String> {
        self.artists_view()
            .map(|v| v.items.into_iter().filter_map(|i| i.portrait).collect())
            .unwrap_or_default()
    }
}
