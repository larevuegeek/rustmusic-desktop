//! L'accueil : salutation, chiffres, reprise de l'écoute, à suivre, mix, récents.

use crate::models::home::*;
use crate::mapper::player::outputs::outputs;
use crate::mapper::search::search_autocomplete::results;
use std::collections::HashMap;
use chrono::Utc;
use crate::models::app::Data;
use crate::helper::string::string::display_title;
use karbon_ui::intl;
use crate::helper::date::elapsed_seconds;
use karbon_ui::intl::t_count;
use crate::models::app::Route;
use crate::models::app::Section;
use super::*;

/// « A; B » → « A, B ».
pub fn readable_artists(value: Option<&str>) -> Option<String> {
    let mut seen = Vec::new();
    for name in value.unwrap_or("").split(';').map(str::trim).filter(|n| !n.is_empty()) {
        if !seen.contains(&name) {
            seen.push(name);
        }
    }
    (!seen.is_empty()).then(|| seen.join(", "))
}

/// « Bonjour » / « Bonsoir » selon l'heure locale, et le prénom.
pub(crate) fn greeting(data: &Data, now: chrono::DateTime<Utc>) -> (String, String) {
    let hour = now.with_timezone(&chrono::Local).format("%H").to_string().parse::<u32>().unwrap_or(12);
    let g = intl::t(if (5..18).contains(&hour) { "home.greeting_morning" } else { "home.greeting_evening" });
    let first_name = data.profile.as_ref().map(|p| p.name.clone()).unwrap_or_default();
    (g, first_name)
}

pub(crate) fn stats(data: &Data) -> Vec<Stat> {
    let Some(s) = &data.stats else {
        return Vec::new();
    };
    vec![
        Stat { n: intl::number(s.total_tracks), label: "titres", route: Route::Library(Section::Tracks) },
        Stat { n: intl::number(s.total_albums), label: "albums", route: Route::Library(Section::Albums) },
        Stat { n: intl::number(s.total_artists), label: "artistes", route: Route::Library(Section::Artists) },
    ]
}

pub(crate) fn recently_added(data: &Data) -> Vec<AlbumCard> {
    let mut albums: Vec<_> = data.albums.iter().collect();
    albums.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    albums
        .into_iter()
        .take(6)
        .map(|a| {
            let year = a.year.map(|y| y.to_string()).unwrap_or_default();
            AlbumCard {
                id: a.id.clone(),
                title: a.title.clone(),
                artist_name: a.artist.clone(),
                subtitle: if year.is_empty() { a.artist.clone() } else { format!("{} · {}", a.artist, year) },
                year,
                cover_path: a.cover_url.clone(),
            }
        })
        .collect()
}

/// La vedette (le dernier écouté hors hero) et la bande des albums d'avant.
pub(crate) fn recent(data: &Data, details: &Record, now: chrono::DateTime<Utc>) -> (Vec<Featured>, Vec<AlbumStrip>) {
    let excluded = if data.player.path.is_some() {
        details.path.as_str()
    } else {
        data.recent.first().map(|r| r.path.as_str()).unwrap_or("")
    };
    let history: Vec<_> = data.recent.iter().filter(|r| r.path != excluded).collect();
    let Some(v) = history.first() else {
        return (Vec::new(), Vec::new());
    };
    let featured = Featured {
        path: v.path.clone(),
        title: display_title(v.title.as_deref(), &v.path, &karbon_ui::intl::t("common.unknown_title")),
        artist_name: readable_artists(v.artist.as_deref()).unwrap_or_default(),
        album: v.album.clone().unwrap_or_default(),
        listen: elapsed_seconds(&v.last_played_at.to_rfc3339(), now).map(intl::relative_time).unwrap_or_default(),
        cover_path: v.thumbnail_path.clone(),
        liked: data.liked.contains(&v.path),
    };
    // groupByAlbum : l'artiste de l'album sert de clé ; un morceau sans album forme son groupe.
    let mut groups: Vec<AlbumStrip> = Vec::new();
    let mut counts: HashMap<String, i64> = HashMap::new();
    for r in &history {
        let artist_name = readable_artists(r.album_artist.as_deref())
            .or_else(|| readable_artists(r.artist.as_deref()));
        let key = match &r.album {
            Some(album) => format!(
                "{}|{}",
                album.trim().to_lowercase(),
                artist_name.as_deref().unwrap_or("").trim().to_lowercase()
            ),
            None => format!("piste:{}", r.path),
        };
        *counts.entry(key.clone()).or_default() += 1;
        if let Some(g) = groups.iter_mut().find(|g| g.key == key) {
            if g.cover_path.is_none() {
                g.cover_path = r.thumbnail_path.clone();
            }
            continue;
        }
        let artist_name = artist_name.unwrap_or_default();
        let listen = elapsed_seconds(&r.last_played_at.to_rfc3339(), now).map(intl::relative_time).unwrap_or_default();
        groups.push(AlbumStrip {
            key,
            title: r.album.clone().or_else(|| r.title.clone()).unwrap_or_default(),
            subtitle: if artist_name.is_empty() { listen.clone() } else { format!("{artist_name} · {listen}") },
            artist_name,
            listen,
            tracks_list: String::new(),
            cover_path: r.thumbnail_path.clone(),
            path: r.path.clone(),
        });
    }
    for g in &mut groups {
        g.tracks_list = t_count(*counts.get(&g.key).unwrap_or(&1), "home.track_one", "home.tracks_n");
    }
    let strip = groups.into_iter().skip(1).take(5).collect();
    (vec![featured], strip)
}

