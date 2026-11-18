//! Les podcasts : chargement des données et réponse aux messages.

use crate::models::podcasts::*;
use crate::mapper::podcasts::*;
use karbon_ui::ui::runtime::Key;
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::commands::online::podcast_command;
use crate::entity::podcast::podcast::EpisodeView;
use crate::entity::queue::queue_track::QueueTrack;

impl MusicApp {
    pub(crate) fn podcasts_open(&mut self) {
        if self.data.setting("podcasts") != Some("true") {
            return;
        }
        self.podcasts_latest();
        self.podcasts_top();
    }

    pub(crate) fn podcasts_latest(&mut self) {
        let Some(profile) = self.data.profile_id() else { return };
        let app = self.backend.app.clone();
        self.backend.call(
            async move { podcast_command::get_podcast_latest(app.state(), profile, Some(20)).await.unwrap_or_default() },
            |me, l| {
                me.views.podcasts.latest = l;
                true
            },
        );
    }

    pub(crate) fn podcasts_top(&mut self) {
        let country = self.podcast_country();
        let genre = self.views.podcasts.genre;
        if self.views.podcasts.popular_for == (country.clone(), genre) && self.views.podcasts.popular.is_some() {
            return;
        }
        let page = &mut self.views.podcasts;
        page.popular = None;
        page.popular_error = false;
        page.all = false;
        page.popular_for = (country.clone(), genre);
        self.backend.call(
            async move { podcast_command::podcast_top(country.clone(), genre).await.map(|l| (country, genre, l)) },
            |me, result| {
                match result {
                    Ok((country, genre, l)) if me.views.podcasts.popular_for == (country.clone(), genre) => me.views.podcasts.popular = Some(l),
                    Ok(_) => return false,
                    Err(_) => me.views.podcasts.popular_error = true,
                }
                true
            },
        );
    }

