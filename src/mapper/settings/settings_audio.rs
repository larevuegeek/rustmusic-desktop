use crate::models::settings::*;
use karbon_ui::ui::prelude::Color;
use crate::app::music_app::MusicApp;
use crate::helper::color::hex;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::settings::Control;
use crate::models::settings::Ghost;
use crate::models::settings::GroupAction;
use crate::models::settings::Note;
use crate::models::settings::OptionGroup;
use crate::models::settings::OptionRow;
use crate::models::settings::PanelChoice;
use crate::models::settings::Tile;
use crate::models::settings::SettingsMsg;

/// La couleur de chaque profil de qualité.
pub(crate) fn tone_of(profile: &str) -> &'static str {
    match profile {
        "high" => "green",
        "medium" => "amber",
        "low" => "sky",
        "minimal" => "rose",
        _ => "",
    }
}

/// `-500` de Tailwind (barres) et `-400` (textes, thème sombre).
pub(crate) fn tone_colors(tone: &str) -> Option<(Color, Color)> {
    Some(match tone {
        // Teinte 600 en clair, 400 en sombre.
        "green" => (hex("#22c55e"), hex(if karbon_ui::ui::styling::color_scheme().0 { "#4ade80" } else { "#16a34a" })),
        "amber" => (hex("#f59e0b"), hex(if karbon_ui::ui::styling::color_scheme().0 { "#fbbf24" } else { "#d97706" })),
        "sky" => (hex("#0ea5e9"), hex(if karbon_ui::ui::styling::color_scheme().0 { "#38bdf8" } else { "#0284c7" })),
        "rose" => (hex("#f43f5e"), hex(if karbon_ui::ui::styling::color_scheme().0 { "#fb7185" } else { "#e11d48" })),
        _ => return None,
    })
}

pub(crate) const WINDOWS: bool = cfg!(target_os = "windows");

/// La couleur du mot en gras d'une note : celle du profil, sinon le texte.
pub fn note_color(tone: &str) -> Color {
    tone_colors(tone).map(|(_, txt)| txt).unwrap_or_else(|| match karbon_ui::ui::styling::color_scheme() {
        (true, true) => hex("#ffffff"),
        (true, false) => hex("#f2f5f3"),
        (false, false) => hex("#1a1c1a"),
        (false, true) => hex("#000000"),
    })
}

impl MusicApp {
    /// Les deux tuiles du mode de sortie.
    pub fn output_tiles(&self) -> Vec<Tile> {
        let platform = if cfg!(target_os = "macos") { "macos" } else if cfg!(target_os = "linux") { "linux" } else { "windows" };
        let exclusive = self.data.flag("wasapi_exclusive");
        vec![
            Tile {
                title: t("settings.output_shared"),
                desc: t(&format!("settings.output_shared_desc_{platform}")),
                selected: !exclusive,
                msg: Msg::Settings(SettingsMsg::Exclusive(false)),
            },
            Tile {
                title: t("settings.output_exclusive"),
                desc: t(&format!("settings.output_exclusive_desc_{platform}")),
                selected: exclusive,
                msg: Msg::Settings(SettingsMsg::Exclusive(true)),
            },
        ]
    }

    pub(crate) fn panel(&self, options: Vec<(&'static str, String, &'static str, u8, &'static str)>, current: Option<&str>, msg: fn(&'static str) -> SettingsMsg) -> Vec<PanelChoice> {
        options
            .into_iter()
            .map(|(v, label, icon, level, tone)| {
                let active = current == Some(v);
                PanelChoice { label, icon, level, tone, active, msg: Msg::Settings(msg(v)) }
            })
            .collect()
    }

    /// Les choix de la qualité de décodage.
    pub fn quality_panel(&self) -> Vec<PanelChoice> {
        let s = self.views.settings.audio.status.as_ref();
        let kept = s.map(|s| tone_of(&s.resolved)).unwrap_or("");
        self.panel(
            vec![
                ("auto", t("settings.audio_quality_auto"), "material-symbols:check-circle-outline-rounded", 0, kept),
                ("high", t("settings.audio_quality_high_short"), "", 4, "green"),
                ("medium", t("settings.audio_quality_medium_short"), "", 3, "amber"),
                ("low", t("settings.audio_quality_low_short"), "", 2, "sky"),
                ("minimal", t("settings.audio_quality_minimal_short"), "", 1, "rose"),
            ],
            s.map(|s| s.setting.as_str()),
            SettingsMsg::Quality,
        )
    }

