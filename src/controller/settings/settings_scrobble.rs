use crate::models::settings::*;
use crate::mapper::settings::*;
use std::time::{Duration, Instant};
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::commands::online::scrobble_command;

/// Le code d'erreur traduit s'il est connu, sinon tel quel.
pub(crate) fn translate(prefix: &str, codes: &[&str], e: String) -> String {
    if codes.contains(&e.as_str()) {
        t(&format!("{prefix}.error_{e}"))
    } else {
        e
    }
}

/// La page dans le navigateur.
pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(url).spawn();
    if let Err(e) = r {
        log::warn!("Ouvrir {url} : {e}");
    }
}

impl MusicApp {
    pub(crate) fn settings_scrobble_load(&mut self) {
        let Some(profile) = self.profile_id() else { return };
        self.views.settings.scrobble.lf_builtin = scrobble_command::lastfm_has_builtin_key();
        let app = self.backend.app.clone();
        self.backend.call(
            async move { (scrobble_command::listenbrainz_status(app.state(), profile).await, scrobble_command::lastfm_status(app.state(), profile).await) },
            |me, (lb, lf)| {
                let s = &mut me.views.settings.scrobble;
                s.lb = lb.ok().flatten().map(account);
                s.lf = lf.ok().flatten().map(account);
                true
            },
        );
    }

    pub(crate) fn settings_scrobble_update(&mut self, msg: ScrobbleMsg) -> bool {
        let Some(profile) = self.profile_id() else { return false };
        let app = self.backend.app.clone();
        let s = &mut self.views.settings.scrobble;
        match msg {
            ScrobbleMsg::Open(url) => {
                open_url(&url);
                return false;
            }
            ScrobbleMsg::LbToken(v) => {
                s.lb_token = v;
                s.lb_error = None;
            }
            ScrobbleMsg::LbConnect => {
                let token = s.lb_token.trim().to_string();
                if token.is_empty() || s.lb_busy {
                    return false;
                }
                s.lb_busy = true;
                s.lb_error = None;
                self.backend.call(async move { scrobble_command::listenbrainz_connect(app.state(), profile, token).await }, |me, r| {
                    let s = &mut me.views.settings.scrobble;
                    s.lb_busy = false;
                    match r {
                        Ok(_) => {
                            s.lb_token.clear();
                            me.settings_scrobble_load();
                        }
                        Err(e) => s.lb_error = Some(translate("scrobble", &["unreachable", "token_refused", "no_account"], e)),
                    }
                    true
                });
            }
            ScrobbleMsg::LbDisconnect => {
                if !s.lb_confirm {
                    s.lb_confirm = true;
                    return true;
                }
                s.lb_confirm = false;
                s.lb = None;
                self.backend.fire(async move {
                    let _ = scrobble_command::listenbrainz_disconnect(app.state(), profile).await;
                });
            }
            ScrobbleMsg::LbKeep => s.lb_confirm = false,
            ScrobbleMsg::LbNowPlaying => {
                let Some(a) = s.lb.as_mut() else { return false };
                a.now_playing = !a.now_playing;
                let on = a.now_playing;
                self.backend.fire(async move {
                    let _ = scrobble_command::listenbrainz_set_now_playing(app.state(), profile, on).await;
                });
            }
            ScrobbleMsg::LfKey(v) => {
                s.lf_key = v;
                s.lf_error = None;
            }
            ScrobbleMsg::LfSecret(v) => {
                s.lf_secret = v;
                s.lf_error = None;
            }
            ScrobbleMsg::LfOwn => {
                s.lf_own = !s.lf_own;
                s.lf_error = None;
            }
            ScrobbleMsg::LfConnect => {
                if !s.lf_ready() || s.lf_busy {
                    return false;
                }
                s.lf_busy = true;
                s.lf_error = None;
                let key = s.lf_with_key().then(|| (s.lf_key.trim().to_string(), s.lf_secret.trim().to_string()));
                let (key, secret) = key.map(|(k, v)| (Some(k), Some(v))).unwrap_or((None, None));
                self.backend.call(async move { scrobble_command::lastfm_begin(profile, key, secret).await }, move |me, r| {
                    let s = &mut me.views.settings.scrobble;
                    s.lf_busy = false;
                    match r {
                        Ok(url) => {
                            open_url(&url);
                            s.lf_wait = Some(url);
                            s.lf_round += 1;
                            me.lastfm_poll(profile, me.views.settings.scrobble.lf_round);
                        }
                        Err(e) => s.lf_error = Some(translate("lastfm", &["unreachable", "bad_key", "expired", "no_key"], e)),
                    }
                    true
                });
            }
            ScrobbleMsg::LfReopen => {
                if let Some(url) = &s.lf_wait {
                    open_url(url);
                }
                return false;
            }
            ScrobbleMsg::LfCancel => {
                s.lf_wait = None;
                s.lf_round += 1;
                scrobble_command::lastfm_cancel(profile);
            }
            ScrobbleMsg::LfDisconnect => {
                if !s.lf_confirm {
                    s.lf_confirm = true;
                    return true;
                }
                s.lf_confirm = false;
                s.lf = None;
                self.backend.fire(async move {
                    let _ = scrobble_command::lastfm_disconnect(app.state(), profile).await;
                });
            }
            ScrobbleMsg::LfKeep => s.lf_confirm = false,
            ScrobbleMsg::LfNowPlaying => {
                let Some(a) = s.lf.as_mut() else { return false };
                a.now_playing = !a.now_playing;
                let on = a.now_playing;
                self.backend.fire(async move {
                    let _ = scrobble_command::lastfm_set_now_playing(app.state(), profile, on).await;
                });
            }
        }
        true
    }

    /// Demande toutes les 3 s si l'utilisateur a autorisé RustMusic, 10 min au plus.
    pub(crate) fn lastfm_poll(&mut self, profile: i64, round: u64) {
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let start = Instant::now();
                loop {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    if start.elapsed() > Duration::from_secs(600) {
                        return Err("expired".to_string());
                    }
                    match scrobble_command::lastfm_finish(app.state(), profile).await {
                        Ok(None) => continue,
                        other => return other,
                    }
                }
            },
            move |me, r| {
                let s = &mut me.views.settings.scrobble;
                if s.lf_round != round || s.lf_wait.is_none() {
                    return false;
                }
                s.lf_wait = None;
                match r {
                    Ok(_) => {
                        s.lf_key.clear();
                        s.lf_secret.clear();
                        s.lf_own = false;
                        me.settings_scrobble_load();
                    }
                    Err(e) => s.lf_error = Some(translate("lastfm", &["unreachable", "bad_key", "expired", "no_key"], e)),
                }
                true
            },
        );
    }
}
