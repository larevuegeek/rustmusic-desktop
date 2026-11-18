use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::mapper::settings::toggle_msg;
use crate::models::settings::Control;
use crate::models::settings::GroupAction;
use crate::models::settings::OptionGroup;
use crate::models::settings::OptionRow;
use crate::models::settings::SettingsMsg;

impl MusicApp {
    pub(crate) fn settings_network(&self) -> Vec<OptionGroup> {
        let d = &self.views.settings.dlna;
        let mut dlna = vec![self.keep(
            OptionRow::new("dlna_share", t("settings.dlna_share"), t("settings.dlna_enabled_desc"), Control::Toggle(Msg::Settings(SettingsMsg::DlnaToggle), d.running))
                .disabled(d.toggling),
            "dlna upnp sonos",
        )];
        if d.running && d.url.is_some() {
            dlna.push(self.keep(OptionRow::block("dlna_active", Control::Block("dlna-active")), &format!("{} url adresse", t("settings.dlna_active"))));
        }
        if d.loaded {
            dlna.push(self.keep(OptionRow::new("dlna_name", t("settings.dlna_friendly_name"), t("settings.dlna_friendly_name_desc"), Control::Custom("dlna-name")), ""));
            dlna.push(self.keep(OptionRow::new("dlna_port", t("settings.dlna_port"), t("settings.dlna_port_desc"), Control::Custom("dlna-port")), "http"));
        }
        let players = |key: &'static str, title: &str, desc: &str, words: &str| {
            self.keep(OptionRow::new(key, t(title), t(desc), Control::Toggle(toggle_msg(key), self.data.flag(key))), words)
        };
        vec![
            self.option_group(
                "network_player",
                Some(t("network_player.group")),
                Some(t("network_player.group_hint")),
                Some("network_player.group"),
                vec![
                    players("network_players", "network_player.toggle", "network_player.toggle_desc", "wiim upnp dlna ampli renderer cast"),
                    players("airplay", "network_player.airplay_toggle", "network_player.airplay_toggle_desc", "airplay apple raop onkyo denon"),
                ],
            )
            .with_action(GroupAction::Badge(t("settings.beta_badge"), "amber")),
            self.option_group("dlna", Some(t("settings.dlna_group")), Some(t("settings.dlna_group_hint")), Some("settings.dlna_group"), dlna),
            self.option_group(
                "online",
                Some(t("settings.online_group")),
                Some(t("settings.online_group_hint")),
                Some("settings.online_group"),
                vec![self.toggle_row(
                    "auto_download_artist_images",
                    t("settings.auto_download_artist_images"),
                    t("settings.auto_download_artist_images_desc"),
                    "deezer portrait",
                )],
            ),
        ]
    }
}
