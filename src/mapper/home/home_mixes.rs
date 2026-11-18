//! Les mix de l'accueil et leur aperçu.

use crate::models::home::*;
use std::collections::HashMap;
use chrono::Utc;
use crate::models::app::Data;
use karbon_ui::intl;
use crate::helper::string::string::hue;
use crate::query::track::track_list_row::TrackListRow;

/// Les mix « Pour vous » et leur aperçu.
pub(crate) fn mixes(data: &Data, now: chrono::DateTime<Utc>) -> Vec<Mix> {
    if data.mix_genres.is_empty() || data.library_id.is_none() {
        return Vec::new();
    }
    let rotating: Vec<_> = data.mix_genres.iter().take(6).collect();
    let day = (now.timestamp() / 86_400) as usize;
    let chosen_items: Vec<_> = if rotating.len() <= 2 {
        rotating.clone()
    } else {
        vec![rotating[day % rotating.len()], rotating[(day + 1) % rotating.len()]]
    };
    let mut specs: Vec<Mix> = chosen_items
        .iter()
        .map(|g| Mix {
            key: format!("genre:{}", g.name.to_lowercase()),
            kind: intl::t("home.mix_of_day"),
            name: g.name.clone(),
            help: intl::t_with("home.mix_genre_help", &[("n", &g.artists.to_string())]),
            h: hue(&g.name.to_lowercase()),
            covers_list: Vec::new(),
            description: String::new(),
            meta: String::new(),
            ready: false,
            source_kind: "genre",
            source_genre: Some(g.name.clone()),
        })
        .collect();
    specs.push(Mix {
        key: "oublies".into(),
        kind: intl::t("home.rediscover"),
        name: intl::t("home.forgotten"),
        help: intl::t("home.forgotten_help"),
        h: 200.,
        covers_list: Vec::new(),
        description: String::new(),
        meta: String::new(),
        ready: false,
        source_kind: "oublies",
        source_genre: None,
    });
    if data.stats.as_ref().is_some_and(|s| s.quality_hires > 0) {
        specs.push(Mix {
            key: "hires".into(),
            kind: intl::t("home.studio_quality"),
            name: "Hi-Res".into(),
            help: intl::t("home.hires_help"),
            h: 150.,
            covers_list: Vec::new(),
            description: String::new(),
            meta: String::new(),
            ready: false,
            source_kind: "hires",
            source_genre: None,
        });
    }
    for mix in &mut specs {
        if let Some(draw) = data.mixes.get(&mix.key).filter(|d| d.loaded) {
            let (covers_list, description, meta) = preview(&draw.tracks);
            mix.covers_list = covers_list;
            mix.description = description;
            mix.meta = meta;
            mix.ready = true;
        }
    }
    specs
}

/// `preview` — quatre pochettes, les artistes de tête, « N titres · durée ».
pub fn preview(tracks: &[TrackListRow]) -> (Vec<String>, String, String) {
    let mut covers_list = Vec::new();
    for p in tracks.iter().filter_map(|t| t.thumbnail_path.clone()) {
        if !covers_list.contains(&p) {
            covers_list.push(p);
            if covers_list.len() == 4 {
                break;
            }
        }
    }
    let mut by_artist: HashMap<&str, usize> = HashMap::new();
    for t in tracks {
        if let Some(a) = t.artist.as_deref() {
            *by_artist.entry(a).or_default() += 1;
        }
    }
    let mut head: Vec<(&str, usize)> = by_artist.into_iter().collect();
    head.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let names = head.iter().take(3).map(|(a, _)| *a).collect::<Vec<_>>().join(", ");
    let description = if head.len() > 3 { intl::t_with("home.mix_artists_more", &[("a", &names)]) } else { names };
    let seconds: f64 = tracks.iter().map(|t| t.duration.unwrap_or(0.)).sum();
    let meta = intl::t_with("home.mix_meta", &[("n", &tracks.len().to_string()), ("d", &intl::duration(seconds))]);
    (covers_list, description, meta)
}
