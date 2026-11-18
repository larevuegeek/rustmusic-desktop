//! La fiche d'un album : ce qu'affichent ses gabarits, bâti depuis l'état de la page,
//! et les pochettes à charger.

use crate::models::albums::*;
use std::collections::HashMap;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::mapper::albums::hires_label;
use crate::mapper::albums::natural_cmp;
use crate::mapper::albums::sort_key;
use crate::models::albums::AlbumItem;
use crate::models::artists::ArtistItem;
use crate::mapper::tracks::format_bitrate;
use crate::mapper::tracks::tier;
use crate::models::tracks::Tier;
use crate::query::album::album_list_row::AlbumListRow;
use crate::query::track::track_list_row::TrackListRow;

/// Disque, piste, puis nom de fichier dans l'ordre naturel.
pub fn disc_order(x: &TrackListRow, y: &TrackListRow) -> std::cmp::Ordering {
    x.disc_number
        .max(1)
        .cmp(&y.disc_number.max(1))
        .then(x.track_number.unwrap_or(10_000).cmp(&y.track_number.unwrap_or(10_000)))
        .then(natural_cmp(&x.filename, &y.filename))
}

/// Les morceaux par disque (indices dans `tracks`), dans l'ordre d'arrivée. Sans
/// regroupement (`by_disc` faux) ou avec un seul disque, un groupe numéroté 0.
pub(crate) fn disc_groups(tracks: &[TrackListRow], by_disc: bool) -> Vec<(i32, Vec<usize>)> {
    let mut groups: Vec<(i32, Vec<usize>)> = Vec::new();
    for (i, x) in tracks.iter().enumerate() {
        let d = if by_disc { x.disc_number.max(1) } else { 0 };
        match groups.iter_mut().find(|(n, _)| *n == d) {
            Some((_, l)) => l.push(i),
            None => groups.push((d, vec![i])),
        }
    }
    if groups.len() == 1 {
        groups[0].0 = 0;
    }
    groups
}

/// Une colonne, vides en dernier, nombres comparés comme tels.
pub fn sort_tracks(tracks: &mut [TrackListRow], key: &str, asc: bool) {
    let cols = crate::mapper::tracks::resolve(&[key.to_string()]);
    let Some(col) = cols.first() else { return };
    let numeric = col.numeric || key == "duration" || key == "index";
    let read = |t: &TrackListRow| -> Option<String> {
        match key {
            "duration" => t.duration.map(|d| d.to_string()),
            "index" => t.track_number.map(|n| n.to_string()),
            k if k.starts_with("tag:") => crate::mapper::tracks::tag_cell(t, k),
            _ => (col.value)(t),
        }
        .filter(|v| !v.is_empty())
    };
    tracks.sort_by(|a, b| {
        let (va, vb) = (read(a), read(b));
        let order = match (&va, &vb) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, _) => return std::cmp::Ordering::Greater,
            (_, None) => return std::cmp::Ordering::Less,
            (Some(x), Some(y)) if numeric => {
                let (x, y) = (x.parse::<f64>().unwrap_or(0.), y.parse::<f64>().unwrap_or(0.));
                x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Some(x), Some(y)) => natural_cmp(x, y),
        };
        if asc { order } else { order.reverse() }
    });
}

/// La meilleure piste de l'album, son palier et sa fiche.
pub fn best_quality(tracks: &[TrackListRow]) -> Option<(Tier, String)> {
    let rank = |x: &TrackListRow| {
        (if x.bits_per_sample == Some(1) { 1e9 } else { x.bits_per_sample.unwrap_or(0) as f64 * 1e6 })
            + x.sample_rate.unwrap_or(0) as f64
            + x.bitrate.unwrap_or(0) as f64 / 1e4
    };
    let m = tracks.iter().max_by(|a, b| rank(a).partial_cmp(&rank(b)).unwrap_or(std::cmp::Ordering::Equal))?;
    let f = m.audio_format.clone().unwrap_or_else(|| m.extension.clone()).to_uppercase();
    let tier = tier(m.audio_format.as_deref(), m.bits_per_sample, m.sample_rate);
    if m.bits_per_sample == Some(1) || f == "DSF" || f == "DFF" {
        return Some((tier, crate::helper::audio::dsd_label(m.sample_rate.map_or(0, |r| r.max(0) as u32))));
    }
    if tier != Tier::Lossy {
        if let (Some(b), Some(r)) = (m.bits_per_sample, m.sample_rate) {
            if b > 0 && r > 0 {
                let khz = if r % 1000 != 0 { format!("{:.1}", r as f64 / 1000.).replace('.', ",") } else { (r / 1000).to_string() };
                return Some((tier, format!("{f} {b} bit / {khz} kHz")));
            }
        }
    }
    Some((tier, if m.bitrate.unwrap_or(0) > 0 { format!("{f} {}", format_bitrate(m.bitrate)) } else { f }))
}

