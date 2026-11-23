//! La fiche d'un genre : ce qu'affichent ses gabarits, bâti depuis l'état de la page.

use crate::models::genres::*;
use karbon_ui::ui::prelude::*;
use crate::models::app::Choice;
use crate::models::albums::AlbumTrack;
use crate::models::albums::SortChoice;
use crate::models::artists::ArtistItem;
use crate::query::track::track_list_row::TrackListRow;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use crate::helper::string::string::hue;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Msg;
use crate::mapper::albums::album_detail::album_item;
use crate::mapper::albums::album_detail::sort_tracks;
use crate::mapper::albums::natural_cmp;
use crate::mapper::albums::sort_key;
use crate::models::albums::ViewMode;
use crate::query::album::album_list_row::AlbumListRow;
use crate::query::artist::artist_list_row::ArtistListRow;
use crate::models::shared::ListWindow;

/// Les cases de la mosaïque sans pochette : un dégradé dans la teinte du genre.
pub fn genre_tiles(h: f32) -> [Vec<Background>; 4] {
    std::array::from_fn(|k| {
        let kf = k as f32;
        let hk = (h + kf * 23.) % 360.;
        vec![LinearGradient::new(
            45. + kf * 40.,
            [
                GradientStop { position: 0., color: Color::oklch(0.55 - kf * 0.06, 0.1, hk, 1.) },
                GradientStop { position: 1., color: Color::oklch(0.2, 0.04, hk, 1.) },
            ],
        )
        .into()]
    })
}

/// « 80 » avant 2000, « 2010 » après.
pub fn short_decade(d: i64) -> String {
    if (1920..2000).contains(&d) { (d % 100).to_string() } else { d.to_string() }
}

/// Une teinte par décennie.
pub fn decade_hue(d: i64) -> f32 {
    ((((d - 1950) as f32 / 10.) * 47. + 360.) % 360. + 360.) % 360.
}

impl GenrePage {
    /// Les morceaux de la page, pour son menu « … ».
    pub fn collection_source(&self) -> crate::models::library::CollectionSource {
        use crate::models::library::CollectionSource;
        match self.period {
            _ if self.composer => CollectionSource::Composer(self.name.clone()),
            Some((decade, year)) => year.map_or(CollectionSource::Period(decade, decade + 9), |y| CollectionSource::Period(y, y)),
            None => CollectionSource::Genre(self.name.clone()),
        }
    }

    pub(crate) fn new(name: String, period: Option<(i64, Option<i64>)>) -> Self {
        Self {
            name,
            period,
            composer: false,
            photo: None,
            tracks: Vec::new(),
            loading: false,
            error: false,
            view: None,
            sort_albums: "year",
            sort: if period.is_some() { "year".into() } else { "title".into() },
            asc: period.is_none(),
            window: ListWindow::default(),
        }
    }
}

/// Les albums de la page : ceux du genre, de la période (décennie ou année) ou, pour un
/// compositeur, ceux de ses morceaux ; dans l'ordre choisi.
pub(crate) fn genre_albums<'a>(page: &GenrePage, albums: &'a [AlbumListRow]) -> Vec<&'a AlbumListRow> {
    let g = page.name.to_lowercase();
    let by_title = |x: &str, y: &str| natural_cmp(&sort_key(x), &sort_key(y));
    let ids: std::collections::HashSet<&str> = page.tracks.iter().filter_map(|x| x.album_id.as_deref()).collect();
    let mut l: Vec<&AlbumListRow> = match page.period {
        _ if page.composer => albums.iter().filter(|a| ids.contains(a.id.as_str())).collect(),
        Some((d, y)) => {
            let (a, b) = y.map(|y| (y as i32, y as i32)).unwrap_or((d as i32, d as i32 + 9));
            albums.iter().filter(|x| x.year.is_some_and(|y| y >= a && y <= b)).collect()
        }
        None => albums.iter().filter(|a| a.genre.as_ref().is_some_and(|x| x.to_lowercase() == g)).collect(),
    };
    let period = page.period.is_some();
    match page.sort_albums {
        "title" => l.sort_by(|x, y| by_title(&x.title, &y.title)),
        "artist" => l.sort_by(|x, y| by_title(&x.artist, &y.artist).then(y.year.unwrap_or(0).cmp(&x.year.unwrap_or(0)))),
        _ if period => l.sort_by(|x, y| y.year.unwrap_or(0).cmp(&x.year.unwrap_or(0)).then(by_title(&x.artist, &y.artist)).then(by_title(&x.title, &y.title))),
        _ => l.sort_by(|x, y| y.year.unwrap_or(0).cmp(&x.year.unwrap_or(0)).then(by_title(&x.title, &y.title))),
    }
    l
}

