//! La fiche d'un artiste : ce qu'affichent ses gabarits, bâti depuis l'état de la page,
//! et les pochettes à charger.

use crate::models::artists::*;
use std::collections::HashMap;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::mapper::albums::album_detail::album_item;
use crate::mapper::albums::album_detail::best_quality;
use crate::mapper::albums::album_detail::disc_order;
use crate::mapper::albums::album_detail::sort_tracks;
use crate::models::albums::AlbumTrack;
use crate::models::albums::SortChoice;
use crate::mapper::albums::natural_cmp;
use crate::mapper::albums::sort_key;
use crate::models::albums::AlbumItem;
use crate::models::albums::ViewMode;
use crate::models::artists::ArtistItem;
use crate::query::album::album_list_row::AlbumListRow;
use crate::query::track::track_list_row::TrackListRow;

/// Les morceaux regroupés par album, dans l'ordre du disque ; les albums par année
/// (`sort` = « year ») ou par titre, dans le sens choisi.
pub(crate) fn album_groups(tracks: &[TrackListRow], albums: &[AlbumListRow], sort: &str, asc: bool) -> Vec<Group> {
    let by_id: HashMap<&str, &AlbumListRow> = albums.iter().map(|a| (a.id.as_str(), a)).collect();
    let mut order: Vec<String> = Vec::new();
    let mut m: HashMap<String, Vec<TrackListRow>> = HashMap::new();
    for x in tracks {
        let key = x.album_id.clone().unwrap_or_else(|| format!("?{}", x.album.clone().unwrap_or_default()));
        if !m.contains_key(&key) {
            order.push(key.clone());
        }
        m.entry(key).or_default().push(x.clone());
    }
    let mut list: Vec<Group> = order
        .into_iter()
        .map(|key| {
            let mut tracks_list = m.remove(&key).unwrap_or_default();
            tracks_list.sort_by(disc_order);
            let a = by_id.get(key.as_str()).copied();
            let p = &tracks_list[0];
            Group {
                album: a.map(|a| a.id.clone()),
                title: a.map(|a| a.title.clone()).or_else(|| p.album.clone()).unwrap_or_else(|| t("artist_view.unknown_album")),
                year: a.and_then(|a| a.year).or_else(|| p.year.as_deref().and_then(|y| y.get(..4)?.parse().ok())),
                cover: a.and_then(|a| a.cover_url.clone()).or_else(|| p.thumbnail_path.clone()),
                key,
                tracks_list,
            }
        })
        .collect();
    let by_title = |x: &Group, y: &Group| natural_cmp(&sort_key(&x.title), &sort_key(&y.title));
    if sort == "year" {
        list.sort_by(|x, y| {
            let o = x.year.unwrap_or(0).cmp(&y.year.unwrap_or(0));
            (if asc { o } else { o.reverse() }).then(by_title(x, y))
        });
    } else {
        list.sort_by(|x, y| {
            let o = by_title(x, y);
            if asc { o } else { o.reverse() }
        });
    }
    list
}

impl MusicApp {
    pub(crate) fn artist_group(&self) -> bool {
        self.views.artist.as_ref().is_some_and(|p| p.sort == "year" || p.sort == "album")
    }

    pub(crate) fn artist_groups(&self) -> Vec<Group> {
        let Some(page) = self.views.artist.as_ref() else { return Vec::new() };
        if !self.artist_group() {
            return Vec::new();
        }
        album_groups(&page.tracks, &page.albums, &page.sort, page.asc)
    }

    /// L'ordre de lecture : l'ordre affiché, albums repliés compris.
    pub(crate) fn artist_order(&self) -> Vec<TrackListRow> {
        let Some(page) = self.views.artist.as_ref() else { return Vec::new() };
        if self.artist_group() {
            self.artist_groups().into_iter().flat_map(|g| g.tracks_list).collect()
        } else {
            let mut l = page.tracks.clone();
            sort_tracks(&mut l, &page.sort, page.asc);
            l
        }
    }

