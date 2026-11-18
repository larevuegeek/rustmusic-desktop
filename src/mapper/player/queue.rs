use karbon_ui::intl::{self, t, t_count};

use crate::app::music_app::MusicApp;
use crate::entity::queue::queue_track::QueueTrack;
use crate::models::app::{Menu, Status};
use crate::models::player::{QueueNowPlayingView, QueuePanelView, QueueRowView};
use crate::query::queue::queue_state_row::QueueStateRow;
use crate::models::shared::ListWindow;

/// La pastille d'état : libellé, en lecture.
fn status_label(s: &Status) -> (String, bool) {
    match s {
        Status::Playing => (t("queue_panel.status_playing"), true),
        Status::Paused => (t("home.paused"), false),
        Status::Ended => (t("queue_panel.status_ended"), false),
        Status::Ready => (t("queue_panel.status_ready"), false),
    }
}

/// Rang du premier titre après le morceau en cours.
fn next_index(queue: &QueueStateRow) -> usize {
    usize::try_from(queue.current_index + 1).unwrap_or(0)
}

/// Les titres à venir bâtis (`window`), avec leur rang dans la file.
pub fn upcoming<'a>(queue: &'a QueueStateRow, window: &ListWindow) -> Vec<(usize, &'a QueueTrack)> {
    let range = window.range(upcoming_total(queue));
    queue.tracks.iter().enumerate().skip(next_index(queue) + range.start).take(range.len()).collect()
}

pub fn upcoming_total(queue: &QueueStateRow) -> usize {
    queue.tracks.len().saturating_sub(next_index(queue))
}

fn artist(track: &QueueTrack) -> String {
    track.artist.clone().unwrap_or_else(|| t("common.unknown_artist"))
}

fn duration(track: &QueueTrack) -> String {
    intl::clock_padded(track.duration.unwrap_or(0.))
}

/// La carte du morceau en cours (volet de la file, paroles).
pub fn now_playing(track: &QueueTrack) -> QueueNowPlayingView {
    QueueNowPlayingView {
        title: track.title.clone(),
        artist: artist(track),
        cover: track.cover.clone(),
        duration: duration(track),
    }
}

/// Le volet de la file : `current` est le morceau en cours, `window` les titres à venir bâtis.
pub fn panel(queue: &QueueStateRow, current: Option<&QueueTrack>, status: Status, window: &ListWindow) -> QueuePanelView {
    let (label, playing) = status_label(&status);
    let next = next_index(queue);
    let total = upcoming_total(queue);
    QueuePanelView {
        playing,
        paused: status == Status::Paused,
        status: label,
        count: t_count(queue.tracks.len() as i64, "queue_panel.tracks_one", "queue_panel.tracks_n"),
        toggle_label: if playing { t("home.pause_short") } else { t("home.play") },
        has_tracks: !queue.tracks.is_empty(),
        now_playing: current.map(now_playing),
        upcoming_count: (total > 0)
            .then(|| t_count(total as i64, "queue_panel.upcoming_one", "queue_panel.upcoming_n")),
        upcoming: upcoming(queue, window)
            .into_iter()
            .map(|(index, track)| QueueRowView {
                index,
                rank: index + 1 - next,
                queue_id: track.queue_id.clone(),
                title: track.title.clone(),
                artist: artist(track),
                cover: track.cover.clone(),
                duration: duration(track),
            })
            .collect(),
    }
}

impl MusicApp {
    /// Le volet de la file, quand il est ouvert.
    pub fn queue_panel(&self) -> Option<QueuePanelView> {
        self.open(Menu::File).then(|| {
            panel(&self.data.queue, self.data.current_track(), self.data.player.status, &self.views.queue_window)
        })
    }

    /// La carte du morceau en cours.
    pub fn now_playing_card(&self) -> Option<QueueNowPlayingView> {
        self.data.current_track().map(now_playing)
    }

    /// Les titres à venir montrés, avec leur rang dans la file.
    pub fn queue_upcoming(&self) -> Vec<(usize, &QueueTrack)> {
        upcoming(&self.data.queue, &self.views.queue_window)
    }
}

#[cfg(test)]
mod tests;