/// Les artistes des morceaux, les plus présents d'abord (identifiant, nombre de titres).
pub(crate) fn artist_counts(tracks: &[TrackListRow]) -> Vec<(String, usize)> {
    let mut count: Vec<(String, usize)> = Vec::new();
    for x in tracks {
        if let Some(id) = &x.artist_id {
            match count.iter_mut().find(|(k, _)| k == id) {
                Some((_, n)) => *n += 1,
                None => count.push((id.clone(), 1)),
            }
        }
    }
    count.sort_by(|a, b| b.1.cmp(&a.1));
    count
}

/// « 1994 », « 1971 – 1999 », sinon rien (aucune année connue).
pub(crate) fn year_span(albums: &[&AlbumListRow]) -> Option<String> {
    let mut years: Vec<i32> = albums.iter().filter_map(|a| a.year).filter(|y| *y > 0).collect();
    years.sort();
    match (years.first(), years.last()) {
        (Some(a), Some(b)) if a == b => Some(a.to_string()),
        (Some(a), Some(b)) => Some(format!("{a} – {b}")),
        _ => None,
    }
}

impl MusicApp {
    pub(crate) fn genre_albums(&self) -> Vec<&AlbumListRow> {
        let Some(page) = self.views.genre.as_ref() else { return Vec::new() };
        genre_albums(page, &self.data.albums)
    }

    /// Les morceaux dans l'ordre affiché.
    pub(crate) fn genre_sorted(&self) -> Vec<TrackListRow> {
        let Some(page) = self.views.genre.as_ref() else { return Vec::new() };
        let mut l = page.tracks.clone();
        sort_tracks(&mut l, &page.sort, page.asc);
        l
    }

    /// Le compositeur s'il est aussi un artiste de la bibliothèque (sa photo et sa page),
    /// et les artistes des morceaux, les plus présents d'abord.
    pub(crate) fn genre_artists(&self) -> (Option<&ArtistListRow>, Vec<&ArtistListRow>) {
        let Some(page) = self.views.genre.as_ref() else { return (None, Vec::new()) };
        let as_artist = page
            .composer
            .then(|| self.data.artists.iter().find(|a| a.name.trim().eq_ignore_ascii_case(page.name.trim())))
            .flatten();
        let artists = artist_counts(&page.tracks)
            .iter()
            .filter(|(id, _)| as_artist.is_none_or(|a| a.id != *id))
            .filter_map(|(id, _)| self.data.artists.iter().find(|a| a.id == *id))
            .collect();
        (as_artist, artists)
    }

