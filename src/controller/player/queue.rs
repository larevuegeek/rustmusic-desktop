use crate::app::music_app::MusicApp;

impl MusicApp {
    pub(crate) fn queue_save(&mut self) {
        for (i, t) in self.data.queue.tracks.iter_mut().enumerate() {
            t.position = i as i32;
        }
        let all = self.data.queue.tracks.clone();
        let profile = self.data.profile_id().unwrap_or(0);
        let app = self.backend.app.clone();
        self.backend.fire(async move {
            let _ = crate::commands::playback::queue_command::replace_queue_tracks(app.state(), profile, all).await;
        });
    }

    /// `queueState.clear`.
    pub(crate) fn queue_clear(&mut self) -> bool {
        self.data.queue.tracks.clear();
        self.data.queue.current_index = -1;
        self.queue_save();
        true
    }

    /// `queueState.clearUpcoming` : la suite du morceau en cours est retirée (mini-lecteur).
    pub(crate) fn queue_clear_upcoming(&mut self) -> bool {
        let Ok(keep) = usize::try_from(self.data.queue.current_index + 1) else { return false };
        if keep == 0 || keep >= self.data.queue.tracks.len() {
            return false;
        }
        self.data.queue.tracks.truncate(keep);
        self.queue_save();
        true
    }

    /// `queueState.removeTrack` : le rang du morceau en cours suit.
    pub(crate) fn queue_remove(&mut self, queue_id: &str) -> bool {
        let Some(i) = self.data.queue.tracks.iter().position(|t| t.queue_id == queue_id) else { return false };
        self.data.queue.tracks.remove(i);
        if (i as i32) < self.data.queue.current_index {
            self.data.queue.current_index -= 1;
        }
        self.queue_save();
        true
    }
}
