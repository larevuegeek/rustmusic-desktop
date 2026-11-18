//! Les briques des pages de réglages : lignes (`OptionItem`), blocs libres,
//! groupes (`OptionGroup`) et leurs contrôles (`ToggleSwitch`, liste déroulante,
//! `SegmentedControl`, `TogglePill`, `GhostButton`, `ChoiceCard`…).

use crate::models::settings::*;
use karbon_ui::ui::prelude::Color;
use crate::models::app::Choice;
use crate::models::app::Msg;
use crate::models::settings::SettingsMsg;
use crate::models::app::SettingsPage;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::{t, t_with};

/// Un interrupteur qui bascule un réglage.
pub fn toggle_msg(key: &'static str) -> Msg {
    Msg::Settings(SettingsMsg::Toggle(key))
}

/// Un choix qui écrit une valeur.
pub fn set_msg(key: &'static str, value: &str) -> Msg {
    Msg::Settings(SettingsMsg::Set(key, value.to_string()))
}

/// Options d'un `SegmentedControl` qui écrit `key`.
pub fn segments(key: &'static str, current: &str, options: &[(&str, String)]) -> Vec<Choice> {
    options
        .iter()
        .map(|(v, label)| Choice { label: label.clone(), icon: String::new(), active: *v == current, msg: set_msg(key, v), icon_only: false })
        .collect()
}

