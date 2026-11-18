use crate::app::music_app::MusicApp;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Choice;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::mapper::radios::{country_name, COUNTRY_CODES};
use crate::mapper::settings::segments;
use crate::mapper::settings::set_msg;
use crate::mapper::settings::toggle_msg;
use crate::models::settings::Control;
use crate::models::settings::Ghost;
use crate::models::settings::GroupAction;
use crate::models::settings::OptionGroup;
use crate::models::settings::OptionRow;
use crate::models::settings::SettingsMsg;

/// Vitesses de lecture proposées.
pub(crate) const SPEEDS: [&str; 6] = ["0.75", "1", "1.25", "1.5", "1.75", "2"];

impl MusicApp {
    /// Un `MenuSelect` qui écrit `key` : ancre, libellé courant, options.
    pub(crate) fn menu_of(&self, anchor: &'static str, key: &'static str, options: Vec<(String, String, String)>) -> Control {
        let current_item = self.setting_or_default(key);
        let choice: Vec<Choice> = options
            .into_iter()
            .map(|(v, label, icon)| Choice { label, icon, active: v == current_item, msg: set_msg(key, &v), icon_only: false })
            .collect();
        let label = choice.iter().find(|c| c.active).or(choice.first()).map(|c| c.label.clone()).unwrap_or_default();
        Control::Menu(anchor, label, choice)
    }

