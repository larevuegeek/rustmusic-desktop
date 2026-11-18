//! La reprise de l'écoute : le morceau en cours ou le dernier écouté, son état, la suite.

use crate::models::home::*;
use crate::mapper::player::outputs::is_dsd;
use std::collections::HashSet;
use crate::models::app::Data;
use crate::helper::audio::bitrate;
use crate::helper::string::string::display_title;
use crate::helper::audio::khz;
use crate::helper::files::path::split_path;
use crate::helper::date::date_to_year;
use super::*;

/// Ce qui joue, sinon le dernier morceau écouté.
pub(crate) fn details(data: &Data) -> Option<Record> {
    if let (Some(path), Some(audio)) = (&data.player.path, &data.player.audio) {
        let tags = &audio.tags;
        let track = data.current_track().filter(|t| &t.path == path);
        let cover_path = track
            .and_then(|t| t.cover.clone())
            .or_else(|| data.recent.iter().find(|r| &r.path == path).and_then(|r| r.thumbnail_path.clone()));
        return Some(Record {
            path: path.clone(),
            title: display_title(tags.title.as_deref(), path, &karbon_ui::intl::t("common.unknown_title")),
            artist_name: readable_artists(tags.artist.as_deref()).unwrap_or_default(),
            album: tags.album.clone().unwrap_or_default(),
            year: tags.year.as_deref().map(date_to_year).unwrap_or_default(),
            duration: if data.player.duration > 0. { data.player.duration } else { f64::from(audio.duration) },
            duration_label: String::new(),
            cover_path,
        })
        .map(with_duration_label);
    }
    let last = data.recent.first()?;
    Some(Record {
        path: last.path.clone(),
        title: display_title(last.title.as_deref(), &last.path, &karbon_ui::intl::t("common.unknown_title")),
        artist_name: readable_artists(last.artist.as_deref()).unwrap_or_default(),
        album: last.album.clone().unwrap_or_default(),
        year: last.year.as_deref().map(date_to_year).unwrap_or_default(),
        duration: last.duration.unwrap_or(0.),
        duration_label: String::new(),
        cover_path: last.thumbnail_path.clone(),
    })
    .map(with_duration_label)
}

/// Durée affichée par le hero, d'après `duration`.
fn with_duration_label(mut record: Record) -> Record {
    record.duration_label = karbon_ui::intl::clock_padded(record.duration);
    record
}

/// La ligne d'état.
pub(crate) fn statut(data: &Data) -> Statut {
    let Some(audio) = &data.player.audio else {
        return Statut::default();
    };
    let dsd = is_dsd(&audio.audio_format);
    let format = if dsd {
        crate::helper::audio::dsd_label(audio.sample_rate)
    } else {
        format!("{:?}", audio.audio_format).to_uppercase()
    };
    let frequency = if audio.sample_rate == 0 {
        None
    } else if dsd {
        Some(format!("{:.1} MHz", audio.sample_rate as f64 / 1_000_000.).replace('.', ","))
    } else {
        Some(khz(audio.sample_rate))
    };
    let bits = (audio.bits_per_sample > 0 && !dsd).then(|| format!("{} bit", audio.bits_per_sample));
    let quality = [bits, frequency].into_iter().flatten().collect::<Vec<_>>().join(" · ");
    let output = data
        .player
        .pipeline
        .as_ref()
        .and_then(|p| p.get("output_sample_rate").or_else(|| p.get("output_rate")))
        .and_then(|v| v.as_u64())
        .filter(|rate| *rate as u32 != audio.sample_rate && *rate > 0)
        .map(|rate| format!("→ {}", khz(rate as u32)))
        .unwrap_or_default();
    let bitrate = [
        (audio.bitrate > 0).then(|| bitrate(audio.bitrate)),
        (audio.file_size > 0).then(|| karbon_ui::intl::bytes(audio.file_size as u64)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    let (folder, file) = split_path(&audio.path);
    Statut {
        format,
        quality,
        output,
        bitrate,
        folder,
        file,
    }
}

pub(crate) const UP_NEXT_TOTAL: usize = 4;

/// La file, puis les derniers écoutés, puis du hasard.
pub(crate) fn up_next(data: &Data, details: &Record) -> Vec<Card> {
    let mut cards: Vec<Card> = data
        .upcoming()
        .filter(|(_, t)| !t.title.trim().is_empty() && t.title != "Inconnu")
        .take(UP_NEXT_TOTAL)
        .map(|(index, t)| Card {
            key: format!("f:{}", t.queue_id),
            title: t.title.clone(),
            artist_name: readable_artists(t.artist.as_deref()).unwrap_or_default(),
            duration: t.duration.unwrap_or(0.),
            duration_label: karbon_ui::intl::clock_padded(t.duration.unwrap_or(0.)),
            cover_path: t.cover.clone(),
            launch: Launch::Queue(index),
        })
        .collect();
    let mut seen: HashSet<&str> = HashSet::new();
    seen.insert(details.path.as_str());
    for (_, t) in data.upcoming().take(UP_NEXT_TOTAL) {
        seen.insert(t.path.as_str());
    }
    for r in data.recent.iter().filter(|r| !seen.contains(r.path.as_str())) {
        if cards.len() >= UP_NEXT_TOTAL {
            break;
        }
        cards.push(Card {
            key: format!("r:{}", r.path),
            title: display_title(r.title.as_deref(), &r.path, &karbon_ui::intl::t("common.unknown_title")),
            artist_name: readable_artists(r.artist.as_deref()).unwrap_or_default(),
            duration: r.duration.unwrap_or(0.),
            duration_label: karbon_ui::intl::clock_padded(r.duration.unwrap_or(0.)),
            cover_path: r.thumbnail_path.clone(),
            launch: Launch::Recent(r.path.clone()),
        });
    }
    let already: HashSet<String> = cards.iter().map(|c| c.key.clone()).collect();
    for t in data.random.iter().filter(|t| !seen.contains(t.path.as_str())) {
        if cards.len() >= UP_NEXT_TOTAL {
            break;
        }
        let key = format!("h:{}", t.path);
        if already.contains(&key) {
            continue;
        }
        cards.push(Card {
            key,
            title: t.title.clone(),
            artist_name: readable_artists(t.artist.as_deref()).unwrap_or_default(),
            duration: t.duration.unwrap_or(0.),
            duration_label: karbon_ui::intl::clock_padded(t.duration.unwrap_or(0.)),
            cover_path: t.thumbnail_path.clone(),
            launch: Launch::Random(t.path.clone()),
        });
    }
    cards
}
