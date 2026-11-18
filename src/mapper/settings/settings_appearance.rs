use crate::models::settings::*;
use std::sync::OnceLock;
use crate::app::music_app::MusicApp;
use karbon_ui::ui::prelude::Color;
use crate::helper::color::hex;
use karbon_ui::intl::t;
use crate::mapper::settings::segments;
use crate::mapper::settings::set_msg;
use crate::mapper::settings::toggle_msg;
use crate::models::settings::Card;
use crate::models::settings::Control;
use crate::models::settings::OptionGroup;
use crate::models::settings::OptionRow;
use crate::models::settings::Pill;
use crate::models::settings::Preview;
use crate::models::settings::Swatch;
use crate::models::app::Msg;
use crate::models::settings::SettingsMsg;

/// Couleur et libellé ; la première est celle d'origine.
pub const ACCENTS: [(&str, &str); 10] = [
    ("#22c55e", "settings.accent_green"),
    ("#14b8a6", "settings.accent_teal"),
    ("#06b6d4", "settings.accent_cyan"),
    ("#3b82f6", "settings.accent_blue"),
    ("#6366f1", "settings.accent_indigo"),
    ("#a855f7", "settings.accent_purple"),
    ("#ec4899", "settings.accent_pink"),
    ("#ef4444", "settings.accent_red"),
    ("#f97316", "settings.accent_orange"),
    ("#f59e0b", "settings.accent_amber"),
];

/// Clé et libellé, dans l'ordre de référence.
pub const LIBRARY_TABS: [(&str, &str); 8] = [
    ("tracks", "library.tracks"),
    ("artists", "library.artists"),
    ("albums", "library.albums"),
    ("genres", "library.genres"),
    ("years", "library.years"),
    ("folders", "nav.folders"),
    ("mixes", "library.mixes"),
    ("playlists", "nav.playlists"),
];

/// `palettes` de `ThemePreview` : clair ou sombre.
pub fn theme_palette(light: bool) -> Palette {
    if light {
        Palette { sb: hex("#efece5"), ct: hex("#f6f4ef"), pl: hex("#ffffff"), plbd: hex("#e2ddd3"), bar: hex("#d8d3c8"), hero: Color::oklch(0.9, 0.04, 25.0, 1.0) }
    } else {
        Palette { sb: hex("#0f1211"), ct: hex("#0b0d0c"), pl: hex("#141816"), plbd: hex("#222222"), bar: hex("#2a312d"), hero: Color::oklch(0.32, 0.07, 25.0, 1.0) }
    }
}