    /// Le profil retenu par « Auto », ou ce que fait le profil choisi.
    pub fn quality_note(&self) -> Option<Note> {
        let s = self.views.settings.audio.status.as_ref()?;
        if s.setting == "auto" {
            let host = match &s.virt {
                Some(k) => t("settings.audio_quality_host_vm_short").replace("{kind}", k),
                None => t("settings.audio_quality_host_native_short"),
            };
            let text = t("settings.audio_quality_now");
            let (before, after) = text.split_once("{profile}").map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or((text, String::new()));
            Some(Note {
                before,
                bold: Some(t(&format!("settings.audio_quality_{}", s.resolved))),
                after,
                tone: tone_of(&s.resolved),
                hint: Some(t("settings.audio_quality_auto_reason").replace("{n}", &s.cores.to_string()).replace("{host}", &host)),
            })
        } else {
            Some(Note {
                before: t(&format!("settings.audio_quality_{}_summary", s.setting)),
                bold: None,
                after: String::new(),
                tone: "",
                hint: Some(t(&format!("settings.audio_quality_{}_advice", s.setting))),
            })
        }
    }

    /// Les choix du Replay Gain.
    pub fn replay_gain_panel(&self) -> Vec<PanelChoice> {
        let s = self.views.settings.audio.status.as_ref();
        self.panel(
            vec![
                ("off", t("settings.replay_gain_off"), "material-symbols:do-not-disturb-on-outline-rounded", 0, ""),
                ("track", t("settings.replay_gain_track"), "material-symbols:music-note-rounded", 0, ""),
                ("album", t("settings.replay_gain_album"), "material-symbols:album-outline-rounded", 0, ""),
            ],
            s.map(|s| s.rg_mode.as_str()),
            SettingsMsg::ReplayGain,
        )
    }

    pub fn replay_gain_note(&self) -> Option<Note> {
        let s = self.views.settings.audio.status.as_ref()?;
        Some(Note {
            before: t(&format!("settings.replay_gain_{}_summary", s.rg_mode)),
            bold: None,
            after: String::new(),
            tone: "",
            hint: (s.rg_mode != "off").then(|| format!("{} {}", t(&format!("settings.replay_gain_{}_advice", s.rg_mode)), t("settings.replay_gain_note"))),
        })
    }

    /// Le préampli affiché (brouillon pendant le glissement).
    pub fn preamp(&self) -> f32 {
        self.views.settings.audio.preamp.or(self.views.settings.audio.status.as_ref().map(|s| s.preamp)).unwrap_or(0.0)
    }

    pub fn preamp_label(&self) -> String {
        let p = self.preamp();
        format!("{}{:.1} dB", if p > 0.0 { "+" } else { "" }, p)
    }