    /// L'onglet ouvert : celui choisi, sinon les albums (les morceaux d'un genre sans album).
    pub(crate) fn genre_tab(&self, albums: &[&AlbumListRow]) -> &'static str {
        let Some(page) = self.views.genre.as_ref() else { return "albums" };
        page.view.unwrap_or(if albums.is_empty() && page.period.is_none() { "morceaux" } else { "albums" })
    }

    /// Éléments de la liste de l'onglet ouvert.
    pub(crate) fn genre_list_len(&self) -> usize {
        let Some(page) = self.views.genre.as_ref() else { return 0 };
        let albums = self.genre_albums();
        match self.genre_tab(&albums) {
            "albums" => albums.len(),
            "artistes" => self.genre_artists().1.len(),
            _ => page.tracks.len(),
        }
    }


    pub fn genre_view(&self) -> Option<GenreView> {
        let page = self.views.genre.as_ref()?;
        let grid = self.views.view_mode == ViewMode::Grid;
        let albums = self.genre_albums();
        let year_span = year_span(&albums);
        let (as_artist, artists) = self.genre_artists();
        let view = self.genre_tab(&albums);
        let h = match page.period {
            Some((d, _)) => decade_hue(d),
            None => hue(&page.name),
        };
        // L'en-tête d'une période ne suit pas le tri : les plus récents.
        let mut recent = albums.clone();
        if page.period.is_some() {
            recent.sort_by(|x, y| y.year.unwrap_or(0).cmp(&x.year.unwrap_or(0)));
        }
        let covers: Vec<String> = recent.iter().filter_map(|a| a.cover_url.clone()).take(4).collect();
        let period = page.period.map(|(d, y)| {
            let decade: Vec<&AlbumListRow> = self.data.albums.iter().filter(|a| a.year.is_some_and(|x| x as i64 >= d && x as i64 <= d + 9)).collect();
            PeriodHead {
                big: y.map(|y| y.to_string()).unwrap_or_else(|| short_decade(d)),
                decade: d,
                year: y,
                total: intl::number(decade.len() as i64),
                label: t_with("home.decade_name", &[("d", &short_decade(d))]),
                chips: (d..d + 10)
                    .map(|yy| {
                        let n = decade.iter().filter(|a| a.year == Some(yy as i32)).count();
                        (yy, n, match n {
                            0 => t("years_view.year_empty"),
                            1 => t("years_view.year_album"),
                            n => t_with("years_view.year_albums", &[("n", &intl::number(n as i64))]),
                        })
                    })
                    .collect(),
            }
        });
        let pl = |n: usize, one: &str, many: &str| (intl::number(n as i64), t(if n == 1 { one } else { many }));
        let duration: f64 = page.tracks.iter().map(|x| x.duration.unwrap_or(0.)).sum();
        let tab = |key: &'static str, label: String, n: usize| Choice {
            label: format!("{label} · {}", intl::number(n as i64)),
            icon: String::new(),
            active: view == key,
            msg: Msg::Genre(GenreMsg::View(key)),
            icon_only: false,
        };
        let mut tabs = vec![
            tab("albums", t("library.albums"), albums.len()),
            tab("morceaux", t("library.tracks"), page.tracks.len()),
            tab("artistes", t(if page.composer { "composers_view.performers" } else { "library.artists" }), artists.len()),
        ];
        let mode_counts = vec![intl::number(albums.len() as i64), intl::number(page.tracks.len() as i64), intl::number(artists.len() as i64)];
        if page.composer {
            // ModeSwitch : icône, libellé et nombre à part.
            let icons = ["material-symbols:album-outline-rounded", "material-symbols:music-note-rounded", "material-symbols:mic-external-on-outline-rounded"];
            let labels = [t("library.albums"), t("library.tracks"), t("composers_view.performers")];
            for ((o, i), l) in tabs.iter_mut().zip(icons).zip(labels) {
                o.icon = i.into();
                o.label = l;
            }
        }
        let album_sorts = [("year", "period_view.sort_year"), ("title", "period_view.sort_title"), ("artist", "period_view.sort_artist")]
            .into_iter()
            .map(|(k, l)| Choice { label: t(l), icon: String::new(), active: page.sort_albums == k, msg: Msg::Genre(GenreMsg::SortAlbums(k)), icon_only: false })
            .collect();
        let arrow = |k: &str| (page.sort == k).then_some(if page.asc { "material-symbols:arrow-upward-rounded" } else { "material-symbols:arrow-downward-rounded" });
        let order: [(&str, &str); 4] = if page.composer {
            [("album", "composers_view.sort_album"), ("title", "artist_view.sort_title"), ("artist", "genre_view.sort_artist"), ("year", "artist_view.sort_year")]
        } else if page.period.is_some() {
            [("year", "artist_view.sort_year"), ("title", "artist_view.sort_title"), ("artist", "genre_view.sort_artist"), ("created_at", "genre_view.sort_added")]
        } else {
            [("title", "artist_view.sort_title"), ("artist", "genre_view.sort_artist"), ("year", "artist_view.sort_year"), ("created_at", "genre_view.sort_added")]
        };
        let sorts = order
            .into_iter()
            .map(|(k, l)| SortChoice { key: k, label: t(l), active: page.sort == k, arrow: arrow(k) })
            .collect();

        let (mut tracks, mut table) = (Vec::new(), None);
        if view == "morceaux" {
            let tries = self.genre_sorted();
            let range = page.window.range(tries.len());
            let first = range.start;
            let visible = &tries[range];
            if grid {
                let playing = self.data.player.path.as_deref();
                tracks = visible
                    .iter()
                    .enumerate()
                    .map(|(k, x)| (first + k, x))
                    .map(|(i, x)| AlbumTrack {
                        i,
                        number: (i + 1).to_string(),
                        numbered: true,
                        cover: x.thumbnail_path.clone(),
                        title: x.title.clone(),
                        invite: Some(
                            [x.artist.clone(), x.album.clone(), page.period.and(x.year.as_ref().and_then(|y| y.get(..4)).map(String::from))]
                                .into_iter()
                                .flatten()
                                .filter(|s| !s.is_empty())
                                .collect::<Vec<_>>()
                                .join(" · "),
                        )
                        .filter(|s| !s.is_empty()),
                        duration: x.duration.map(intl::clock).unwrap_or_default(),
                        liked: self.data.liked.contains(&x.path),
                        in_progress: playing == Some(x.path.as_str()),
                        artwork: true,
                        mark: false,
                    })
                    .collect();
            } else {
                table = Some(self.track_table(visible, false, Some(&page.sort), page.asc, false));
            }
        }
        let portrait = as_artist.and_then(|a| a.thumbnail_path.clone()).or_else(|| page.photo.clone());
        let tint = portrait
            .as_ref()
            .or(covers.first())
            .and_then(|c| self.assets.as_ref()?.covers.get(c))
            .map(|img| img.dominant_hue())
            .unwrap_or((150., 0.));
        let title = match page.period {
            Some((_, Some(y))) => y.to_string(),
            Some((d, None)) => t_with("home.decade_name", &[("d", &short_decade(d))]),
            None => page.name.clone(),
        };
        Some(GenreView {
            composer: page.composer,
            avatar: crate::mapper::artists::avatar(&page.name),
            artist_page: as_artist.map(|a| (a.id.clone(), a.name.clone())),
            mode_counts,
            portrait: if page.composer { portrait } else { None },
            kicker: t(match page.period {
                _ if page.composer => "composers_view.kicker",
                Some((_, Some(_))) => "period_view.kicker_year",
                Some(_) => "period_view.kicker_decade",
                None => "genre_view.kicker",
            }),
            back: t(if page.composer {
                "composers_view.back"
            } else if page.period.is_some() {
                "period_view.back"
            } else {
                "genre_view.back"
            }),
            empty: t(if page.composer {
                "composers_view.empty"
            } else if page.period.is_some() {
                "period_view.empty"
            } else {
                "genre_view.empty"
            }),
            artists_desc: t(if page.composer {
                "composers_view.performers_desc"
            } else if page.period.is_some() {
                "period_view.artists_desc"
            } else {
                "genre_view.artists_desc"
            }),
            period,
            name: title,
            name_long: page.period.is_none() && page.name.chars().count() > 24,
            tint,
            color: Color::oklch(0.7, 0.17, h, 1.),
            covers: std::array::from_fn(|k| covers.get(k).cloned()),
            tiles: genre_tiles(h),
            icon: crate::mapper::home::genre_icon(&page.name),
            albums_n: (!albums.is_empty()).then(|| pl(albums.len(), "library_head.albums_one", "library_head.albums_n")),
            tracks_n: pl(page.tracks.len(), "library_head.tracks_one", "library_head.tracks_n"),
            artists_n: (!artists.is_empty()).then(|| {
                if page.composer {
                    pl(artists.len(), "composers_view.performers_one", "composers_view.performers_n")
                } else {
                    pl(artists.len(), "library_head.artists_one", "library_head.artists_n")
                }
            }),
            duration: if duration > 0. { intl::duration(duration) } else { String::new() },
            year_span: if page.composer { None } else { year_span },
            has_tracks: !page.tracks.is_empty(),
            top_artists: if page.period.is_some() { Vec::new() } else { artists.iter().take(3).map(|a| (a.id.clone(), a.name.clone())).collect() },
            tabs,
            view,
            album_sorts,
            sorts,
            albums: if view == "albums" { albums[page.window.range(albums.len())].iter().map(|a| album_item(a)).collect() } else { Vec::new() },
            artists: if view == "artistes" { artists[page.window.range(artists.len())].iter().map(|a| ArtistItem::from_view(a)).collect() } else { Vec::new() },
            tracks,
            table,
            loading: page.loading,
        })
    }

    pub(crate) fn genre_covers(&self) -> Vec<String> {
        let Some(v) = self.genre_view() else { return Vec::new() };
        let small = crate::app::resources::assets::small_key;
        let mut paths: Vec<String> = v.covers.iter().flatten().cloned().collect();
        paths.extend(v.albums.iter().filter_map(|a| a.cover.clone()));
        paths.extend(v.artists.iter().filter_map(|a| a.portrait.clone()));
        paths.extend(v.tracks.iter().filter_map(|x| x.cover.as_deref().map(small)));
        if let Some(tb) = &v.table {
            paths.extend(tb.rows.iter().filter_map(|r| r.cover.as_deref().map(small)));
        }
        paths
    }
}
