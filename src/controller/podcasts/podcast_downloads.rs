use crate::models::podcasts::*;
use crate::app::music_app::MusicApp;
use crate::commands::online::podcast_command;
use crate::entity::podcast::podcast::{DownloadView, EpisodeView};

impl MusicApp {
    pub(crate) fn downloads_open(&mut self) {
        let Some(profile) = self.data.profile_id() else { return };
        let app = self.backend.app.clone();
        self.backend.call(
            async move { podcast_command::get_podcast_downloads(app.state(), profile).await.unwrap_or_default() },
            |me, l| {
                me.views.downloads.list = Some(l);
                true
            },
        );
    }

    pub(crate) fn downloads_update(&mut self, msg: DownloadsMsg) -> bool {
        match msg {
            DownloadsMsg::Confirm(k, id) => {
                self.views.downloads.confirm = (k.is_some() || id.is_some()).then_some((k, id));
            }
            DownloadsMsg::Clear(podcast, played_only) => {
                let Some(profile) = self.data.profile_id() else { return false };
                self.views.downloads.confirm = None;
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        let _ = podcast_command::podcast_delete_downloads(app.state(), profile, podcast, played_only).await;
                    },
                    |me, ()| {
                        me.downloads_open();
                        true
                    },
                );
            }
            DownloadsMsg::Delete(id) => {
                if let Some(l) = self.views.downloads.list.as_mut() {
                    l.retain(|d| d.id != id);
                }
                let app = self.backend.app.clone();
                self.backend.fire(async move {
                    let _ = podcast_command::podcast_delete_download(app.state(), id).await;
                });
            }
            DownloadsMsg::Play(id) => {
                let Some(list) = self.views.downloads.list.clone() else { return false };
                let Some(d) = list.iter().find(|d| d.id == id).cloned() else { return false };
                if self.data.player.path.as_deref() == Some(d.file_path.as_str()) {
                    return self.handle(crate::models::app::Msg::TogglePlay);
                }
                let ep = |x: &DownloadView| EpisodeView {
                    id: x.id,
                    podcast_id: x.podcast_id,
                    title: x.title.clone(),
                    summary: None,
                    description: None,
                    published_at: x.published_at.clone(),
                    duration: x.duration,
                    audio_url: String::new(),
                    audio_type: None,
                    audio_size: None,
                    image_url: None,
                    season: None,
                    episode: None,
                    file_path: Some(x.file_path.clone()),
                    position: x.position,
                    played: x.played,
                    is_new: false,
                    path: x.file_path.clone(),
                };
                let same: Vec<&DownloadView> = list.iter().filter(|x| x.podcast_id == d.podcast_id).collect();
                let i = same.iter().position(|x| x.id == id).unwrap_or(0);
                let following: Vec<EpisodeView> = same[i + 1..].iter().map(|x| ep(x)).collect();
                self.play_episode(&d.podcast_title, d.cover_path.as_deref(), &ep(&d), &following);
            }
        }
        true
    }
}
