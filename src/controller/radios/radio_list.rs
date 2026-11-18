//! Les radios : chargement des données et réponse aux messages.

use crate::models::radios::*;
use crate::mapper::radios::*;
use karbon_ui::ui::runtime::Key;
use crate::app::music_app::MusicApp;
use crate::commands::online::radio_command;

impl MusicApp {
    pub(crate) fn radios_open(&mut self) {
        if self.data.setting("radios") != Some("true") {
            return;
        }
        let Some(profile) = self.data.profile_id() else { return };
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let recent = radio_command::get_radio_recent(app.state(), profile).await.unwrap_or_default();
                let titles = radio_command::get_radio_titles(app.state(), profile).await.unwrap_or_default();
                (recent, titles)
            },
            |me, (recent, titles)| {
                me.views.radios.recent_items = recent;
                me.views.radios.tracks_count = titles;
                true
            },
        );
        self.radios_top();
    }

    pub(crate) fn radios_top(&mut self) {
        let country = self.radio_country();
        let genre = self.views.radios.genre;
        if self.views.radios.popular_for == (country.clone(), genre) && self.views.radios.popular.is_some() {
            return;
        }
        let page = &mut self.views.radios;
        page.popular = None;
        page.popular_error = false;
        page.all = false;
        page.popular_for = (country.clone(), genre);
        self.backend.call(
            async move { radio_command::radio_top(country.clone(), genre.map(String::from)).await.map(|l| (country, genre, l)) },
            |me, result| {
                match result {
                    Ok((country, genre, l)) if me.views.radios.popular_for == (country.clone(), genre) => me.views.radios.popular = Some(l),
                    Ok(_) => return false,
                    Err(_) => me.views.radios.popular_error = true,
                }
                true
            },
        );
    }

    pub(crate) fn radios_update(&mut self, msg: RadiosMsg) -> bool {
        match msg {
            m @ (RadiosMsg::Menu(..) | RadiosMsg::MenuClose | RadiosMsg::MenuPlay | RadiosMsg::MenuFavorite | RadiosMsg::MenuSite | RadiosMsg::MenuVariant(_)) => {
                return self.radios_menu_update(m);
            }
            RadiosMsg::Input(v) => {
                self.views.radios.input = v;
                if self.views.radios.input.trim().is_empty() {
                    self.views.radios.results = None;
                }
            }
            RadiosMsg::Key(Key::Enter) | RadiosMsg::Submit => {
                let input = self.views.radios.input.trim().to_string();
                if input.is_empty() {
                    return false;
                }
                let address = is_address(&input);
                self.views.radios.search_term = true;
                self.backend.call(
                    async move {
                        if address {
                            radio_command::radio_resolve(input).await.map(|r| vec![r])
                        } else {
                            radio_command::radio_search(input, None, None).await
                        }
                    },
                    |me, result| {
                        me.views.radios.search_term = false;
                        match result {
                            Ok(l) => me.views.radios.results = Some(l),
                            Err(e) => log::warn!("Radios : {e}"),
                        }
                        true
                    },
                );
            }
            RadiosMsg::Key(_) => return false,
            RadiosMsg::Clear => {
                self.views.radios.input.clear();
                self.views.radios.results = None;
            }
            RadiosMsg::Genre(g) => {
                self.views.radios.genre = g;
                self.radios_top();
            }
            RadiosMsg::Country(c) => {
                self.menu = None;
                self.set_setting("radio_country", c);
                self.radios_top();
            }
            RadiosMsg::ShowAll => self.views.radios.all = true,
            RadiosMsg::Play(l, i) => {
                let Some((station, _logo, id)) = self.radio_at(l, i) else { return false };
                let profile = self.data.profile_id().unwrap_or(0);
                self.data.player.path = Some(crate::core::radio::live::LIVE_PATH.to_string());
                self.data.player.audio = None;
                self.data.player.duration = 0.;
                let app = self.backend.app.clone();
                self.backend.call(
                    async move { radio_command::radio_play(app.clone(), app.state(), profile, station, id).await },
                    |_, result| {
                        if let Err(e) = result {
                            log::warn!("Radio : {e}");
                        }
                        true
                    },
                );
            }
            RadiosMsg::Favorite(l, i) => {
                let Some((station, _, _)) = self.radio_at(l, i) else { return false };
                let Some(profile) = self.data.profile_id() else { return false };
                let existing = self.data.radios.iter().find(|f| (station.uuid.is_some() && f.uuid == station.uuid) || f.stream_url == station.stream_url).map(|f| f.id);
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        match existing {
                            Some(id) => {
                                let _ = radio_command::radio_remove(app.state(), id).await;
                            }
                            None => {
                                let _ = radio_command::radio_add(app.state(), profile, station).await;
                            }
                        }
                        radio_command::get_radios(app.state(), profile).await.unwrap_or_default()
                    },
                    |me, radios| {
                        me.data.radios = radios;
                        true
                    },
                );
            }
            RadiosMsg::TitlesAll => self.views.radios.all_tracks = !self.views.radios.all_tracks,
            RadiosMsg::TitlesClear => {
                let Some(profile) = self.data.profile_id() else { return false };
                self.views.radios.tracks_count.clear();
                let app = self.backend.app.clone();
                self.backend.fire(async move {
                    let _ = radio_command::clear_radio_titles(app.state(), profile).await;
                });
            }
        }
        true
    }

    /// Les logos de l'annuaire affichés et pas encore en cache : demandés une fois.
    pub(crate) fn radios_logos(&mut self) {
        let v = self.radios_view();
        let mut favicons: Vec<String> = Vec::new();
        let page = &self.views.radios;
        for (l, n) in [('r', v.results.as_ref().map_or(0, Vec::len)), ('p', v.popular.as_ref().map_or(0, Vec::len))] {
            for i in 0..n {
                if let Some((r, _, _)) = self.radio_at(l, i) {
                    if let Some(f) = r.favicon.filter(|f| !f.is_empty() && !page.logos.contains_key(f)) {
                        favicons.push(f);
                    }
                }
            }
        }
        for f in favicons {
            self.views.radios.logos.insert(f.clone(), None);
            let url = f.clone();
            self.backend.call(async move { radio_command::radio_logo(url).await.ok().flatten() }, move |me, path| {
                me.views.radios.logos.insert(f, path);
                true
            });
        }
    }

    /// `radio_play` d'une radio favorite.
    pub(crate) fn play_radio(&mut self, id: i64) {
        let Some(r) = self.data.radios.iter().find(|r| r.id == id).cloned() else {
            return;
        };
        let profile = self.data.profile_id().unwrap_or(0);
        let station = crate::core::radio::directory::RadioResult {
            uuid: r.uuid.clone(),
            name: r.name.clone(),
            stream_url: r.stream_url.clone(),
            homepage: r.homepage.clone(),
            favicon: r.favicon_url.clone(),
            country_code: r.country_code.clone(),
            tags: r.tags.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
            codec: r.codec.clone(),
            bitrate: r.bitrate.and_then(|b| u32::try_from(b).ok()),
            reduced_quality: r.reduced_quality,
            variants: Vec::new(),
        };
        self.data.player.path = Some(crate::core::radio::live::LIVE_PATH.to_string());
        self.data.player.audio = None;
        self.data.player.duration = 0.;
        let app = self.backend.app.clone();
        self.backend.call(
            async move { radio_command::radio_play(app.clone(), app.state(), profile, station, Some(id)).await },
            |_, result| {
                if let Err(e) = result {
                    log::warn!("Radio : {e}");
                }
                true
            },
        );
    }
}
