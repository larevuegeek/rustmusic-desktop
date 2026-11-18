//! La page de recherche : ce qu'affichent ses gabarits, bâti depuis l'état de la page,
//! et les pochettes à charger.

use crate::models::search::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::t;
use crate::models::app::Choice;
use crate::models::app::Msg;
use crate::mapper::artists::avatar;
use crate::mapper::tracks::short_format;
use crate::mapper::tracks::tier;
use crate::entity::search::search_result::SearchResult;

impl MusicApp {
    pub(crate) fn search_hit(&self, n: usize, r: &SearchResult) -> Hit {
        let details = r.path.as_ref().and_then(|p| self.views.search.records.iter().find(|f| f.path == *p));
        let round = r.result_type == "artist" || r.result_type == "composer";
        Hit {
            n,
            kind: r.result_type.clone(),
            title: r.title.clone(),
            subtitle: if r.result_type == "track" {
                [r.subtitle.clone(), details.and_then(|f| f.album.clone())].into_iter().flatten().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")
            } else {
                r.subtitle.clone().unwrap_or_default()
            },
            cover: r.thumbnail_path.clone(),
            round,
            avatar: (r.result_type == "composer" && r.thumbnail_path.is_none()).then(|| avatar(&r.title)),
            type_label: t(match r.result_type.as_str() {
                "track" => "track_view.kicker",
                "album" => "sidebar.album",
                "composer" => "composers_view.kicker",
                _ => "sidebar.artist",
            }),
            quality: details
                .filter(|f| f.audio_format.is_some())
                .map(|f| (tier(f.audio_format.as_deref(), f.bits_per_sample, f.sample_rate), short_format(f.audio_format.as_deref(), f.bits_per_sample, f.sample_rate))),
            duration: details.and_then(|f| f.duration).filter(|d| *d > 0.).map(intl::clock).unwrap_or_default(),
        }
    }

    pub fn search_view(&self) -> SearchView {
        let page = &self.views.search;
        let by = |k: &str| -> Vec<Hit> { page.results.iter().enumerate().filter(|(_, r)| r.result_type == k).map(|(i, r)| self.search_hit(i, r)).collect() };
        let (tracks, albums, artists, composers) = (by("track"), by("album"), by("artist"), by("composer"));
        // Le meilleur résultat : un album d'abord, sinon un morceau, un artiste, un compositeur.
        let top = albums.first().or(tracks.first()).or(artists.first()).or(composers.first()).cloned();
        let mut counts = vec![
            (intl::number(tracks.len() as i64), t("library.tracks").to_lowercase()),
            (intl::number(albums.len() as i64), t("library.albums").to_lowercase()),
            (intl::number(artists.len() as i64), t("library.artists").to_lowercase()),
        ];
        if !composers.is_empty() {
            counts.push((intl::number(composers.len() as i64), t("composers_view.mode_composers").to_lowercase()));
        }
        let mut filtres = vec![Choice { label: t("search.tab_all"), icon: String::new(), active: page.filtre == "all", msg: Msg::SearchPage(SearchMsg::Filter("all")), icon_only: false }];
        for (k, l, n) in [("tracks", "library.tracks", tracks.len()), ("albums", "library.albums", albums.len()), ("artists", "library.artists", artists.len()), ("composers", "composers_view.mode_composers", composers.len())] {
            if n > 0 {
                filtres.push(Choice { label: format!("{} · {}", t(l), intl::number(n as i64)), icon: String::new(), active: page.filtre == k, msg: Msg::SearchPage(SearchMsg::Filter(k)), icon_only: false });
            }
        }
        // « Voir tous les résultats » quand la section en montre moins que le total.
        let section = |key: &str, total: usize, shown: usize, filter: &'static str| SearchSection {
            title: t(key),
            count: (total > 0).then(|| intl::number(total as i64)),
            see_all: (total > shown).then(|| Msg::SearchPage(SearchMsg::Filter(filter))),
        };
        let tracks_beside_top = section("library.tracks", tracks.len(), 5, "tracks");
        let tracks_head = section("library.tracks", tracks.len(), 8, "tracks");
        let albums_head = section("library.albums", albums.len(), 6, "albums");
        let artists_head = section("library.artists", artists.len(), 6, "artists");
        let composers_head = section("composers_view.mode_composers", composers.len(), 6, "composers");
        SearchView {
            top_head: SearchSection { title: t("search.top_result"), count: None, see_all: None },
            tracks_beside_top,
            tracks_head,
            albums_head,
            artists_head,
            composers_head,
            query: page.query.clone(),
            counts,
            has_tracks: !tracks.is_empty(),
            empty: page.results.is_empty(),
            loading: page.loading,
            filtres,
            filtre: page.filtre,
            top,
            tracks,
            albums,
            artists,
            composers,
        }
    }

    pub(crate) fn search_covers(&self) -> Vec<String> {
        let v = self.search_view();
        let small = crate::app::resources::assets::small_key;
        let mut paths: Vec<String> = v.top.iter().filter_map(|h| h.cover.clone()).collect();
        paths.extend(v.tracks.iter().filter_map(|h| h.cover.as_deref().map(small)));
        paths.extend(v.albums.iter().chain(&v.artists).chain(&v.composers).filter_map(|h| h.cover.clone()));
        paths
    }
}
