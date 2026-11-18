use crate::models::podcasts::*;
use karbon_ui::ui::prelude::Color;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use crate::helper::color::hex;
use karbon_ui::intl::{t, t_with};
use crate::mapper::podcasts::episode_date;
use crate::mapper::podcasts::episode_duration;
use crate::entity::podcast::podcast::DownloadView;

/// Une couleur par émission, des plus grosses aux plus petites ; le reste en « Autres ».
pub(crate) const COLORS: [&str; 5] = ["#8b5cf6", "#d946ef", "#6366f1", "#0ea5e9", "#14b8a6"];

pub(crate) fn size(n: u64) -> String {
    if n > 0 { karbon_ui::intl::bytes(n) } else { "0".into() }
}

impl MusicApp {
    pub fn downloads_view(&self) -> DownloadsView {
        let page = &self.views.downloads;
        let list = page.list.as_deref().unwrap_or(&[]);
        let total: u64 = list.iter().map(|d| d.size).sum();
        let listens: Vec<&DownloadView> = list.iter().filter(|d| d.played).collect();
        let listens_size: u64 = listens.iter().map(|d| d.size).sum();
        let mut groups: Vec<(i64, String, Option<String>, Vec<&DownloadView>, u64)> = Vec::new();
        for d in list {
            match groups.iter_mut().find(|g| g.0 == d.podcast_id) {
                Some(g) => {
                    g.3.push(d);
                    g.4 += d.size;
                }
                None => groups.push((d.podcast_id, d.podcast_title.clone(), d.cover_path.clone(), vec![d], d.size)),
            }
        }
        let mut by_size: Vec<&(i64, String, Option<String>, Vec<&DownloadView>, u64)> = groups.iter().collect();
        by_size.sort_by(|a, b| b.4.cmp(&a.4));
        let gray = Color::rgba(0x6f, 0x78, 0x73, 255);
        let color = |id: i64| by_size.iter().take(COLORS.len()).position(|g| g.0 == id).map(|i| hex(COLORS[i])).unwrap_or(gray);
        let mut segments: Vec<(String, String, f32, Color)> = by_size
            .iter()
            .take(COLORS.len())
            .enumerate()
            .map(|(i, g)| (g.0.to_string(), g.1.clone(), g.4 as f32, hex(COLORS[i])))
            .collect();
        let rest: u64 = by_size.iter().skip(COLORS.len()).map(|g| g.4).sum();
        if rest > 0 {
            segments.push(("autres".into(), t("podcasts.others"), rest as f32, gray));
        }
        let segments = segments
            .into_iter()
            .map(|(key, title, weight, color)| StorageSegment { key, title, size: size(weight as u64), weight, color })
            .collect();
        let playing_path = self.data.player.path.as_deref();
        let groups = groups
            .iter()
            .map(|(id, title, cover, items, group_size)| ShowGroup {
                id: *id,
                title: title.clone(),
                cover: cover.clone(),
                color: color(*id),
                subline: format!("{} {} · {}", intl::number(items.len() as i64), t(if items.len() == 1 { "podcasts.episodes_one" } else { "podcasts.episodes_n" }), size(*group_size)),
                confirm: page.confirm == Some((None, Some(*id))),
                items: items
                    .iter()
                    .map(|d| {
                        let duration = d.duration.unwrap_or(0) as f64;
                        let playback = playing_path == Some(d.file_path.as_str());
                        DownloadRow {
                            id: d.id,
                            title: d.title.clone(),
                            meta: [episode_date(d.published_at.as_deref()), episode_duration(duration)].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · "),
                            played: d.played,
                            progress: (!d.played && d.position > 0. && duration > 0.).then(|| ((d.position / duration).min(1.) * 100.) as f32),
                            size: size(d.size),
                            playback,
                            playing: playback && self.playing(),
                        }
                    })
                    .collect(),
            })
            .collect();
        DownloadsView {
            loading: page.list.is_none(),
            empty: list.is_empty(),
            episodes: (intl::number(list.len() as i64), t(if list.len() == 1 { "podcasts.episodes_one" } else { "podcasts.episodes_n" })),
            total: size(total),
            listens: intl::number(listens.len() as i64),
            recoverable: (!listens.is_empty()).then(|| t_with("podcasts.reclaimable", &[("size", &size(listens_size))])),
            segments,
            confirm: page.confirm.and_then(|(k, _)| k),
            delete_listens: (!listens.is_empty()).then(|| t_with("podcasts.delete_played", &[("n", &intl::number(listens.len() as i64))])),
            groups,
        }
    }
}
