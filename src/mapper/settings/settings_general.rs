use crate::app::music_app::MusicApp;
use karbon_ui::intl::{t, t_with};
use crate::mapper::settings::matches;
use crate::models::settings::Control;
use crate::models::settings::OptionGroup;
use crate::models::settings::OptionRow;

impl MusicApp {
    pub(crate) fn settings_general(&self) -> Vec<OptionGroup> {
        let system = if cfg!(target_os = "macos") {
            "macOS"
        } else if cfg!(target_os = "linux") {
            "Linux"
        } else {
            "Windows"
        };
        let txt = |key: &str| t(&format!("settings.{key}")).replace("{os}", system);
        let search = &self.views.settings.search;
        let language = OptionRow::new(
            "language",
            t("settings.language"),
            t("settings.language_desc"),
            Control::Select(
                "language",
                self.setting_or_default("language"),
                [("fr", "Français"), ("en", "English"), ("es", "Español"), ("de", "Deutsch"), ("it", "Italiano")]
                    .into_iter()
                    .map(|(v, l)| (v.to_string(), l.to_string()))
                    .collect(),
            ),
        );
        let mut thresholds = vec![("auto".to_string(), t("settings.listen_threshold_auto"))];
        thresholds.extend(["25", "50", "75", "80", "90"].map(|n| (n.to_string(), t_with("settings.listen_threshold_pct", &[("n", n)]))));
        let threshold = OptionRow::new(
            "listen_threshold",
            t("settings.listen_threshold"),
            t("settings.listen_threshold_desc"),
            Control::Select("listen_threshold", self.setting_or_default("listen_threshold"), thresholds),
        );
        let row = |key: &'static str, title: &str, desc: &str, words: &str| self.toggle_row(key, txt(title), txt(desc), words);
        vec![
            self.option_group(
                "language",
                None,
                None,
                None,
                vec![matches(search, &language.title, &language.desc, "language langue").then_some(language)],
            ),
            self.option_group(
                "startup",
                Some(txt("group_startup")),
                Some(txt("group_startup_hint")),
                Some("general-0"),
                vec![
                    row("auto_start", "autostart", "autostart_desc", ""),
                    row("check_updates", "check_updates", "check_updates_desc", "mise à jour update version"),
                    row("minimize_to_tray", "tray", "tray_desc", "tray"),
                    row("scan_on_startup", "scan_on_startup", "scan_on_startup_desc", "scan"),
                    row("play_on_start", "play_on_start", "play_on_start_desc", "lecture automatique autoplay /play"),
                ],
            ),
            self.option_group(
                "playback",
                Some(txt("group_playback")),
                Some(txt("group_playback_hint")),
                Some("general-1"),
                vec![
                    row("single_click_play", "single_click_play", "single_click_play_desc", ""),
                    row("resume_playback", "resume_playback", "resume_playback_desc", ""),
                    row("gapless", "gapless", "gapless_desc", "gapless"),
                    row("prevent_sleep", "prevent_sleep", "prevent_sleep_desc", "veille sleep suspend"),
                    matches(search, &threshold.title, &threshold.desc, "écoute compteur pourcentage play count").then_some(threshold),
                ],
            ),
            self.option_group(
                "system",
                Some(txt("group_system")),
                Some(txt("group_system_hint")),
                Some("general-2"),
                vec![
                    row("system_media_controls", "system_media_controls", "system_media_controls_desc", "smtc mpris"),
                    row("show_notifications", "notifications", "notifications_desc", ""),
                    row("show_sleep_timer", "show_sleep_timer", "show_sleep_timer_desc", ""),
                    row("show_whats_new_button", "show_whats_new_button", "show_whats_new_button_desc", "nouveautés mégaphone annonces"),
                    row("show_whats_new_at_startup", "show_whats_new_at_startup", "show_whats_new_at_startup_desc", "nouveautés annonces démarrage lancement"),
                    row("show_theme_toggle", "show_theme_toggle", "show_theme_toggle_desc", "thème clair sombre"),
                ],
            ),
        ]
    }
}
