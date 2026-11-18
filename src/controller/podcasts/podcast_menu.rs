use crate::models::podcasts::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::layout::ToastKind;
use crate::models::podcasts::PodcastsMsg;
use crate::commands::online::podcast_command;

impl MusicApp {
    pub(crate) fn podcast_menu_update(&mut self, msg: PodcastMenuMsg) -> bool {
        match msg {
            PodcastMenuMsg::OpenFollowed(at, id) => {
                let Some(p) = self.data.podcasts.iter().find(|p| p.id == id) else { return false };
                self.menu = None;
                self.views.podcast_menu = Some(PodcastMenu { at, title: p.title.clone(), followed: Some((p.id, p.pinned)), directory: None, confirm: false });
            }
            PodcastMenuMsg::OpenDirectory(at, l, i) => {
                let Some(r) = self.directory_at(l, i) else { return false };
                let tracked = self.data.podcasts.iter().find(|p| p.feed_url == r.feed_url).map(|p| (p.id, p.pinned));
                self.menu = None;
                self.views.podcast_menu = Some(PodcastMenu { at, title: r.title, followed: tracked, directory: Some((l, i)), confirm: false });
            }
            PodcastMenuMsg::Unfollow => {
                let Some(m) = self.views.podcast_menu.as_mut() else { return false };
                if !m.confirm {
                    m.confirm = true;
                    return true;
                }
                let Some((id, _)) = self.views.podcast_menu.take().and_then(|m| m.followed) else { return false };
                let app = self.backend.app.clone();
                let profile = self.data.profile_id().unwrap_or(0);
                self.backend.call(
                    async move {
                        podcast_command::podcast_unsubscribe(app.state(), id).await?;
                        podcast_command::get_podcasts(app.state(), profile).await
                    },
                    |me, r| {
                        match r {
                            Ok(l) => me.data.podcasts = l,
                            Err(e) => me.toast(ToastKind::Error, t("notify.error"), Some(e)),
                        }
                        true
                    },
                );
            }
            PodcastMenuMsg::Preview => return self.podcast_preview_update(crate::models::podcasts::PodcastPreviewMsg::Open),
            PodcastMenuMsg::Close => self.views.podcast_menu = None,
            PodcastMenuMsg::Go | PodcastMenuMsg::Pin | PodcastMenuMsg::Follow => {
                let Some(m) = self.views.podcast_menu.take() else { return false };
                let following = match (msg, m.followed, m.directory) {
                    (PodcastMenuMsg::Go, Some((id, _)), _) => Msg::Go(Route::Podcast(id)),
                    (PodcastMenuMsg::Pin, Some((id, pinned)), _) => Msg::Podcasts(PodcastsMsg::Pin(id, !pinned)),
                    (PodcastMenuMsg::Follow, _, Some((l, i))) => Msg::Podcasts(PodcastsMsg::Follow(l, i)),
                    _ => return true,
                };
                return self.handle(following);
            }
        }
        true
    }
}
