use crate::models::podcasts::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::models::layout::PopinSize;
use crate::models::layout::ToastKind;
use crate::commands::online::podcast_command;

impl MusicApp {
    pub(crate) fn podcast_preview(&mut self) -> Option<&mut PodcastPreviewState> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::PodcastPreview(f)) => Some(f),
            _ => None,
        }
    }

    pub(crate) fn podcast_preview_update(&mut self, msg: PodcastPreviewMsg) -> bool {
        match msg {
            PodcastPreviewMsg::Open => {
                let Some(m) = self.views.podcast_menu.take() else { return false };
                let mut p = PodcastPreviewState {
                    feed_url: String::new(),
                    title: m.title.clone(),
                    author: None,
                    genre: None,
                    released_at: None,
                    image: None,
                    preview: None,
                    error: false,
                    open_desc: false,
                    adding: false,
                };
                if let Some((l, i)) = m.directory {
                    let Some(r) = self.directory_at(l, i) else { return false };
                    p.image = r.image_url.as_ref().and_then(|u| self.views.podcasts.images.get(u).cloned().flatten());
                    (p.feed_url, p.author, p.genre, p.released_at) = (r.feed_url, r.author, r.genre, r.released_at);
                }
                if let Some((id, _)) = m.followed {
                    if let Some(s) = self.data.podcasts.iter().find(|x| x.id == id) {
                        p.feed_url = s.feed_url.clone();
                        p.author = p.author.or_else(|| s.author.clone());
                        p.image = s.cover_path.clone().or(p.image);
                    }
                }
                let url = p.feed_url.clone();
                self.open_popin(Popin { title: m.title, icon: None, body: PopinBody::PodcastPreview(Box::new(p)), size: PopinSize::Xl, bare: true, flush: true });
                self.backend.call(async move { podcast_command::podcast_preview(url).await }, |me, r| {
                    let Some(p) = me.podcast_preview() else { return false };
                    match r {
                        Ok(v) => p.preview = Some(v),
                        Err(_) => p.error = true,
                    }
                    true
                });
            }
            PodcastPreviewMsg::ToggleDesc => {
                let Some(p) = self.podcast_preview() else { return false };
                p.open_desc = !p.open_desc;
            }
            PodcastPreviewMsg::Site => {
                if let Some(url) = self.podcast_preview().and_then(|p| p.site()) {
                    crate::controller::settings::open_url(&url);
                }
                return false;
            }
            PodcastPreviewMsg::Follow | PodcastPreviewMsg::OpenPage => {
                let open = msg == PodcastPreviewMsg::OpenPage;
                let Some(p) = self.views.popin.as_ref().and_then(|x| match &x.body {
                    PopinBody::PodcastPreview(p) => Some(p.as_ref().clone()),
                    _ => None,
                }) else {
                    return false;
                };
                if let Some(id) = p.followed(self) {
                    if open {
                        self.views.popin = None;
                        return self.handle(Msg::Go(Route::Podcast(id)));
                    }
                    return false;
                }
                let Some(profile) = self.data.profile_id() else { return false };
                if let Some(f) = self.podcast_preview() {
                    f.adding = true;
                }
                let app = self.backend.app.clone();
                let url = p.feed_url.clone();
                self.backend.call(
                    async move {
                        let r = podcast_command::podcast_subscribe(app.state(), profile, url).await;
                        let all = podcast_command::get_podcasts(app.state(), profile).await.unwrap_or_default();
                        (r, all)
                    },
                    move |me, (r, all)| {
                        me.data.podcasts = all;
                        if let Some(f) = me.podcast_preview() {
                            f.adding = false;
                        }
                        match r {
                            Ok(s) => {
                                me.toast(ToastKind::Success, t("podcasts.followed").replace("{title}", &s.title), None);
                                if open {
                                    me.views.popin = None;
                                    return me.handle(Msg::Go(Route::Podcast(s.id)));
                                }
                            }
                            Err(e) => me.toast(ToastKind::Error, t("podcasts.error_follow"), Some(e)),
                        }
                        true
                    },
                );
            }
        }
        true
    }
}