    pub(crate) fn settings_audio(&self) -> Vec<OptionGroup> {
        let exclusive = self.data.flag("wasapi_exclusive");
        let dop = self.data.flag("dsd_dop");
        let dop_available = !WINDOWS || exclusive;
        let names = self.audio_outputs().into_iter().map(|o| o.name).collect::<Vec<_>>().join(" ");
        let mut mode = vec![
            self.keep(
                OptionRow::block("output_mode", Control::Block("output-mode")),
                &format!("{} {} {} wasapi alsa bit-perfect", t("settings.output_mode"), t("settings.output_shared"), t("settings.output_exclusive")),
            ),
            self.keep(
                OptionRow::new(
                    "dsd_dop",
                    t("settings.dsd_dop"),
                    if dop_available { t("settings.dsd_dop_desc") } else { format!("{} {}", t("settings.dsd_dop_desc"), t("settings.dsd_dop_needs_exclusive")) },
                    Control::Toggle(crate::mapper::settings::toggle_msg("dsd_dop"), dop),
                )
                .disabled(!dop_available),
                "dsd dop",
            ),
        ];
        if WINDOWS {
            mode.push(self.keep(OptionRow::block("dac_test", Control::Block("dac-test")), &format!("{} wasapi", t("settings.dac_test"))));
        }
        let qualities = ["auto", "high", "medium", "low", "minimal"].map(|q| t(&format!("settings.audio_quality_{}", if q == "auto" { q.to_string() } else { format!("{q}_short") }))).join(" ");
        let rg_active = self.views.settings.audio.status.as_ref().is_some_and(|s| s.rg_mode != "off");
        let mut rg = vec![self.keep(
            OptionRow::block("replay_gain", Control::Block("replay-gain")),
            &format!("{} {} {} {} volume", t("settings.replay_gain"), t("settings.replay_gain_off"), t("settings.replay_gain_track"), t("settings.replay_gain_album")),
        )];
        if rg_active {
            rg.push(self.keep(
                OptionRow::new("replay_gain_preamp", t("settings.replay_gain_preamp"), t("settings.replay_gain_preamp_desc"), Control::Custom("preamp")),
                "preamp gain db",
            ));
        }
        vec![
            self.option_group(
                "output",
                Some(t("settings.output_group")),
                Some(t("settings.output_group_hint")),
                Some("settings.output_group"),
                vec![self.keep(OptionRow::block("outputs", Control::Block("audio-outputs")), &format!("{} DAC {names}", t("settings.output_group")))],
            )
            .with_action(GroupAction::Button(
                Ghost::new("material-symbols:sync-rounded", t("settings.audio_devices_refresh"), Msg::Settings(SettingsMsg::RefreshDevices))
                    .title(t("settings.audio_devices_refresh_hint"))
                    .busy(self.views.settings.audio.refreshing),
            )),
            self.option_group("output_mode", Some(t("settings.output_mode")), Some(t("settings.output_mode_hint")), Some("settings.output_mode"), mode)
                .with_action(GroupAction::Badge(t("settings.beta_badge"), "amber")),
            self.option_group(
                "quality",
                Some(t("settings.audio_quality")),
                Some(t("settings.audio_quality_hint")),
                Some("settings.audio_quality"),
                vec![self.keep(OptionRow::block("quality", Control::Block("audio-quality")), &format!("{} {qualities} dsd cpu", t("settings.audio_quality")))],
            ),
            self.option_group("replay_gain", Some(t("settings.replay_gain")), Some(t("settings.replay_gain_hint")), Some("settings.replay_gain"), rg),
        ]
    }
}

impl PanelChoice {
    /// Les couleurs d'une case, active ou non, selon sa teinte.
    pub fn look(&self) -> PanelLook {
        let tone = tone_colors(self.tone);
        // Jetons --rg-s2, --rg-bd2, --rg-tx, --rg-mu, --rg-mu2, --rg-g et --rg-off selon le
        // thème et le contraste (voir assets/css/theme-modes.css).
        let (dark, high) = karbon_ui::ui::styling::color_scheme();
        let (s2, bd2, tx, mu) = match (dark, high) {
            (true, true) => ("#1b211e", "#2e3632", "#ffffff", "#b0b9b4"),
            (true, false) => ("#1b211e", "#2e3632", "#f2f5f3", "#8b948f"),
            (false, false) => ("#f1eee8", "#cfc9bd", "#1a1c1a", "#5e625d"),
            (false, true) => ("#f1eee8", "#cfc9bd", "#000000", "#3a433e"),
        };
        let (background, border, text) = match (self.active, tone) {
            (true, Some((bar, txt))) => (Color { alpha: 31, ..bar }, Color { alpha: 115, ..bar }, txt),
            (true, None) => (hex(s2), hex(bd2), hex(tx)),
            (false, _) => (Color { alpha: 0, ..hex("#000000") }, Color { alpha: 0, ..hex("#000000") }, hex(mu)),
        };
        let (accent, mu2, off) = match (dark, high) {
            (true, true) => ("#22c55e", "#9aa39e", "#2a312d"),
            (true, false) => ("#22c55e", "#6b746f", "#2a312d"),
            (false, false) => ("#16a34a", "#6f736d", "#d3cdc1"),
            (false, true) => ("#16a34a", "#4a534e", "#d3cdc1"),
        };
        let tint = match tone {
            Some((bar, _)) if self.active => bar,
            Some((bar, _)) => Color { alpha: 204, ..bar },
            None if self.active => hex(accent),
            None => hex(mu2),
        };
        let bars = [0u8, 1, 2, 3].map(|i| if i >= self.level { hex(off) } else { tint });
        PanelLook { background, border, text, icon: tone.map(|(_, txt)| txt), bars }
    }
}