/// La valeur de chaque réglage avant tout choix.
pub const DEFAULTS: &[(&str, &str)] = &[
    ("language", "fr"), ("auto_start", "false"), ("minimize_to_tray", "true"), ("show_notifications", "false"),
    ("scan_on_startup", "false"), ("single_click_play", "false"), ("show_sleep_timer", "true"),
    ("show_whats_new_button", "true"), ("show_whats_new_at_startup", "false"), ("show_theme_toggle", "true"), ("system_media_controls", "true"),
    ("wasapi_exclusive", "false"), ("dsd_dop", "false"), ("gapless", "true"), ("resume_playback", "true"),
    ("play_on_start", "false"), ("listen_threshold", "auto"), ("prevent_sleep", "true"), ("theme", "dark"),
    ("contrast", "normal"), ("window_controls_style", "auto"), ("window_controls_position", "right"),
    ("show_liked_in_playlists", "true"), ("show_recent_in_playlists", "true"), ("show_recent_in_home", "true"),
    ("show_open_buttons", "true"), ("show_favorites", "true"), ("library_tabs_position", "both"),
    ("library_tabs", r#"["tracks","artists","albums","genres","folders"]"#), ("amp_remote", "false"),
    ("podcasts", "false"), ("podcast_play_mode", "stream"), ("podcast_country", ""), ("radios", "false"),
    ("radio_country", ""), ("sidebar_order", r#"["playlists","online","pins"]"#), ("sidebar_playlists_order", "[]"),
    ("pins_hint_dismissed", "false"), ("show_pinned_albums", "true"), ("show_pinned_artists", "true"),
    ("show_help_bubbles", "true"), ("auto_download_artist_images", "true"), ("podcast_speed", "1"),
    ("podcast_refresh", "6h"), ("podcast_cleanup", "24h"), ("network_players", "false"), ("airplay", "false"),
    ("artist_check_auto", "false"), ("accent_color", ""), ("mini_volume_layout", "vertical"),
    ("blurred_backgrounds", "precomputed"), ("visual_effects", "auto"), ("shortcuts", ""),
    ("show_sidebar_playlists", "true"),
];

/// Page, adresse, icône, libellé, sous-titre, phrase d'en-tête, réglages
/// comptés, intitulé de groupe dans la navigation.
#[allow(clippy::type_complexity)]
pub const SECTIONS: &[(SettingsPage, &str, &str, &str, &str, &str, &[&str], Option<&str>)] = &[
    (SettingsPage::General, "general", "material-symbols:tune-rounded", "settings.general", "settings.general_hint", "settings.general_desc",
     &["auto_start", "minimize_to_tray", "scan_on_startup", "play_on_start", "single_click_play", "resume_playback", "listen_threshold", "gapless",
       "prevent_sleep", "system_media_controls", "show_notifications", "show_sleep_timer", "show_whats_new_button", "show_whats_new_at_startup", "show_theme_toggle"], None),
    (SettingsPage::Appearance, "appearance", "material-symbols:palette-outline-rounded", "settings.appearance", "settings.appearance_hint", "settings.appearance_desc",
     &["theme", "contrast", "show_help_bubbles", "show_open_buttons", "show_pinned_albums", "show_pinned_artists", "show_liked_in_playlists",
       "show_recent_in_playlists", "show_recent_in_home", "show_favorites", "library_tabs_position", "library_tabs", "sidebar_order",
       "sidebar_playlists_order", "pins_hint_dismissed", "window_controls_style", "window_controls_position"], None),
    (SettingsPage::Audio, "audio", "material-symbols:speaker-outline-rounded", "settings.audio", "settings.audio_hint", "settings.audio_desc", &["wasapi_exclusive", "dsd_dop"], None),
    (SettingsPage::Network, "network", "material-symbols:cast-rounded", "settings.network", "settings.network_hint", "settings.network_desc", &[], None),
    (SettingsPage::Online, "online", "material-symbols:podcasts-rounded", "settings.online", "settings.online_hint", "settings.online_desc",
     &["podcasts", "podcast_play_mode", "podcast_country", "radios", "radio_country"], None),
    (SettingsPage::Remote, "remote", "material-symbols:settings-remote-outline-rounded", "settings.remote", "settings.remote_hint", "settings.remote_desc", &["amp_remote"], None),
    (SettingsPage::Storage, "storage", "material-symbols:database-outline-rounded", "settings.storage", "settings.storage_hint", "settings.storage_desc", &[], None),
    (SettingsPage::Shortcuts, "shortcuts", "material-symbols:keyboard-outline-rounded", "settings.shortcuts", "settings.shortcuts_hint", "settings.shortcuts_desc", &[], None),
    (SettingsPage::Guide, "guide", "material-symbols:school-outline-rounded", "settings.guide", "settings.guide_hint", "settings.guide_desc", &[], Some("settings.nav_help")),
    (SettingsPage::About, "about", "material-symbols:info-outline-rounded", "settings.about_short", "settings.about_hint", "settings.about_desc", &[], None),
];

/// Minuscules sans accents, pour la recherche.
pub(crate) fn fold(s: &str) -> String {
    crate::mapper::albums::sort_key(s)
}

/// `matches` : chaque mot de la recherche se trouve dans le titre, la description ou les mots-clés.
pub fn matches(query: &str, title: &str, desc: &str, keywords: &str) -> bool {
    let q = fold(query);
    if q.trim().is_empty() {
        return true;
    }
    let text = fold(&format!("{title} {} {keywords}", plain_text(desc)));
    q.split_whitespace().all(|m| text.contains(m))
}

/// « [[dac|convertisseur]] » marque un terme expliqué. Rend les morceaux, avec
/// l'identifiant du terme.
pub fn help_split(text: &str) -> Vec<(String, Option<String>)> {
    let mut tracks = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        let Some(end) = rest[start..].find("]]").map(|f| start + f) else { break };
        let tag = &rest[start + 2..end];
        let Some((id, word)) = tag.split_once('|') else { break };
        if !id.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            break;
        }
        if start > 0 {
            tracks.push((rest[..start].to_string(), None));
        }
        tracks.push((word.to_string(), Some(id.to_string())));
        rest = &rest[end + 2..];
    }
    if !rest.is_empty() {
        tracks.push((rest.to_string(), None));
    }
    tracks
}

/// Le texte sans balisage, pour la recherche et les attributs.
pub fn plain_text(text: &str) -> String {
    help_split(text).into_iter().map(|(t, _)| t).collect()
}

impl Ghost {
    pub fn new(icon: &'static str, label: String, msg: Msg) -> Self {
        Ghost { label, icon, danger: false, busy: false, disabled: false, title: None, msg }
    }

