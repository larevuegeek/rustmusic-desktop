use crate::models::settings::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::settings::Control;
use crate::models::settings::OptionGroup;
use crate::models::settings::OptionRow;
use crate::models::settings::SettingsMsg;

/// Action, libellé, groupe.
pub const ACTIONS: [(&str, &str, &str); 10] = [
    ("playPause", "settings.shortcut_play_pause", "playback"),
    ("seekForward", "settings.shortcut_forward", "playback"),
    ("seekBackward", "settings.shortcut_backward", "playback"),
    ("next", "settings.shortcut_next", "playback"),
    ("previous", "settings.shortcut_previous", "playback"),
    ("volumeUp", "settings.shortcut_volume_up", "volume"),
    ("volumeDown", "settings.shortcut_volume_down", "volume"),
    ("mute", "settings.shortcut_mute", "volume"),
    ("search", "settings.shortcut_search", "navigation"),
    ("toggleMini", "settings.shortcut_toggle_mini", "navigation"),
];

pub(crate) fn defaults(action: &str) -> &'static [&'static str] {
    match action {
        "playPause" => &["Space"],
        "seekForward" => &["ArrowRight"],
        "seekBackward" => &["ArrowLeft"],
        "next" => &["Mod+ArrowRight"],
        "previous" => &["Mod+ArrowLeft"],
        "volumeUp" => &["ArrowUp"],
        "volumeDown" => &["ArrowDown"],
        "mute" => &["M"],
        "search" => &["Mod+K", "Mod+F"],
        _ => &["Mod+Shift+M"],
    }
}

/// « Mod+Shift+M » en touches.
pub(crate) fn key_caps(binding: &str) -> Vec<KeyCap> {
    let mac = cfg!(target_os = "macos");
    binding
        .split('+')
        .map(|part| {
            let arrow = match part {
                "ArrowRight" => Some("material-symbols:arrow-forward-rounded"),
                "ArrowLeft" => Some("material-symbols:arrow-back-rounded"),
                "ArrowUp" => Some("material-symbols:arrow-upward-rounded"),
                "ArrowDown" => Some("material-symbols:arrow-downward-rounded"),
                _ => None,
            };
            let label = match part {
                "Mod" if mac => "⌘".into(),
                "Mod" => "Ctrl".into(),
                "Alt" if mac => "⌥".into(),
                "Shift" if mac => "⇧".into(),
                "Shift" => t("settings.key_shift"),
                "Space" => t("settings.key_space"),
                p => p.to_string(),
            };
            KeyCap { label, arrow }
        })
        .collect()
}

/// Des libellés simples en touches (`combo`).
pub fn caps_of(labels: &[String]) -> Vec<KeyCap> {
    labels.iter().map(|l| KeyCap { label: l.clone(), arrow: None }).collect()
}

impl MusicApp {
    /// Les actions modifiées et leurs touches.
    pub(crate) fn shortcut_overrides(&self) -> serde_json::Map<String, serde_json::Value> {
        self.data
            .setting("shortcuts")
            .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default()
    }

    /// Les combinaisons d'une action, en touches.
    pub fn shortcut_combos(&self, action: &str) -> Vec<Vec<KeyCap>> {
        let modified_items = self.shortcut_overrides();
        let bindings: Vec<String> = match modified_items.get(action).and_then(|v| v.as_array()) {
            Some(a) => a.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
            None => defaults(action).iter().map(|s| s.to_string()).collect(),
        };
        bindings.iter().map(|b| key_caps(b)).collect()
    }

    pub fn shortcut_overridden(&self, action: &str) -> bool {
        self.shortcut_overrides().contains_key(action)
    }

    pub fn shortcut_editing(&self, action: &str) -> bool {
        self.views.settings.shortcut_editing.as_deref() == Some(action)
    }

    pub(crate) fn settings_shortcuts(&self) -> Vec<OptionGroup> {
        let groups = [
            ("playback", "settings.group_playback", "settings.shortcuts_playback_hint"),
            ("volume", "settings.shortcuts_volume", "settings.shortcuts_volume_hint"),
            ("navigation", "settings.shortcuts_navigation", "settings.shortcuts_navigation_hint"),
        ];
        let mut out: Vec<OptionGroup> = groups
            .iter()
            .map(|(g, title, hint)| {
                let mut lines: Vec<Option<OptionRow>> = ACTIONS
                    .iter()
                    .filter(|a| a.2 == *g)
                    .map(|(action, label, _)| {
                        let desc = if self.shortcut_editing(action) { t("settings.shortcut_press") } else { String::new() };
                        let words = self.shortcut_combos(action).iter().map(|c| c.iter().map(|k| k.label.clone()).collect::<Vec<_>>().join("+")).collect::<Vec<_>>().join(" ");
                        self.keep(OptionRow::new(action, t(label), desc, Control::Custom("shortcut")), &words)
                    })
                    .collect();
                if *g == "navigation" {
                    lines.push(self.keep(OptionRow::new("range", t("settings.shortcut_range"), t("settings.shortcut_range_desc"), Control::Custom("shortcut-range")), ""));
                    lines.push(self.keep(OptionRow::new("close", t("settings.shortcut_close"), t("settings.shortcut_close_desc"), Control::Custom("shortcut-close")), "escape echap"));
                }
                self.option_group(g, Some(t(title)), Some(t(hint)), Some(title), lines)
            })
            .collect();
        let customized = !self.shortcut_overrides().is_empty();
        out.push(self.option_group(
            "custom",
            Some(t("settings.shortcuts_custom")),
            Some(t("settings.shortcuts_custom_hint")),
            Some("settings.shortcuts_custom"),
            vec![self.keep(
                OptionRow::new("reset_all", t("settings.shortcuts_reset_all"), t("settings.shortcuts_reset_all_desc"), Control::Custom("shortcut-reset-all")).disabled(!customized),
                "",
            )],
        ));
        let active_media = self.setting_or_default("system_media_controls") == "true";
        let mut media = Vec::new();
        if !active_media {
            media.push(self.keep(OptionRow::new("media_off", t("settings.shortcuts_media_off"), t("settings.shortcuts_media_off_desc"), Control::Custom("media-enable")), ""));
        }
        for (key, icon) in [
            ("settings.shortcut_play_pause", "material-symbols:play-pause-rounded"),
            ("settings.shortcut_next", "material-symbols:skip-next-rounded"),
            ("settings.shortcut_previous", "material-symbols:skip-previous-rounded"),
        ] {
            media.push(self.keep(OptionRow::new(icon, t(key), String::new(), Control::Custom("media-key")), "media multimedia"));
        }
        out.push(self.option_group("media", Some(t("settings.shortcuts_media")), Some(t("settings.shortcuts_media_hint")), Some("settings.shortcuts_media"), media));
        out
    }

    /// `ShortcutEdit` pour une action de la table.
    pub fn shortcut_edit_msg(&self, action: &str) -> Msg {
        let a = ACTIONS.iter().find(|a| a.0 == action).map(|a| a.0).unwrap_or("playPause");
        Msg::Settings(SettingsMsg::ShortcutEdit(a))
    }

    pub fn shortcut_reset_msg(&self, action: &str) -> Msg {
        let a = ACTIONS.iter().find(|a| a.0 == action).map(|a| a.0).unwrap_or("playPause");
        Msg::Settings(SettingsMsg::ShortcutReset(a))
    }
}