    /// `follow` : s'abonne au flux, puis ouvre sa page si demandé.
    pub(crate) fn podcast_follow(&mut self, url: String, open: bool) {
        let Some(profile) = self.data.profile_id() else { return };
        self.views.podcasts.added = Some(url.clone());
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let p = podcast_command::podcast_subscribe(app.state(), profile, url).await;
                let all = podcast_command::get_podcasts(app.state(), profile).await.unwrap_or_default();
                (p, all)
            },
            move |me, (p, all)| {
                me.views.podcasts.added = None;
                me.data.podcasts = all;
                match p {
                    Ok(p) if open => {
                        me.views.podcasts.input.clear();
                        return me.handle(Msg::Go(Route::Podcast(p.id)));
                    }
                    Ok(_) => {}
                    Err(e) => log::warn!("Podcasts : {e}"),
                }
                true
            },
        );
    }

    pub(crate) fn podcasts_update(&mut self, msg: PodcastsMsg) -> bool {
        match msg {
            PodcastsMsg::Input(v) => {
                self.views.podcasts.input = v;
                if self.views.podcasts.input.trim().is_empty() {
                    self.views.podcasts.results = None;
                }
            }
            PodcastsMsg::Key(Key::Enter) | PodcastsMsg::Submit => {
                let input = self.views.podcasts.input.trim().to_string();
                if input.is_empty() {
                    return false;
                }
                if is_address(&input) {
                    self.podcast_follow(input, true);
                    return true;
                }
                let country = self.podcast_country();
                self.views.podcasts.search_term = true;
                self.backend.call(async move { podcast_command::podcast_search(input, country).await }, |me, r| {
                    me.views.podcasts.search_term = false;
                    match r {
                        Ok(l) => me.views.podcasts.results = Some(l),
                        Err(e) => log::warn!("Podcasts : {e}"),
                    }
                    true
                });
            }
            PodcastsMsg::Key(_) => return false,
            PodcastsMsg::Clear => {
                self.views.podcasts.input.clear();
                self.views.podcasts.results = None;
            }
            PodcastsMsg::Filter(v) => self.views.podcasts.filtre = v,
            PodcastsMsg::Genre(g) => {
                self.views.podcasts.genre = g;
                self.podcasts_top();
            }
            PodcastsMsg::Country(c) => {
                self.menu = None;
                self.set_setting("podcast_country", c);
                self.podcasts_top();
            }
            PodcastsMsg::Sort(k) => {
                self.views.podcasts.sort = k;
                self.menu = None;
            }
            PodcastsMsg::PopularSort(k) => {
                self.views.podcasts.sort_popular = k;
                self.menu = None;
            }
            PodcastsMsg::ShowAll => self.views.podcasts.all = true,
            PodcastsMsg::Open(l, i) => {
                let Some(r) = self.directory_at(l, i) else { return false };
                match self.data.podcasts.iter().find(|p| p.feed_url == r.feed_url) {
                    Some(p) => return self.handle(Msg::Go(Route::Podcast(p.id))),
                    None => self.podcast_follow(r.feed_url, true),
                }
            }
            PodcastsMsg::Follow(l, i) => {
                let Some(r) = self.directory_at(l, i) else { return false };
                self.podcast_follow(r.feed_url, false);
            }
            PodcastsMsg::PlayLatest(id) => {
                let Some(l) = self.views.podcasts.latest.iter().find(|l| l.episode.id == id).cloned() else { return false };
                let (title, cover) = (l.podcast_title.clone(), l.cover_path.clone());
                let pid = l.episode.podcast_id;
                let app = self.backend.app.clone();
                self.backend.call(
                    async move { podcast_command::get_podcast_episodes(app.state(), pid).await.unwrap_or_default() },
                    move |me, list| {
                        let i = list.iter().position(|x| x.id == l.episode.id);
                        let following: Vec<EpisodeView> = i.map(|i| list[i + 1..].to_vec()).unwrap_or_default();
                        me.play_episode(&title, cover.as_deref(), &l.episode, &following);
                        true
                    },
                );
            }
            PodcastsMsg::Pin(id, pinned) => {
                if let Some(p) = self.data.podcasts.iter_mut().find(|p| p.id == id) {
                    p.pinned = pinned;
                }
                let app = self.backend.app.clone();
                self.backend.fire(async move {
                    let _ = podcast_command::set_podcast_pinned(app.state(), id, pinned).await;
                });
            }
            PodcastsMsg::CheckAll => {
                let Some(profile) = self.data.profile_id() else { return false };
                self.views.podcasts.verification = true;
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        let _ = podcast_command::podcast_refresh(app.state(), None).await;
                        podcast_command::get_podcasts(app.state(), profile).await.unwrap_or_default()
                    },
                    |me, all| {
                        me.views.podcasts.verification = false;
                        me.data.podcasts = all;
                        me.podcasts_latest();
                        true
                    },
                );
            }
            PodcastsMsg::ListMenu => {
                self.menu = if self.popup("podcasts.listen") { None } else { Some(crate::models::app::Menu::Popup("podcasts.listen")) };
            }
            PodcastsMsg::EmptyListen => {
                self.menu = None;
                let Some(profile) = self.data.profile_id() else { return false };
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        let _ = podcast_command::podcast_empty_listen(app.state(), profile).await;
                    },
                    |me, ()| {
                        me.podcasts_latest();
                        true
                    },
                );
            }
        }
        true
    }

    /// L'épisode puis les suivants non écoutés de l'émission (20 au plus).
    pub(crate) fn play_episode(&mut self, podcast: &str, cover: Option<&str>, episode: &EpisodeView, following: &[EpisodeView]) {
        let profile = self.data.profile_id().unwrap_or(0);
        let list: Vec<QueueTrack> = std::iter::once(episode)
            .chain(following.iter().filter(|e| !e.played && e.id != episode.id).take(20))
            .map(|e| QueueTrack {
                queue_id: String::new(),
                profile_id: profile,
                position: 0,
                path: e.path.clone(),
                title: e.title.clone(),
                artist: Some(podcast.to_string()),
                duration: e.duration.map(|d| d as f64),
                cover: cover.map(String::from),
            })
            .collect();
        let path = episode.path.clone();
        self.play_list(list, &path);
    }

    /// Les visuels de l'annuaire affichés et pas encore en cache.
    pub(crate) fn podcasts_images(&mut self) {
        let page = &self.views.podcasts;
        let mut urls: Vec<String> = Vec::new();
        for l in [page.results.as_ref(), page.popular.as_ref()].into_iter().flatten() {
            for r in l.iter().take(if page.all { usize::MAX } else { 60 }) {
                if let Some(u) = r.image_url.as_ref().filter(|u| !page.images.contains_key(*u)) {
                    urls.push(u.clone());
                }
            }
        }
        for u in urls {
            self.views.podcasts.images.insert(u.clone(), None);
            let url = u.clone();
            self.backend.call(async move { crate::service::radio::radio_service::logo(&url).await }, move |me, path| {
                me.views.podcasts.images.insert(u, path);
                true
            });
        }
    }
}
