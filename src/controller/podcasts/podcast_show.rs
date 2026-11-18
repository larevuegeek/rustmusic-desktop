use crate::models::podcasts::*;
use crate::mapper::podcasts::*;
use crate::app::music_app::MusicApp;
use crate::models::app::Menu;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::commands::online::podcast_command;
use crate::entity::podcast::podcast::EpisodeView;
use crate::models::shared::ListWindow;

impl MusicApp {

    pub(crate) fn podcast_open(&mut self, id: i64) {
        if !self.views.show.as_ref().is_some_and(|p| p.id == id) {
            self.views.show = Some(ShowPage {
                id,
                podcast: None,
                episodes: Vec::new(),
                state: "chargement",
                unplayed_only: false,
                older: false,
                window: ListWindow::new(EPISODES),
                description: false,
                open_items: Vec::new(),
                downloads: Vec::new(),
                verification: false,
                confirm: false,
                menu_episode: None,
            });
        }
        self.show_load();
    }

    pub(crate) fn show_load(&mut self) {
        let Some(id) = self.views.show.as_ref().map(|p| p.id) else { return };
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let p = podcast_command::get_podcast(app.state(), id).await.ok().flatten();
                let e = podcast_command::get_podcast_episodes(app.state(), id).await.unwrap_or_default();
                (p, e)
            },
            move |me, (p, e)| {
                let Some(page) = me.views.show.as_mut().filter(|x| x.id == id) else { return false };
                page.state = if p.is_some() { "pret" } else { "introuvable" };
                page.podcast = p;
                page.episodes = e;
                page.downloads.retain(|d| page.episodes.iter().any(|x| x.id == *d && x.file_path.is_none()));
                true
            },
        );
    }

    pub(crate) fn show_update(&mut self, msg: ShowMsg) -> bool {
        let Some(id) = self.views.show.as_ref().map(|p| p.id) else { return false };
        match msg {
            ShowMsg::Play(eid) => {
                let Some(e) = self.views.show.as_ref().and_then(|p| p.episodes.iter().find(|e| e.id == eid)).cloned() else { return false };
                if self.episode_state(&e).is_some() {
                    return self.handle(Msg::TogglePlay);
                }
                let visible: Vec<EpisodeView> = self.show_visible().into_iter().cloned().collect();
                let i = visible.iter().position(|x| x.id == eid);
                let following: Vec<EpisodeView> = i.map(|i| visible[i + 1..].to_vec()).unwrap_or_default();
                let p = self.views.show.as_ref().and_then(|p| p.podcast.clone());
                let (title, cover) = p.map(|p| (p.title, p.cover_path)).unwrap_or_default();
                self.play_episode(&title, cover.as_deref(), &e, &following);
            }
            ShowMsg::PlayFirst => {
                let Some(eid) = self.show_to_listen().map(|e| e.id) else { return false };
                return self.show_update(ShowMsg::Play(eid));
            }
            ShowMsg::Refresh => {
                self.menu = None;
                if let Some(p) = self.views.show.as_mut() {
                    p.verification = true;
                }
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        let _ = podcast_command::podcast_refresh(app.state(), Some(id)).await;
                    },
                    |me, ()| {
                        if let Some(p) = me.views.show.as_mut() {
                            p.verification = false;
                        }
                        me.show_load();
                        true
                    },
                );
            }
            ShowMsg::Pin => {
                let Some(p) = self.views.show.as_mut().and_then(|p| p.podcast.as_mut()) else { return false };
                p.pinned = !p.pinned;
                let pinned = p.pinned;
                if let Some(x) = self.data.podcasts.iter_mut().find(|x| x.id == id) {
                    x.pinned = pinned;
                }
                let app = self.backend.app.clone();
                self.backend.fire(async move {
                    let _ = podcast_command::set_podcast_pinned(app.state(), id, pinned).await;
                });
            }
            ShowMsg::AutoDownload => {
                let Some(p) = self.views.show.as_mut().and_then(|p| p.podcast.as_mut()) else { return false };
                p.auto_download = !p.auto_download;
                let on = p.auto_download;
                let app = self.backend.app.clone();
                self.backend.fire(async move {
                    let _ = podcast_command::set_podcast_auto_download(app.state(), id, on).await;
                });
            }
            ShowMsg::SubscriptionMenu => {
                if let Some(p) = self.views.show.as_mut() {
                    p.confirm = false;
                }
                self.menu = if self.popup("podcast.subscription") { None } else { Some(Menu::Popup("podcast.subscription")) };
            }
            ShowMsg::ConfirmUnsubscribe(on) => {
                if let Some(p) = self.views.show.as_mut() {
                    p.confirm = on;
                }
            }
            ShowMsg::Unsubscribe => {
                self.menu = None;
                let profile = self.data.profile_id().unwrap_or(0);
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        let _ = podcast_command::podcast_unsubscribe(app.state(), id).await;
                        podcast_command::get_podcasts(app.state(), profile).await.unwrap_or_default()
                    },
                    |me, all| {
                        me.data.podcasts = all;
                        me.views.show = None;
                        me.handle(Msg::Go(Route::Podcasts))
                    },
                );
            }
            ShowMsg::MarkAllPlayed => {
                if let Some(p) = self.views.show.as_mut() {
                    for e in p.episodes.iter_mut() {
                        e.played = true;
                        e.is_new = false;
                        e.position = 0.;
                    }
                }
                let app = self.backend.app.clone();
                self.backend.fire(async move {
                    let _ = podcast_command::podcast_set_all_played(app.state(), id).await;
                });
            }
            ShowMsg::Order => {
                if let Some(p) = self.views.show.as_mut() {
                    p.older = !p.older;
                    p.window.reset();
                }
            }
            ShowMsg::Filter(unplayed) => {
                if let Some(p) = self.views.show.as_mut() {
                    p.unplayed_only = unplayed;
                    p.window.reset();
                }
            }
            ShowMsg::Scrolled(offset, max) => {
                let (len, export) = (self.show_visible().len(), self.export);
                let Some(p) = self.views.show.as_mut() else { return false };
                return p.window.scrolled(offset, max, len, export);
            }
            ShowMsg::Description => {
                if let Some(p) = self.views.show.as_mut() {
                    p.description = !p.description;
                }
            }
            ShowMsg::Expand(eid) => {
                let Some(p) = self.views.show.as_mut() else { return false };
                if let Some(i) = p.open_items.iter().position(|(x, _)| *x == eid) {
                    p.open_items.remove(i);
                    return true;
                }
                p.open_items.push((eid, None));
                let app = self.backend.app.clone();
                self.backend.call(
                    async move { podcast_command::get_podcast_episode(app.state(), eid).await.ok().flatten() },
                    move |me, e| {
                        let Some(p) = me.views.show.as_mut() else { return false };
                        if let Some(o) = p.open_items.iter_mut().find(|(x, _)| *x == eid) {
                            o.1 = Some(e.and_then(|e| e.description.or(e.summary)).unwrap_or_default());
                        }
                        true
                    },
                );
            }
            ShowMsg::TogglePlayed(eid) => {
                self.menu = None;
                let Some(e) = self.views.show.as_mut().and_then(|p| p.episodes.iter_mut().find(|e| e.id == eid)) else { return false };
                e.played = !e.played;
                e.is_new = false;
                e.position = 0.;
                let played = e.played;
                let app = self.backend.app.clone();
                self.backend.fire(async move {
                    let _ = podcast_command::podcast_set_played(app.state(), eid, played).await;
                });
            }
            ShowMsg::Download(eid) => {
                self.menu = None;
                if let Some(p) = self.views.show.as_mut() {
                    p.downloads.push(eid);
                }
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        let _ = podcast_command::podcast_download(app.state(), eid).await;
                    },
                    |me, ()| {
                        me.show_load();
                        true
                    },
                );
            }
            ShowMsg::DeleteDownload(eid) => {
                self.menu = None;
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        let _ = podcast_command::podcast_delete_download(app.state(), eid).await;
                    },
                    |me, ()| {
                        me.show_load();
                        true
                    },
                );
            }
            ShowMsg::EpisodeMenu(eid) => {
                let open = self.menu == Some(Menu::Popup("podcast.episode")) && self.views.show.as_ref().is_some_and(|p| p.menu_episode == Some(eid));
                self.menu = if open { None } else { Some(Menu::Popup("podcast.episode")) };
                if let Some(p) = self.views.show.as_mut() {
                    p.menu_episode = Some(eid);
                }
            }
        }
        true
    }
}