    /// L'icône affichée : la roue pendant `busy`.
    pub fn shown_icon(&self) -> &'static str {
        if self.busy {
            "material-symbols:progress-activity"
        } else {
            self.icon
        }
    }

    /// Inerte : désactivé ou occupé.
    pub fn inert(&self) -> bool {
        self.busy || self.disabled
    }
}

impl Preview {
    pub fn theme(&self) -> Option<&'static str> {
        match self {
            Preview::Theme(m) => Some(m),
            _ => None,
        }
    }

    pub fn library(&self) -> Option<&'static str> {
        match self {
            Preview::Library(p) => Some(p),
            _ => None,
        }
    }

    pub fn window(&self) -> Option<(&'static str, bool)> {
        match self {
            Preview::Window(s, left) => Some((s, *left)),
            _ => None,
        }
    }
}

impl OptionRow {
    pub fn new(key: &str, title: String, desc: String, control: Control) -> Self {
        OptionRow { key: key.into(), title, desc, value: None, lead: None, control, disabled: false }
    }

    /// Un bloc sans texte propre.
    pub fn block(key: &str, control: Control) -> Self {
        Self::new(key, String::new(), String::new(), control)
    }

    /// Bloc pleine largeur plutôt que ligne.
    pub fn is_block(&self) -> bool {
        matches!(
            self.control,
            Control::Cards(..) | Control::Accent(..) | Control::PillsBlock(_) | Control::Info(_) | Control::Block(_)
        )
    }

    /// Le message de la ligne cliquable (interrupteur actif).
    pub fn click(&self) -> Option<Msg> {
        match &self.control {
            Control::Toggle(m, _) if !self.disabled => Some(m.clone()),
            _ => None,
        }
    }

    pub fn toggle(&self) -> Option<bool> {
        match self.control {
            Control::Toggle(_, on) => Some(on),
            _ => None,
        }
    }

    /// Liste de choix : sa clé, l'indice choisi et les libellés.
    pub fn select(&self) -> Option<(&'static str, Option<u64>, Vec<String>)> {
        match &self.control {
            Control::Select(k, v, opts) => Some((k, opts.iter().position(|o| o.0 == *v).map(|i| i as u64), opts.iter().map(|o| o.1.clone()).collect())),
            _ => None,
        }
    }

    pub fn menu(&self) -> Option<(&'static str, &String, &Vec<Choice>)> {
        match &self.control {
            Control::Menu(a, l, o) => Some((a, l, o)),
            _ => None,
        }
    }

    pub fn segmented(&self) -> Option<(&Vec<Choice>, bool)> {
        match &self.control {
            Control::Segmented(o, d) => Some((o, *d)),
            _ => None,
        }
    }

    pub fn pills(&self) -> Option<&Vec<Pill>> {
        match &self.control {
            Control::Pills(p) => Some(p),
            _ => None,
        }
    }

    pub fn buttons(&self) -> Option<&Vec<Ghost>> {
        match &self.control {
            Control::Buttons(b) => Some(b),
            _ => None,
        }
    }

    /// Le nom du contrôle ou du bloc propre à une page, sinon vide.
    pub fn custom(&self) -> &'static str {
        match self.control {
            Control::Custom(c) | Control::Block(c) => c,
            _ => "",
        }
    }

    pub fn cards(&self) -> Option<(u8, &Vec<Card>)> {
        match &self.control {
            Control::Cards(n, c) => Some((*n, c)),
            _ => None,
        }
    }

    pub fn accent(&self) -> Option<(&Vec<Swatch>, Option<Color>)> {
        match &self.control {
            Control::Accent(s, c) => Some((s, *c)),
            _ => None,
        }
    }

    pub fn pills_block(&self) -> Option<&Vec<Pill>> {
        match &self.control {
            Control::PillsBlock(p) => Some(p),
            _ => None,
        }
    }

    pub fn info(&self) -> Option<&'static str> {
        match self.control {
            Control::Info(i) => Some(i),
            _ => None,
        }
    }
}

impl OptionGroup {
    pub fn badge(&self) -> Option<(&String, &'static str)> {
        match &self.action {
            Some(GroupAction::Badge(t, tone)) => Some((t, *tone)),
            _ => None,
        }
    }