/// « Scannée il y a 4 j », « Jamais scannée », « Scan en cours 42 % ».
pub(crate) fn scan(data: &Data, now: chrono::DateTime<Utc>) -> String {
    if let Some(s) = &data.scan {
        return if s.total > 0 {
            intl::t_with("sidebar.scanning_percent", &[("n", &(s.current * 100 / s.total).min(100).to_string())])
        } else {
            intl::t("library_head.status_scanning")
        };
    }
    match &data.last_scan {
        Some(d) => intl::t_with("sidebar.scanned", &[("ago", &intl::relative_time_short(elapsed_seconds(d, now).unwrap_or(0)))]),
        None => intl::t("sidebar.never_scanned"),
    }
}

impl Launch {
    /// Le message qui lance cette carte.
    pub fn msg(&self) -> crate::models::app::Msg {
        use crate::models::app::Msg;
        match self {
            Launch::Queue(i) => Msg::PlayQueue(*i),
            Launch::Recent(p) => Msg::PlayRecent(p.clone()),
            Launch::Random(p) => Msg::PlayRandom(p.clone()),
        }
    }
}

impl ViewModel {
    pub fn build(data: &Data, route: &Route, source: usize, now: chrono::DateTime<Utc>) -> Self {
        let details = details(data);
        let hero = details.clone().into_iter().collect::<Vec<_>>();
        let details = details.unwrap_or_default();
        let hero_loaded = data.player.path.is_some() && data.player.path.as_deref() == Some(details.path.as_str());
        let hero_position = if hero_loaded {
            data.player.position
        } else if data.flag_default_on("resume_playback") {
            data.recent.first().map(|r| r.last_position).unwrap_or(0.)
        } else {
            0.
        };
        let liked = data.player.path.as_ref().is_some_and(|p| data.liked.contains(p));
        let statut = statut(data);
        let up_next = up_next(data, &details);
        let (greeting, first_name) = greeting(data, now);
        let initial = first_name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
        let profile_color = data.profile.as_ref().map(|p| p.color.clone()).unwrap_or_else(|| "#22c55e".into());
        let stats = stats(data);
        let mixes = mixes(data, now);
        let home_playlists = home_playlists(data);
        let sidebar_playlists = sidebar_playlists(data, route);
        let genres = genres(data);
        let recently_added = recently_added(data);
        let (featured, strip) = recent(data, &details, now);
        let (outputs, output) = outputs(data);
        let sources = sources(data);
        let libraries = libraries(data, sources.get(source).and_then(|s| s.id));
        let library = libraries
            .iter()
            .position(|b| Some(b.id) == data.library_id)
            .unwrap_or(0);
        let current_source = sources.get(source).or(sources.first()).cloned().expect("au moins une source");
        let current_library: Vec<Library> =
            libraries.iter().find(|b| Some(b.id) == data.library_id).cloned().into_iter().collect();
        let library_summary = current_library
            .first()
            .map(|b| {
                let count = intl::t_with("sidebar.libraries_n", &[("n", &libraries.len().to_string())]);
                format!("{} · {count}", b.tracks_count)
            })
            .unwrap_or_default();
        let output_name = outputs.get(output).map(|s| s.label.clone());
        let output_label = output_name.clone().unwrap_or_else(|| intl::t("player.output_default"));
        let output_title = intl::t_with(
            "player.output_title",
            &[("name", &output_name.unwrap_or_else(|| intl::t("player.output_default_short")))],
        );
        let results = results(data);
        let (tiles, tabs) = tiles(data, route);
        let scan = scan(data, now);
        let podcast_covers = data
            .podcasts
            .iter()
            .filter_map(|p| p.cover_path.clone())
            .take(4)
            .collect();
        let new_items: i64 = data.podcasts.iter().map(|p| p.new_count).sum();
        let podcasts_detail = if data.podcasts.is_empty() {
            intl::t("sidebar.podcasts_none")
        } else if new_items == 1 {
            format!("1 {}", intl::t("podcasts.new_one"))
        } else {
            format!("{} {}", intl::number(new_items), intl::t("podcasts.new_n"))
        };
        let radios_detail = match data.radios.len() {
            0 => intl::t("sidebar.radios_none"),
            1 => format!("1 {}", intl::t("radios.favorites_one")),
            n => format!("{} {}", intl::number(n as i64), intl::t("radios.favorites_n")),
        };
        let radio_logo = data
            .radios
            .iter()
            .max_by(|a, b| a.last_played_at.cmp(&b.last_played_at))
            .and_then(|r| r.logo_path.clone());
        let home_empty = data.libraries.is_empty()
            || data.library().is_some_and(|l| l.total_tracks == 0);
        let show_recent = data.flag_default_on("show_recent_in_home") && !data.recent.is_empty();
        Self {
            hero,
            hero_loaded,
            hero_position,
            hero_position_label: intl::clock_padded(hero_position),
            position_label: intl::clock(data.player.position),
            duration_label: intl::clock(details.duration),
            liked,
            details,
            statut,
            up_next,
            greeting_line: format!("{greeting}, {first_name}"),
            greeting,
            first_name,
            initial,
            profile_color,
            stats,
            mixes,
            home_playlists,
            sidebar_playlists,
            genres,
            recently_added,
            featured,
            strip,
            outputs,
            output,
            output_label,
            output_title,
            volume_label: format!("{} %", data.volume),
            volume_title: format!("{} : {} %", intl::t("common.volume"), data.volume),
            many_sources: sources.len() > 1,
            sources,
            current_source,
            libraries,
            library,
            current_library,
            library_summary,
            results,
            tiles,
            tabs,
            scan,
            podcast_covers,
            podcasts_detail,
            radios_detail,
            radio_logo,
            exclusive: data.flag("wasapi_exclusive"),
            dop: data.flag("dsd_dop"),
            network: [
                data.flag("network_players"),
                data.flag("airplay"),
                data.flag("dlna_enabled"),
            ],
            open_buttons: data.flag("show_open_buttons"),
            shortcuts: [
                data.flag_default_on("show_favorites") && data.flag_default_on("show_liked_in_playlists"),
                data.flag_default_on("show_recent_in_playlists"),
            ],
            home_empty,
            home_loading: data.library_loading,
            show_recent,
        }
    }

    /// Toutes les pochettes que la vue peut montrer, pour les demander avant le dessin.
    pub fn covers(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        fn opt(p: &Option<String>) -> Option<&str> {
            p.as_deref().filter(|p| !p.is_empty())
        }
        out.extend(opt(&self.details.cover_path));
        for c in &self.up_next {
            out.extend(opt(&c.cover_path));
        }
        for a in &self.recently_added {
            out.extend(opt(&a.cover_path));
        }
        for v in &self.featured {
            out.extend(opt(&v.cover_path));
        }
        for b in &self.strip {
            out.extend(opt(&b.cover_path));
        }
        for r in &self.results {
            out.extend(opt(&r.cover_path));
        }
        out.extend(opt(&self.radio_logo));
        for p in self.home_playlists.iter().chain(&self.sidebar_playlists) {
            out.extend(p.covers.iter().take(4).map(String::as_str));
        }
        for m in &self.mixes {
            out.extend(m.covers_list.iter().map(String::as_str));
        }
        out.extend(self.podcast_covers.iter().map(String::as_str));
        out
    }
}