/// « #rrggbb ».
pub(crate) fn valid_accent(v: &str) -> bool {
    v.len() == 7 && v.starts_with('#') && v[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Calculé une fois par lancement.
pub(crate) fn light_effects() -> &'static (bool, Option<String>) {
    static AUTO: OnceLock<(bool, Option<String>)> = OnceLock::new();
    AUTO.get_or_init(|| {
        let r = crate::commands::app::system_command::get_light_effects_auto();
        (r.light, r.reason)
    })
}

impl MusicApp {
    /// `visible` : sans réglage en base, le défaut est « visible ».
    pub(crate) fn visible(&self, key: &str) -> bool {
        self.data.setting(key) != Some("false")
    }

    /// Les clés retenues, dans l'ordre de référence ; abîmé ou vide, le défaut.
    pub(crate) fn library_tab_keys(&self) -> Vec<&'static str> {
        let default_value = || vec!["tracks", "artists", "albums", "genres", "folders"];
        let Some(raw) = self.data.setting("library_tabs") else { return default_value() };
        let Ok(keys) = serde_json::from_str::<Vec<String>>(raw) else { return default_value() };
        let out: Vec<&'static str> = LIBRARY_TABS.iter().map(|(k, _)| *k).filter(|k| keys.iter().any(|c| c == k)).collect();
        if out.is_empty() {
            default_value()
        } else {
            out
        }
    }

    pub(crate) fn settings_appearance(&self) -> Vec<OptionGroup> {
        let s = |k: &str| self.setting_or_default(k);
        let theme = s("theme");

        // ─── Thème ───
        let themes = [("auto", "settings.theme_auto"), ("light", "settings.theme_light"), ("dark", "settings.theme_dark")];
        let theme_cards = OptionRow::block(
            "theme",
            Control::Cards(
                3,
                themes
                    .iter()
                    .map(|(v, key)| Card { label: t(key), title: None, selected: theme == *v, msg: set_msg("theme", v), preview: Preview::Theme(v) })
                    .collect(),
            ),
        );
        let theme_words = format!("{} {} theme", t("settings.theme"), themes.iter().map(|(_, k)| t(k)).collect::<Vec<_>>().join(" "));
        let accent = s("accent_color").to_lowercase();
        let current_item = if valid_accent(&accent) { accent } else { ACCENTS[0].0.to_string() };
        let free = !ACCENTS.iter().any(|(h, _)| *h == current_item);
        let pills = ACCENTS
            .iter()
            .enumerate()
            .map(|(i, (h, key))| Swatch {
                color: hex(h),
                label: t(key),
                selected: current_item == *h,
                msg: set_msg("accent_color", if i == 0 { "" } else { h }),
            })
            .collect();
        let accent_row = OptionRow::new("accent", t("settings.accent"), t("settings.accent_desc"), Control::Accent(pills, free.then(|| hex(&current_item))));
        let contrast = if s("contrast") == "high" { "high" } else { "normal" };
        let contrast_row = OptionRow::new(
            "contrast",
            t("settings.contrast"),
            t("settings.contrast_desc"),
            Control::Segmented(segments("contrast", contrast, &[("normal", t("settings.contrast_normal")), ("high", t("settings.contrast_high"))]), false),
        );

        // ─── Barre latérale ───
        let open_it = s("show_open_buttons") == "true";
        let playlists = self.visible("show_sidebar_playlists");
        let albums = self.visible("show_pinned_albums");
        let artists = self.visible("show_pinned_artists");
        let bar = [
            ("show_open_buttons", "settings.open_buttons", "settings.open_buttons_desc", open_it),
            ("show_sidebar_playlists", "settings.sidebar_playlists", "settings.sidebar_playlists_desc", playlists),
            ("show_pinned_albums", "settings.pinned_albums", "settings.pinned_albums_desc", albums),
            ("show_pinned_artists", "settings.pinned_artists", "settings.pinned_artists_desc", artists),
        ];

        // ─── Listes automatiques ───
        let favorites = self.visible("show_favorites");
        let lists: [(&str, &str, i32, bool, Vec<(&'static str, String)>); 2] = [
            ("playlist_page.liked_title", "material-symbols:favorite-rounded", 10, !favorites, vec![("show_liked_in_playlists", t("nav.playlists"))]),
            (
                "playlist_page.recent_title",
                "material-symbols:history-rounded",
                230,
                false,
                vec![("show_recent_in_playlists", t("nav.playlists")), ("show_recent_in_home", t("nav.home"))],
            ),
        ];
        let mut list_rows: Vec<Option<OptionRow>> = lists
            .into_iter()
            .map(|(title, icon, tint, disabled, places)| {
                let hidden = places.iter().all(|(k, _)| !self.visible(k));
                let desc = if disabled {
                    t("settings.liked_needs_favorites")
                } else if hidden {
                    t("settings.builtin_playlists_hidden_note")
                } else {
                    String::new()
                };
                let words = places.iter().map(|(_, l)| l.clone()).collect::<Vec<_>>().join(" ");
                let pills = places
                    .into_iter()
                    .map(|(k, label)| Pill { label, checked: self.visible(k) && !disabled, disabled, title: None, msg: toggle_msg(k) })
                    .collect();
                self.keep(OptionRow::new(icon, t(title), desc, Control::Pills(pills)).lead(icon, tint), &words)
            })
            .collect();
        list_rows.push(self.keep(
            OptionRow::new("show_favorites", t("settings.favorites"), t("settings.favorites_desc"), Control::Toggle(toggle_msg("show_favorites"), favorites)),
            "favoris coeur like heart",
        ));

        // ─── Navigation de la bibliothèque ───
        let placement = match s("library_tabs_position").as_str() {
            "sidebar" => "sidebar",
            "top" => "top",
            _ => "both",
        };
        let placements = [("sidebar", "settings.library_nav_sidebar"), ("top", "settings.library_nav_top"), ("both", "settings.library_nav_both")];
        let nav_cards = OptionRow::block(
            "library_nav",
            Control::Cards(
                3,
                placements
                    .iter()
                    .map(|(v, key)| Card { label: t(key), title: None, selected: placement == *v, msg: set_msg("library_tabs_position", v), preview: Preview::Library(v) })
                    .collect(),
            ),
        );
        let nav_words = format!(
            "{} {} {}",
            t("settings.library_nav"),
            t("settings.library_nav_hint"),
            placements.iter().map(|(_, k)| t(k)).collect::<Vec<_>>().join(" ")
        );
        let kept_items = self.library_tab_keys();
        let tabs = LIBRARY_TABS
            .iter()
            .map(|(k, key)| {
                let checked = kept_items.contains(k);
                let last = checked && kept_items.len() == 1;
                Pill {
                    label: t(key),
                    checked,
                    disabled: last,
                    title: last.then(|| t("settings.library_tabs_last")),
                    msg: Msg::Settings(SettingsMsg::Tab(k)),
                }
            })
            .collect();
        let tabs_row = OptionRow::new("library_tabs", t("settings.library_tabs_shown"), t("settings.library_tabs_note"), Control::PillsBlock(tabs));
        let tab_words = LIBRARY_TABS.iter().map(|(_, k)| t(k)).collect::<Vec<_>>().join(" ");

        // ─── Fenêtre : « Auto » montre le style du système détecté ───
        let (system, name) = if cfg!(target_os = "macos") {
            ("macos", "macOS")
        } else if cfg!(target_os = "linux") {
            ("linux", "Linux")
        } else {
            ("windows", "Windows")
        };
        let left = s("window_controls_position") == "left";
        let style = self.data.setting("window_controls_style").filter(|v| !v.is_empty()).unwrap_or("auto").to_string();
        let mut styles = vec![Card {
            label: t("settings.window_controls_auto"),
            title: Some(t("settings.window_controls_auto_hint").replace("{os}", name)),
            selected: style == "auto",
            msg: set_msg("window_controls_style", "auto"),
            preview: Preview::Window(system, left),
        }];
        styles.extend([("macos", "macOS"), ("windows", "Windows"), ("linux", "Linux")].map(|(v, l)| Card {
            label: l.into(),
            title: None,
            selected: style == v,
            msg: set_msg("window_controls_style", v),
            preview: Preview::Window(v, left),
        }));
        let position = OptionRow::new(
            "window_controls_position",
            t("settings.window_controls_position"),
            t("settings.window_controls_position_desc"),
            Control::Segmented(
                segments("window_controls_position", if left { "left" } else { "right" }, &[("left", t("settings.left")), ("right", t("settings.right"))]),
                false,
            ),
        );

        // ─── Mini-lecteur ───
        let mini = if s("mini_volume_layout") == "horizontal" { "horizontal" } else { "vertical" };
        let mini_row = OptionRow::new(
            "mini_volume_layout",
            t("settings.mini_volume_layout"),
            t("settings.mini_volume_layout_desc"),
            Control::Segmented(segments("mini_volume_layout", mini, &[("vertical", t("settings.vertical")), ("horizontal", t("settings.horizontal"))]), false),
        );

        // ─── Options expert ───
        let blur = if s("blurred_backgrounds") == "live" { "live" } else { "precomputed" };
        let blur_row = OptionRow::new(
            "blurred_backgrounds",
            t("settings.blurred_backgrounds"),
            t("settings.blurred_backgrounds_desc"),
            Control::Segmented(
                segments("blurred_backgrounds", blur, &[("precomputed", t("settings.blurred_precomputed")), ("live", t("settings.blurred_live"))]),
                false,
            ),
        );
        let effects = s("visual_effects");
        let effects = if effects == "full" || effects == "light" { effects } else { "auto".into() };
        let (light, reason) = light_effects();
        let auto_txt = if *light {
            t("settings.visual_effects_auto_light").replace("{reason}", reason.as_deref().unwrap_or(""))
        } else {
            t("settings.visual_effects_auto_full")
        };
        let effects_row = OptionRow::new(
            "visual_effects",
            t("settings.visual_effects"),
            format!("{} {}", t("settings.visual_effects_desc"), auto_txt),
            Control::Segmented(
                segments(
                    "visual_effects",
                    &effects,
                    &[("auto", t("settings.visual_effects_auto")), ("full", t("settings.visual_effects_full")), ("light", t("settings.visual_effects_light"))],
                ),
                false,
            ),
        );

        vec![
            self.option_group(
                "theme",
                Some(t("settings.theme")),
                Some(t("settings.theme_desc")),
                Some("settings.theme"),
                vec![
                    self.keep(theme_cards, &theme_words),
                    self.keep(accent_row, "accent couleur color"),
                    self.keep(contrast_row, "contrast"),
                    self.toggle_row("show_help_bubbles", t("settings.help_bubbles"), t("settings.help_bubbles_desc"), "aide help glossary dac dsd"),
                ],
            ),
            self.option_group(
                "sidebar",
                Some(t("settings.sidebar_group")),
                Some(t("settings.sidebar_group_hint")),
                Some("settings.sidebar_group"),
                bar
                    .iter()
                    .map(|(k, title, desc, on)| self.keep(OptionRow::new(k, t(title), t(desc), Control::Toggle(toggle_msg(k), *on)), "sidebar"))
                    .collect(),
            )
            .with_sidebar([open_it, playlists, albums, artists]),
            self.option_group("lists", Some(t("settings.lists_group")), Some(t("settings.lists_group_hint")), Some("settings.lists_group"), list_rows),
            self.option_group(
                "library_nav",
                Some(t("settings.library_nav")),
                Some(t("settings.library_nav_hint")),
                Some("settings.library_nav"),
                vec![self.keep(nav_cards, &nav_words), self.keep(tabs_row, &tab_words)],
            ),
            self.option_group(
                "window",
                Some(t("settings.window_group")),
                Some(t("settings.window_group_hint")),
                Some("settings.window_group"),
                vec![
                    self.keep(
                        OptionRow::block("window_style", Control::Cards(4, styles)),
                        &format!("{} {} macos windows linux", t("settings.window_controls_style"), t("settings.window_group_hint")),
                    ),
                    self.keep(position, ""),
                ],
            ),
            self.option_group("mini", Some(t("settings.mini_group")), Some(t("settings.mini_group_hint")), Some("settings.mini_group"), vec![self.keep(mini_row, "mini volume")]),
            self.option_group(
                "expert",
                Some(t("settings.expert_group")),
                Some(t("settings.expert_group_hint")),
                Some("settings.expert_group"),
                vec![self.keep(blur_row, "flou blur pochette fond"), self.keep(effects_row, "effets flou fondu lent performance vieux pc vm")],
            ),
        ]
    }
}