    pub fn button(&self) -> Option<&Ghost> {
        match &self.action {
            Some(GroupAction::Button(g)) => Some(g),
            _ => None,
        }
    }
}

impl MusicApp {
    /// La valeur d'un réglage, sinon sa valeur par défaut.
    pub(crate) fn setting_or_default(&self, key: &str) -> String {
        self.data
            .setting(key)
            .map(String::from)
            .unwrap_or_else(|| DEFAULTS.iter().find(|(k, _)| *k == key).map(|(_, v)| v.to_string()).unwrap_or_default())
    }

    pub(crate) fn toggle_row(&self, key: &'static str, title: String, desc: String, keywords: &str) -> Option<OptionRow> {
        let on = self.setting_or_default(key) == "true";
        self.keep(OptionRow::new(key, title, desc, Control::Toggle(crate::mapper::settings::toggle_msg(key), on)), keywords)
    }

    /// La ligne, si la recherche la trouve (titre, description, mots-clés).
    pub(crate) fn keep(&self, row: OptionRow, keywords: &str) -> Option<OptionRow> {
        matches(&self.views.settings.search, &row.title, &row.desc, keywords).then_some(row)
    }

    /// La page de réglages affichée, quand la route en est une.
    pub fn active_settings_view(&self) -> Option<SettingsView> {
        match self.route() {
            crate::models::app::Route::Settings(page) => Some(self.settings_view(*page)),
            _ => None,
        }
    }

    pub fn settings_view(&self, page: SettingsPage) -> SettingsView {
        let section = SECTIONS.iter().find(|s| s.0 == page).unwrap_or(&SECTIONS[0]);
        let nav = SECTIONS
            .iter()
            .map(|s| NavItem { page: s.0, icon: s.2, label: t(s.3), hint: t(s.4), active: s.0 == page, group: s.7.map(t) })
            .collect();
        let modified_items: Vec<&&str> = section.6.iter().filter(|k| {
            let def = DEFAULTS.iter().find(|(d, _)| d == *k).map(|(_, v)| *v).unwrap_or("");
            self.data.setting(k).is_some_and(|v| v != def)
        }).collect();
        let groups = match page {
            SettingsPage::General => self.settings_general(),
            SettingsPage::Appearance => self.settings_appearance(),
            SettingsPage::Audio => self.settings_audio(),
            SettingsPage::Network => self.settings_network(),
            SettingsPage::Online => self.settings_online(),
            SettingsPage::Remote => self.settings_remote(),
            SettingsPage::Storage => self.settings_storage(),
            SettingsPage::Shortcuts => self.settings_shortcuts(),
            SettingsPage::Guide => self.settings_guide(),
            SettingsPage::About => self.settings_about(),
        };
        let search = self.views.settings.search.clone();
        let none_found = (!search.trim().is_empty() && groups.iter().all(|g| g.rows.is_empty()))
            .then(|| t_with("settings.search_empty", &[("query", search.trim())]));
        SettingsView {
            nav,
            title: t(section.3),
            desc: t(section.5),
            modified_items: (!modified_items.is_empty()).then(|| (modified_items.len(), t(if modified_items.len() > 1 { "settings.modified_many" } else { "settings.modified_one" }))),
            groups: groups.into_iter().filter(|g| !g.rows.is_empty() || g.title.is_none()).collect(),
            search,
            none_found,
        }
    }

    /// Un groupe : repliable s'il a une clé de pli (sauf pendant une recherche).
    pub(crate) fn option_group(&self, key: &str, title: Option<String>, hint: Option<String>, fold: Option<&'static str>, rows: Vec<Option<OptionRow>>) -> OptionGroup {
        let search = !self.views.settings.search.trim().is_empty();
        OptionGroup {
            key: key.into(),
            title,
            hint,
            fold,
            closed: fold.is_some_and(|f| self.views.settings.replies.contains(f)) && !search,
            action: None,
            sidebar: None,
            rows: rows.into_iter().flatten().collect(),
        }
    }
}