    pub(crate) fn settings_online(&self) -> Vec<OptionGroup> {
        let podcasts = self.data.flag("podcasts");
        let radios = self.data.flag("radios");
        let mut pod = vec![self.keep(
            OptionRow::new("podcasts", t("podcasts.toggle"), t("podcasts.toggle_desc"), Control::Toggle(toggle_msg("podcasts"), podcasts)),
            "podcast podcasts rss flux épisodes abonnements apple",
        )];
        if podcasts {
            let fr = karbon_ui::intl::locale().tag() == "fr";
            let auto = match karbon_ui::intl::locale().tag() {
                "en" => "US",
                "de" => "DE",
                "es" => "ES",
                "it" => "IT",
                _ => "FR",
            };
            let name = country_name;
            let mut country = vec![(String::new(), t_with("podcasts.country_auto", &[("country", &name(auto))]), crate::mapper::radios::flag(auto).to_string())];
            country.extend(COUNTRY_CODES.iter().map(|c| (c.to_string(), country_name(c), crate::mapper::radios::flag(c).to_string())));
            let speeds = SPEEDS
                .iter()
                .map(|v| (v.to_string(), format!("{}×", if fr { v.replace('.', ",") } else { v.to_string() }), String::new()))
                .collect();
            let list = |keys: &[&str], prefix: &str| keys.iter().map(|v| (v.to_string(), t(&format!("{prefix}{v}")), String::new())).collect::<Vec<_>>();
            let size = self.views.settings.downloads_size.filter(|s| *s > 0).map(|s| karbon_ui::intl::bytes(s as u64)).unwrap_or_default();
            pod.extend([
                self.keep(
                    OptionRow::new(
                        "podcast_play_mode",
                        t("podcasts.play_mode"),
                        t("podcasts.play_mode_desc"),
                        Control::Segmented(
                            segments(
                                "podcast_play_mode",
                                &self.setting_or_default("podcast_play_mode"),
                                &[("stream", t("podcasts.play_mode_stream")), ("download", t("podcasts.play_mode_download"))],
                            ),
                            true,
                        ),
                    ),
                    "podcast lecture téléchargement streaming",
                ),
                self.keep(
                    OptionRow::new("podcast_country", t("podcasts.country"), t("podcasts.country_desc"), self.menu_of("online.country", "podcast_country", country)),
                    "podcast pays country classement annuaire langue",
                ),
                self.keep(
                    OptionRow::new("podcast_speed", t("podcasts.speed_default"), t("podcasts.speed_default_desc"), self.menu_of("online.speed", "podcast_speed", speeds)),
                    "podcast vitesse lecture rapide accélérer",
                ),
                self.keep(
                    OptionRow::new(
                        "podcast_refresh",
                        t("podcasts.refresh"),
                        t("podcasts.refresh_desc"),
                        self.menu_of("online.refresh", "podcast_refresh", list(&["never", "1h", "6h", "24h"], "podcasts.refresh_")),
                    ),
                    "podcast vérifier nouveaux épisodes actualiser automatique",
                ),
                self.keep(
                    OptionRow::new(
                        "podcast_cleanup",
                        t("podcasts.cleanup"),
                        t("podcasts.cleanup_desc"),
                        self.menu_of("online.cleanup", "podcast_cleanup", list(&["never", "now", "24h", "7d", "30d"], "podcasts.cleanup_")),
                    ),
                    "podcast ménage effacer écoutés téléchargements espace",
                ),
                self.keep(
                    OptionRow::new(
                        "opml",
                        t("podcasts.opml"),
                        t("podcasts.opml_desc"),
                        Control::Buttons(vec![
                            Ghost::new("material-symbols:upload-file-outline-rounded", t("podcasts.opml_import_short"), Msg::Noop),
                            Ghost::new("material-symbols:download-rounded", t("podcasts.opml_export_short"), Msg::Noop),
                        ]),
                    ),
                    "podcast opml import export abonnements sauvegarde",
                ),
                self.keep(
                    OptionRow::new(
                        "podcast_history",
                        t("podcasts.clear_history"),
                        t("podcasts.clear_history_desc"),
                        Control::Buttons(vec![Ghost::new(
                            "material-symbols:history-off-rounded",
                            t("podcasts.clear_history_short"),
                            Msg::Settings(SettingsMsg::ClearHistory("podcasts", false)),
                        )
                        .danger()]),
                    ),
                    "podcast historique écoute effacer reprise écoutés",
                ),
                self.keep(
                    OptionRow::new(
                        "podcast_downloads",
                        t("podcasts.downloads"),
                        t("podcasts.space_used").replace("{size}", if size.is_empty() { "0" } else { &size }),
                        Control::Buttons(vec![Ghost::new(
                            "material-symbols:download-for-offline-outline-rounded",
                            t("podcasts.manage_downloads"),
                            Msg::Go(Route::PodcastDownloads),
                        )]),
                    ),
                    "podcast téléchargements cache espace disque vider supprimer",
                ),
                self.keep(OptionRow::new("podcast_network", t("podcasts.network_title"), t("podcasts.network_desc"), Control::Info("material-symbols:public")), ""),
            ]);
        }
        let mut rad = vec![self.keep(
            OptionRow::new("radios", t("radios.toggle"), t("radios.toggle_desc"), Control::Toggle(toggle_msg("radios"), radios)),
            "radio radios webradio direct flux stream",
        )];
        if radios {
            rad.push(self.keep(
                OptionRow::new(
                    "radio_history",
                    t("radios.clear_history"),
                    t("radios.clear_history_desc"),
                    Control::Buttons(vec![Ghost::new(
                        "material-symbols:history-off-rounded",
                        t("radios.clear_history_short"),
                        Msg::Settings(SettingsMsg::ClearHistory("radios", false)),
                    )
                    .danger()]),
                ),
                "radio historique dernières écoutées titres effacer",
            ));
            rad.push(self.keep(OptionRow::new("radio_network", t("radios.network_title"), t("radios.network_desc"), Control::Info("material-symbols:public")), ""));
        }
        let mut groups = vec![
            self.option_group("podcasts", Some(t("podcasts.settings_group")), Some(t("podcasts.settings_group_hint")), Some("online-podcasts"), pod)
                .with_action(GroupAction::Badge(t("settings.beta_badge"), "amber")),
            self.option_group("radios", Some(t("radios.settings_group")), Some(t("radios.settings_group_hint")), Some("online-radios"), rad)
                .with_action(GroupAction::Badge(t("settings.beta_badge"), "amber")),
        ];
        groups.extend(self.settings_scrobble());
        groups
    }
}
