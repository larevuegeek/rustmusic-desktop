use crate::models::radios::*;
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use crate::models::radios::RadiosMsg;
use crate::commands::online::radio_command;

impl MusicApp {
    pub(crate) fn radios_menu_update(&mut self, msg: RadiosMsg) -> bool {
        match msg {
            RadiosMsg::Menu(list, i, at) => {
                let Some((station, _, _)) = self.radio_at(list, i) else { return false };
                let favorite = self.data.radios.iter().any(|f| (station.uuid.is_some() && f.uuid == station.uuid) || f.stream_url == station.stream_url);
                self.menu = None;
                self.views.radio_menu = Some(RadioMenu {
                    at,
                    list,
                    i,
                    name: station.name.clone(),
                    favorite,
                    homepage: station.homepage.clone().filter(|h| h.starts_with("http://") || h.starts_with("https://")),
                    current: station.stream_url.clone(),
                    variants: station.variants.clone(),
                });
                if station.variants.is_empty() {
                    let (name, country) = (station.name, station.country_code);
                    self.backend.call(async move { radio_command::radio_variants(name, country).await.unwrap_or_default() }, |me, v| {
                        let Some(m) = me.views.radio_menu.as_mut() else { return false };
                        m.variants = v;
                        true
                    });
                }
            }
            RadiosMsg::MenuClose => self.views.radio_menu = None,
            RadiosMsg::MenuPlay | RadiosMsg::MenuFavorite => {
                let Some(m) = self.views.radio_menu.take() else { return false };
                let following = if msg == RadiosMsg::MenuPlay { RadiosMsg::Play(m.list, m.i) } else { RadiosMsg::Favorite(m.list, m.i) };
                return self.handle(Msg::Radios(following));
            }
            RadiosMsg::MenuSite => {
                if let Some(url) = self.views.radio_menu.take().and_then(|m| m.homepage) {
                    crate::controller::settings::open_url(&url);
                }
            }
            RadiosMsg::MenuVariant(k) => {
                let Some(m) = self.views.radio_menu.take() else { return false };
                let Some(v) = m.variants.get(k).cloned() else { return false };
                let Some((mut station, _, id)) = self.radio_at(m.list, m.i) else { return false };
                station.stream_url = v.stream_url.clone();
                station.codec = v.codec.clone();
                station.bitrate = v.bitrate;
                station.reduced_quality = v.reduced_quality;
                let favorite = self.data.radios.iter().find(|f| f.name == m.name).map(|f| f.id);
                let profile = self.data.profile_id().unwrap_or(0);
                self.data.player.path = Some(crate::core::radio::live::LIVE_PATH.to_string());
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        if let Some(f) = favorite {
                            let _ = radio_command::radio_set_stream(app.state(), f, v).await;
                        }
                        radio_command::radio_play(app.clone(), app.state(), profile, station, id.or(favorite)).await
                    },
                    |_, r| {
                        if let Err(e) = r {
                            log::warn!("Radio : {e}");
                        }
                        true
                    },
                );
            }
            _ => return false,
        }
        true
    }
}
