use crate::models::settings::*;
use karbon_ui::intl;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::settings::Control;
use crate::models::settings::OptionGroup;
use crate::models::settings::OptionRow;
use crate::models::settings::SettingsMsg;

pub(crate) fn account(s: crate::service::scrobble::scrobble_service::ScrobbleStatus) -> Account {
    Account { user: s.user_name, now_playing: s.now_playing, pending: s.pending, own_key: s.own_key }
}

/// « 3 écoute(s) en attente d'envoi » ou « Toutes les écoutes sont envoyées ».
pub fn pending_label(a: &Account) -> String {
    if a.pending > 0 {
        t("scrobble.pending").replace("{n}", &intl::number(a.pending))
    } else {
        t("scrobble.up_to_date")
    }
}

pub(crate) fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

impl ScrobbleState {
    /// Sans clé intégrée, la clé personnelle est obligatoire.
    pub fn lf_with_key(&self) -> bool {
        self.lf_own || !self.lf_builtin
    }

    pub fn lf_ready(&self) -> bool {
        !self.lf_with_key() || (!self.lf_key.trim().is_empty() && !self.lf_secret.trim().is_empty())
    }
}

impl MusicApp {
    pub(crate) fn profile_id(&self) -> Option<i64> {
        self.data.profile.as_ref().map(|p| p.id)
    }

    pub(crate) fn settings_scrobble(&self) -> Vec<OptionGroup> {
        let s = &self.views.settings.scrobble;
        let sm = |m: ScrobbleMsg| Msg::Settings(SettingsMsg::Scrobble(m));
        let mut lb = Vec::new();
        match &s.lb {
            Some(a) => {
                lb.push(self.keep(OptionRow::block("lb_account", Control::Block("lb-account")), "listenbrainz scrobble écoutes historique compte"));
                lb.push(self.keep(
                    OptionRow::new("lb_now", t("scrobble.now_playing"), t("scrobble.now_playing_desc"), Control::Toggle(sm(ScrobbleMsg::LbNowPlaying), a.now_playing)),
                    "listenbrainz en cours",
                ));
            }
            None => lb.push(self.keep(OptionRow::block("lb_connect", Control::Block("lb-connect")), "listenbrainz scrobble écoutes historique compte jeton")),
        }
        lb.push(self.keep(OptionRow::new("lb_privacy", t("scrobble.privacy_title"), t("scrobble.privacy_desc"), Control::Info("material-symbols:public")), ""));
        let mut lf = Vec::new();
        match (&s.lf, &s.lf_wait) {
            (Some(a), _) => {
                lf.push(self.keep(OptionRow::block("lf_account", Control::Block("lf-account")), "last.fm lastfm scrobble écoutes historique compte"));
                lf.push(self.keep(
                    OptionRow::new("lf_now", t("scrobble.now_playing"), t("lastfm.now_playing_desc"), Control::Toggle(sm(ScrobbleMsg::LfNowPlaying), a.now_playing)),
                    "last.fm en cours",
                ));
            }
            (None, Some(_)) => lf.push(self.keep(OptionRow::block("lf_wait", Control::Block("lf-wait")), "last.fm lastfm autoriser")),
            (None, None) => lf.push(self.keep(OptionRow::block("lf_connect", Control::Block("lf-connect")), "last.fm lastfm scrobble écoutes historique compte clé api")),
        }
        vec![
            self.option_group("listenbrainz", Some("ListenBrainz".into()), Some(t("scrobble.group_hint")), Some("online-listenbrainz"), lb),
            self.option_group("lastfm", Some("Last.fm".into()), Some(t("lastfm.group_hint")), Some("online-lastfm"), lf),
        ]
    }

    pub fn scrobble_account(&self, lastfm: bool) -> Option<AccountView> {
        let s = &self.views.settings.scrobble;
        let (a, confirm) = if lastfm { (s.lf.as_ref()?, s.lf_confirm) } else { (s.lb.as_ref()?, s.lb_confirm) };
        let sm = |m: ScrobbleMsg| Msg::Settings(SettingsMsg::Scrobble(m));
        let warning = confirm.then(|| {
            if a.pending > 0 {
                t("scrobble.disconnect_pending").replace("{n}", &intl::number(a.pending))
            } else {
                t(if lastfm { "lastfm.disconnect_kept" } else { "scrobble.disconnect_kept" })
            }
        });
        let mut sub = pending_label(a);
        if lastfm && a.own_key {
            sub = format!("{sub} · {}", t("lastfm.own_key_used"));
        }
        let user = urlencode(&a.user);
        let profile = sm(ScrobbleMsg::Open(if lastfm { format!("https://www.last.fm/user/{user}") } else { format!("https://listenbrainz.org/user/{user}/") }));
        let mut buttons = Vec::new();
        if confirm {
            buttons.push(crate::models::settings::Ghost::new("", t("common.cancel"), sm(if lastfm { ScrobbleMsg::LfKeep } else { ScrobbleMsg::LbKeep })));
        }
        let quit = crate::models::settings::Ghost::new("material-symbols:link-off-rounded", t("scrobble.disconnect"), sm(if lastfm { ScrobbleMsg::LfDisconnect } else { ScrobbleMsg::LbDisconnect }));
        buttons.push(if confirm { quit.danger() } else { quit });
        Some(AccountView { lastfm, user: t("scrobble.connected_as").replace("{user}", &a.user), warning, sub, profile, buttons })
    }
}