    pub fn artist_view(&self) -> Option<ArtistView> {
        let page = self.views.artist.as_ref()?;
        let artist = page.artist.as_ref()?;
        let grid = self.views.view_mode == ViewMode::Grid;
        let group = self.artist_group();
        let own: Vec<&AlbumListRow> = page.albums.iter().filter(|a| !a.participation).collect();
        let participations: Vec<&AlbumListRow> = page.albums.iter().filter(|a| a.participation).collect();
        let tracks = &page.tracks;
        let duration_s = Some(tracks.iter().map(|x| x.duration.unwrap_or(0.)).sum::<f64>()).filter(|d| *d > 0.).unwrap_or(artist.total_duration);
        let nb_albums = if page.loading_albums { own.len() as i64 } else { artist.total_albums };
        let track_count = if page.loading_tracks { tracks.len() as i64 } else { artist.total_tracks };
        let pl = |n: i64, one: &str, many: &str| t(if n == 1 { one } else { many });
        let ans: Vec<i32> = own.iter().filter_map(|a| a.year).filter(|y| *y > 0).collect();
        let period = match (ans.iter().min(), ans.iter().max()) {
            (Some(a), Some(b)) if a == b => Some(a.to_string()),
            (Some(a), Some(b)) => Some(format!("{a} – {b}")),
            _ => None,
        };
        let mut count: Vec<(String, usize)> = Vec::new();
        for x in tracks {
            if let Some(g) = x.genre.as_ref().map(|g| g.trim()).filter(|g| !g.is_empty()) {
                match count.iter_mut().find(|(k, _)| k == g) {
                    Some((_, n)) => *n += 1,
                    None => count.push((g.to_string(), 1)),
                }
            }
        }
        count.sort_by(|a, b| b.1.cmp(&a.1));
        let genres: Vec<String> = count.into_iter().take(3).map(|(g, _)| g).collect();

        let by_title = |x: &&AlbumListRow, y: &&AlbumListRow| natural_cmp(&sort_key(&x.title), &sort_key(&y.title));
        let mut disco = own.clone();
        if page.sort == "album" || page.sort == "title" {
            disco.sort_by(by_title);
        } else {
            disco.sort_by(|x, y| y.year.unwrap_or(0).cmp(&x.year.unwrap_or(0)).then(by_title(x, y)));
        }
        let discography = (!disco.is_empty()).then(|| {
            let items = disco
                .iter()
                .map(|a| AlbumItem { open: Msg::Artist(ArtistMsg::GoTo(a.id.clone())), ..album_item(a) })
                .collect();
            self.carousel(
                "discographie",
                t("artist_view.discography"),
                format!("{} {}", intl::number(disco.len() as i64), pl(disco.len() as i64, "library_head.albums_one", "library_head.albums_n")),
                items,
            )
        });

        let playing = self.data.player.path.as_deref();
        let line = |i: usize, x: &TrackListRow, number: String, invite: Option<String>| AlbumTrack {
            i,
            numbered: true,
            number,
            cover: x.thumbnail_path.clone(),
            title: x.title.clone(),
            invite,
            duration: x.duration.map(intl::clock).unwrap_or_default(),
            liked: self.data.liked.contains(&x.path),
            in_progress: playing == Some(x.path.as_str()),
            artwork: self.views.artist_artwork,
            mark: false,
        };
        let invite_of = |x: &TrackListRow| x.artist.clone().filter(|a| *a != artist.name);
        let halves = |mut l: Vec<AlbumTrack>| {
            let right = l.split_off(l.len().div_ceil(2));
            (l, right)
        };

        let mut groups = Vec::new();
        let (mut flat_left, mut flat_right, mut table, mut head) = (Vec::new(), Vec::new(), None, None);
        if group {
            let mut offset = 0;
            let mut previous_open = None;
            for g in self.artist_groups() {
                let open = page.open_items.contains(&g.key);
                let rows: Vec<AlbumTrack> = g
                    .tracks_list
                    .iter()
                    .enumerate()
                    .map(|(k, x)| line(offset + k, x, x.track_number.map(|n| n.to_string()).unwrap_or_else(|| (k + 1).to_string()), invite_of(x)))
                    .collect();
                let (left, right) = halves(rows);
                let group_table = (!grid && open).then(|| {
                    let mut tb = self.track_table(&g.tracks_list, false, None, true, true);
                    for r in tb.rows.iter_mut() {
                        r.n += offset;
                    }
                    tb
                });
                let duration = g.tracks_list.iter().map(|x| x.duration.unwrap_or(0.)).sum::<f64>();
                let n = g.tracks_list.len() as i64;
                groups.push(GroupView {
                    title: g.title,
                    year: g.year.map(|y| y.to_string()),
                    tracks_count: format!("{} {}", intl::number(n), pl(n, "library_head.tracks_one", "library_head.tracks_n")),
                    duration: intl::duration(duration),
                    quality: g.album.as_ref().and_then(|_| best_quality(&g.tracks_list)),
                    cover: g.cover,
                    album_route: g.album.map(Route::Album),
                    open,
                    space: match previous_open {
                        None => 2,
                        Some(true) => 7,
                        Some(false) => 1,
                    },
                    left,
                    right,
                    table: group_table,
                    key: g.key,
                });
                previous_open = Some(open);
                offset += g.tracks_list.len();
            }
            if !grid && !page.tracks.is_empty() {
                head = Some(self.track_table(&[], false, None, true, true));
            }
        } else {
            let order = self.artist_order();
            if grid {
                let rows: Vec<AlbumTrack> = order
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        let subline = [x.album.clone(), x.year.clone()].into_iter().flatten().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
                        line(i, x, (i + 1).to_string(), Some(subline).filter(|s| !s.is_empty()))
                    })
                    .collect();
                (flat_left, flat_right) = halves(rows);
            } else {
                table = Some(self.track_table(&order, false, Some(&page.sort), page.asc, false));
            }
        }

        let arrow = |k: &str| {
            (page.sort == k).then_some(if page.asc { "material-symbols:arrow-upward-rounded" } else { "material-symbols:arrow-downward-rounded" })
        };
        let sorts = [("year", "artist_view.sort_year"), ("album", "artist_view.sort_album"), ("title", "artist_view.sort_title")]
            .into_iter()
            .map(|(k, l)| SortChoice { key: k, label: t(l), active: page.sort == k, arrow: arrow(k) })
            .collect();
        let all_open = (group && groups.len() > 1).then(|| groups.iter().all(|g| g.open));
        let subtitle = t(if group {
            "artist_view.grouped"
        } else if page.sort == "title" {
            "artist_view.flat_title"
        } else {
            "artist_view.flat"
        });

        let participations = (!participations.is_empty()).then(|| {
            self.carousel(
                "participations",
                t("artist_view.appears_on"),
                t("artist_view.appears_on_desc"),
                participations.into_iter().map(album_item).collect(),
            )
        });
        let similar = (!page.similar.is_empty()).then(|| {
            let subline = match genres.first() {
                Some(g) => karbon_ui::intl::t_with("album_view.same_genre", &[("g", g)]),
                None => t("artist_view.same_genre"),
            };
            self.carousel("voisins", t("album_view.similar_artists"), subline, page.similar.iter().map(ArtistItem::from_view).collect())
        });

        let tint = page
            .portrait
            .clone()
            .or_else(|| page.albums.iter().find_map(|a| a.cover_url.clone()))
            .and_then(|c| self.assets.as_ref()?.covers.get(&c))
            .map(|img| img.dominant_hue())
            .unwrap_or((150., 0.));
        Some(ArtistView {
            portrait: page.portrait.clone(),
            tint,
            name: artist.name.clone(),
            name_long: artist.name.chars().count() > 24,
            albums_n: (nb_albums > 0).then(|| (intl::number(nb_albums), pl(nb_albums, "library_head.albums_one", "library_head.albums_n"))),
            tracks_n: (intl::number(track_count), pl(track_count, "library_head.tracks_one", "library_head.tracks_n")),
            duration: if duration_s > 0. { intl::duration(duration_s) } else { String::new() },
            period,
            liked_all: !tracks.is_empty() && tracks.iter().all(|x| self.data.liked.contains(&x.path)),
            has_tracks: !tracks.is_empty(),
            genres,
            discography,
            subtitle,
            sorts,
            all_open,
            loading: page.loading_tracks,
            groups,
            flat_left,
            flat_right,
            table,
            head,
            participations,
            similar,
        })
    }

    pub(crate) fn artist_covers(&self) -> Vec<String> {
        let Some(v) = self.artist_view() else { return Vec::new() };
        let mut paths: Vec<String> = v.portrait.into_iter().collect();
        for c in [&v.discography, &v.participations].into_iter().flatten() {
            paths.extend(c.items.iter().filter_map(|a| a.cover.clone()));
        }
        if let Some(c) = &v.similar {
            paths.extend(c.items.iter().filter_map(|a| a.portrait.clone()));
        }
        for g in &v.groups {
            paths.extend(g.cover.clone());
            if g.open {
                paths.extend(g.left.iter().chain(&g.right).filter_map(|x| x.cover.as_deref().map(crate::app::resources::assets::small_key)));
                if let Some(tb) = &g.table {
                    paths.extend(tb.rows.iter().filter_map(|r| r.cover.as_deref().map(crate::app::resources::assets::small_key)));
                }
            }
        }
        paths.extend(v.flat_left.iter().chain(&v.flat_right).filter_map(|x| x.cover.as_deref().map(crate::app::resources::assets::small_key)));
        if let Some(tb) = &v.table {
            paths.extend(tb.rows.iter().filter_map(|r| r.cover.as_deref().map(crate::app::resources::assets::small_key)));
        }
        paths
    }
}
