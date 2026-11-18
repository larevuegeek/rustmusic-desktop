//! Les sources (cet ordinateur, serveurs) et les bibliothèques.

use crate::models::home::*;
use crate::models::app::Data;
use karbon_ui::intl::{t, t_count, t_with};

/// Cet ordinateur, chaque serveur, puis toutes les sources.
pub(crate) fn sources(data: &Data) -> Vec<Source> {
    let locales: Vec<_> = data.libraries.iter().filter(|l| l.server_id.is_none()).collect();
    let tracks_count: i64 = locales.iter().map(|l| l.total_tracks).sum();
    let mut sources = vec![Source {
        id: Some(0),
        name: "Cet ordinateur".into(),
        icon: "material-symbols:computer-outline-rounded",
        tint: "local",
        detail: format!(
            "{} · {}",
            match locales.len() {
                1 => t("sources.library_one"),
                n => t_with("sources.libraries_n", &[("n", &n.to_string())]),
            },
            t_count(tracks_count, "home.track_one", "home.tracks_n")
        ),
        online: None,
    }];
    for s in &data.servers {
        let tracks_count: i64 = data
            .libraries
            .iter()
            .filter(|l| l.server_id == Some(s.server.id))
            .map(|l| l.total_tracks)
            .sum();
        sources.push(Source {
            id: Some(s.server.id),
            name: s.server.name.clone(),
            icon: "material-symbols:cloud-outline-rounded",
            tint: "serveur",
            detail: format!(
                "{} · {}",
                if data.server_online.get(&s.server.id).copied().unwrap_or(false) { t("sources.online") } else { t("sources.unreachable") },
                t_count(tracks_count, "home.track_one", "home.tracks_n")
            ),
            online: Some(data.server_online.get(&s.server.id).copied().unwrap_or(false)),
        });
    }
    if !data.servers.is_empty() {
        let total: i64 = data.libraries.iter().map(|l| l.total_tracks).sum();
        sources.push(Source {
            id: None,
            name: t("sources.all"),
            icon: "material-symbols:stacks-outline-rounded",
            tint: "toutes",
            detail: t_count(total, "home.track_one", "home.tracks_n"),
            online: None,
        });
    }
    sources
}

pub(crate) fn libraries(data: &Data, source: Option<i64>) -> Vec<Library> {
    data.libraries
        .iter()
        .filter(|l| match source {
            None => true,
            Some(0) => l.server_id.is_none(),
            Some(id) => l.server_id == Some(id),
        })
        .map(|l| {
            let tracks_count = t_count(l.total_tracks, "home.track_one", "home.tracks_n");
            let albums = if l.total_albums == 1 {
                t("years_view.year_album")
            } else {
                t_with("years_view.year_albums", &[("n", &l.total_albums.to_string())])
            };
            Library {
                id: l.id,
                name: l.name.clone(),
                summary: format!("{tracks_count} · {albums}"),
                tracks_count,
                albums,
                default_value: l.is_default,
            }
        })
        .collect()
}