/// Date complète et label, lus dans les tags de la première piste qui les porte.
pub(crate) fn details(tracks: &[TrackListRow]) -> (Option<String>, Option<String>) {
    let (mut date, mut label): (Option<String>, Option<String>) = (None, None);
    let complete = |v: &str| v.len() >= 10 && v.as_bytes()[4] == b'-' && v.as_bytes()[7] == b'-' && v[..4].bytes().all(|c| c.is_ascii_digit());
    for x in tracks {
        let Some(json) = x.tags.as_deref() else { continue };
        let Ok(fields) = serde_json::from_str::<serde_json::Value>(json) else { continue };
        let custom: HashMap<String, String> = fields
            .get("custom_tags")
            .and_then(|v| v.as_array())
            .map(|l| {
                l.iter()
                    .filter_map(|p| Some((p.get(0)?.as_str()?.to_uppercase(), p.get(1)?.as_str()?.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        let read = |k: &str| custom.get(k).filter(|v| !v.is_empty()).cloned();
        if label.is_none() {
            label = fields
                .get("publisher")
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty())
                .map(String::from)
                .or_else(|| read("LABEL"))
                .or_else(|| read("ORGANIZATION"))
                .or_else(|| read("PUBLISHER"));
        }
        if date.is_none() {
            let year = fields.get("year").and_then(|v| v.as_str()).map(String::from);
            date = [read("RELEASEDATE"), read("DATE"), year, read("ORIGINALDATE")]
                .into_iter()
                .flatten()
                .find(|v| complete(v));
        }
        if label.is_some() && date.is_some() {
            break;
        }
    }
    (date, label)
}

/// « 12 mars 1994 » (date longue, dans la langue de l'utilisateur).
pub(crate) fn long_date(date: &str) -> Option<String> {
    let y: i32 = date.get(..4)?.parse().ok()?;
    let m: u32 = date.get(5..7)?.parse().ok().filter(|m| (1..=12).contains(m))?;
    let d: u32 = date.get(8..10)?.parse().ok()?;
    Some(crate::helper::date::long_date(d, m, y))
}

/// Dossier commun des fichiers (un album en CD1/CD2 remonte d'un cran).
pub(crate) fn common_dir(tracks: &[TrackListRow]) -> Option<String> {
    let parent = |p: &str| p.rfind(['\\', '/']).map(|i| p[..i].to_string()).unwrap_or_default();
    let dirs: Vec<String> = tracks.iter().map(|x| parent(&x.path)).collect();
    let mut p = dirs.first()?.clone();
    for d in &dirs {
        while !p.is_empty() && *d != p && !d.starts_with(&format!("{p}\\")) && !d.starts_with(&format!("{p}/")) {
            let q = parent(&p);
            p = if q == p { String::new() } else { q };
        }
    }
    (!p.is_empty()).then_some(p)
}

pub fn album_item(a: &AlbumListRow) -> AlbumItem {
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
}

impl crate::models::albums::AlbumItem {
    /// Ancre du menu « … » de la carte.
    pub fn more_id(&self) -> String {
        format!("album.more.{}", self.id)
    }
}

impl crate::models::albums::AlbumTrack {
    /// Ancre du menu « … » de la ligne.
    pub fn more_id(&self) -> String {
        format!("album.more.{}", self.i)
    }
}

impl MusicApp {
    /// Un carrousel avec l'état de sa zone (flèches, glissement demandé).
    pub fn carousel<T>(&self, id: &'static str, title: String, subtitle: String, items: Vec<T>) -> Carousel<T> {
        let state = self.views.carousels.get(id).copied().unwrap_or_default();
        Carousel {
            id,
            title,
            subtitle,
            total: items.len(),
            items,
            view: self.views.carousel_view,
            back: state.can_go_back(),
            forward: state.can_go_forward(),
            request: state.request,
            by_year: None,
            by_year_msg: None,
        }
    }

    /// Les titres dans l'ordre affiché.
    pub(crate) fn album_sorted(&self) -> Vec<TrackListRow> {
        let Some(page) = self.views.album.as_ref() else { return Vec::new() };
        let mut tracks = page.tracks.clone();
        if !tracks.iter().all(|x| x.track_number.is_some()) {
            tracks.sort_by(disc_order);
        }
        if let Some(key) = page.sort.as_deref() {
            sort_tracks(&mut tracks, key, page.asc);
        }
        tracks
    }

    pub fn album_view(&self) -> Option<AlbumView> {
        let page = self.views.album.as_ref()?;
        let album = page.album.as_ref()?;
        let grid = self.views.view_mode == crate::models::albums::ViewMode::Grid;
        let sorted = self.album_sorted();
        let tracks = &page.tracks;
        let duration_s = Some(tracks.iter().map(|x| x.duration.unwrap_or(0.)).sum::<f64>()).filter(|d| *d > 0.).unwrap_or(album.total_duration);
        let weight: i64 = tracks.iter().map(|x| x.file_size.unwrap_or(x.size)).sum();
        let nb = if tracks.is_empty() { album.total_tracks } else { tracks.len() as i64 };
        let count_label = t(if nb == 1 { "library_head.tracks_one" } else { "library_head.tracks_n" });
        let mut genres: Vec<String> = Vec::new();
        for x in tracks {
            if let Some(g) = x.genre.as_ref().map(|g| g.trim()).filter(|g| !g.is_empty()) {
                if !genres.iter().any(|v| v == g) {
                    genres.push(g.to_string());
                }
            }
        }
        if genres.is_empty() {
            genres.extend(album.genre.clone());
        }
        genres.truncate(4);

        let playing = self.data.player.path.as_deref();
        let item = |i: usize, x: &TrackListRow, position: Option<usize>| AlbumTrack {
            i,
            number: x
                .track_number
                .map(|n| n.to_string())
                .or_else(|| position.map(|p| p.to_string()))
                .unwrap_or_else(|| "—".into()),
            numbered: x.track_number.is_some(),
            cover: x.thumbnail_path.clone(),
            title: x.title.clone(),
            invite: x.artist.clone().filter(|a| *a != album.artist),
            duration: x.duration.map(intl::clock).unwrap_or_default(),
            liked: self.data.liked.contains(&x.path),
            in_progress: playing == Some(x.path.as_str()),
            artwork: !self.views.without_artwork,
            mark: false,
        };
        // Disque par disque dans l'ordre du disque ; deux colonnes lues de haut en bas.
        let by_disc = page.sort.is_none();
        let discs = disc_groups(&sorted, by_disc)
            .into_iter()
            .map(|(n, indices)| {
                let mut l: Vec<AlbumTrack> = indices.into_iter().map(|i| item(i, &sorted[i], by_disc.then_some(i + 1))).collect();
                let right = l.split_off(l.len().div_ceil(2));
                Disc { n, label: t_with("album_view.disc", &[("n", &n.to_string())]), left: l, right }
            })
            .collect();
        let arrow = |k: &str| {
            (page.sort.as_deref() == Some(k)).then_some(if page.asc {
                "material-symbols:arrow-upward-rounded"
            } else {
                "material-symbols:arrow-downward-rounded"
            })
        };
        let sorts = vec![
            SortChoice { key: "n", label: t("album_view.sort_number"), active: page.sort.is_none(), arrow: None },
            SortChoice { key: "title", label: t("album_view.sort_title"), active: page.sort.as_deref() == Some("title"), arrow: arrow("title") },
            SortChoice { key: "duration", label: t("album_view.sort_duration"), active: page.sort.as_deref() == Some("duration"), arrow: arrow("duration") },
        ];
        let table = (!grid).then(|| self.track_table(&sorted, false, page.sort.as_deref(), page.asc, page.sort.is_none()));

        let (date, label) = details(tracks);
        let output = date
            .as_deref()
            .and_then(long_date)
            .map(|d| t_with("album_view.released_on", &[("d", &d)]))
            .or_else(|| album.year.map(|y| t_with("album_view.released_in", &[("d", &y.to_string())])));
        let remote = tracks.first().is_some_and(|x| x.path.starts_with("http") || x.path.starts_with("smb:"));
        let folder = common_dir(tracks).filter(|_| !remote);

        // Autour de l'album.
        let all: Vec<&AlbumListRow> = self.data.albums.iter().collect();
        let by_title = |x: &&AlbumListRow, y: &&AlbumListRow| natural_cmp(&sort_key(&x.title), &sort_key(&y.title));
        let others = (!album.artist_id.is_empty()).then(|| {
            let mut l: Vec<&AlbumListRow> = all.iter().copied().filter(|x| x.artist_id.as_deref() == Some(album.artist_id.as_str()) && x.id != album.id).collect();
            if page.others_by_year {
                l.sort_by(|x, y| y.year.unwrap_or(0).cmp(&x.year.unwrap_or(0)).then(by_title(x, y)));
            } else {
                l.sort_by(by_title);
            }
            l
        });
        let others = others.filter(|l| !l.is_empty()).map(|l| {
            let mut c = self.carousel(
                "autres",
                if album.artist.is_empty() { t("album_view.other_albums_same") } else { t_with("album_view.other_albums", &[("a", &album.artist)]) },
                format!("{} {}", intl::number(l.len() as i64), t(if l.len() == 1 { "library_head.albums_one" } else { "library_head.albums_n" })),
                l.into_iter().map(album_item).collect(),
            );
            c.by_year = Some(page.others_by_year);
            c.by_year_msg = Some((Msg::Album(AlbumMsg::OthersByYear(true)), Msg::Album(AlbumMsg::OthersByYear(false))));
            c
        });
        let same_genre = t_with("album_view.same_genre", &[("g", album.genre.as_deref().unwrap_or(""))]);
        let similar: Vec<AlbumItem> = page
            .similar
            .iter()
            .filter_map(|id| self.data.albums.iter().find(|a| a.id == *id))
            .map(album_item)
            .collect();
        let similar = (!similar.is_empty()).then(|| self.carousel("similaires", t("album_view.similar_albums"), same_genre.clone(), similar));
        let artists: Vec<ArtistItem> = page
            .similar_artists
            .iter()
            .filter_map(|id| self.data.artists.iter().find(|a| a.id == *id))
            .map(ArtistItem::from_view)
            .collect();
        let artists = (!artists.is_empty()).then(|| self.carousel("artistes", t("album_view.similar_artists"), same_genre, artists));

        // Précédent / suivant : l'ordre alphabétique de la vue Albums.
        let mut order = all.clone();
        order.sort_by(by_title);
        let rank = order.iter().position(|a| a.id == album.id);
        let prev = rank.filter(|r| *r > 0).map(|r| Route::Album(order[r - 1].id.clone()));
        let next = rank.and_then(|r| order.get(r + 1)).map(|a| Route::Album(a.id.clone()));

        let tint = album
            .cover_url
            .as_deref()
            .and_then(|c| self.assets.as_ref()?.covers.get(c))
            .map(|img| img.dominant_hue())
            .unwrap_or((150., 0.));
        let type_key = if ["single", "ep", "compilation"].contains(&album.album_type.as_str()) { album.album_type.as_str() } else { "album" };
        Some(AlbumView {
            cover: album.cover_url.clone(),
            tint,
            type_label: t(&format!("album_view.type_{type_key}")),
            quality: best_quality(tracks),
            title: album.title.clone(),
            title_long: album.title.chars().count() > 40,
            artist: if album.artist.is_empty() { t("album_view.unknown_artist") } else { album.artist.clone() },
            artist_route: (!album.artist_id.is_empty()).then(|| Route::Artist(album.artist_id.clone())),
            artist_portrait: self.data.artists.iter().find(|a| a.id == album.artist_id).and_then(|a| a.thumbnail_path.clone()),
            year: album.year,
            tracks_count: format!("{} {}", intl::number(nb), count_label),
            duration: if duration_s > 0. { intl::duration(duration_s) } else { String::new() },
            weight: if weight > 0 { karbon_ui::intl::bytes(weight as u64) } else { String::new() },
            count_n: intl::number(nb),
            count_label,
            liked_all: !tracks.is_empty() && tracks.iter().all(|x| self.data.liked.contains(&x.path)),
            genres,
            sorts,
            multi: tracks.len() > 1,
            discs,
            table,
            output,
            label,
            folder,
            reveal: tracks.first().map(|x| x.path.clone()).unwrap_or_default(),
            notes: album.notes.clone().filter(|n| !n.trim().is_empty()),
            others,
            similar,
            artists,
            prev,
            next,
        })
    }

    /// Les pochettes et portraits que la page montre.
    pub(crate) fn album_covers(&self) -> Vec<String> {
        let Some(v) = self.album_view() else { return Vec::new() };
        let mut paths: Vec<String> = v.cover.into_iter().chain(v.artist_portrait).collect();
        for d in &v.discs {
            paths.extend(d.left.iter().chain(&d.right).filter_map(|x| x.cover.as_deref().map(crate::app::resources::assets::small_key)));
        }
        if let Some(table) = &v.table {
            paths.extend(table.rows.iter().filter_map(|r| r.cover.as_deref().map(crate::app::resources::assets::small_key)));
        }
        for c in [&v.others, &v.similar].into_iter().flatten() {
            paths.extend(c.items.iter().filter_map(|a| a.cover.clone()));
        }
        if let Some(c) = &v.artists {
            paths.extend(c.items.iter().filter_map(|a| a.portrait.clone()));
        }
        paths
    }
}
