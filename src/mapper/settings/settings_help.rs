use crate::models::settings::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::settings::Control;
use crate::models::settings::Ghost;
use crate::models::settings::OptionGroup;
use crate::models::settings::OptionRow;
use crate::mapper::settings::plain_text;
use crate::models::settings::SettingsMsg;

/// Les étapes du trajet du son : clé, icône, teinte de la tuile.
pub const STEPS: [(&str, &str, i32); 5] = [
    ("file", "material-symbols:audio-file-outline-rounded", 250),
    ("decode", "material-symbols:unarchive-outline-rounded", 155),
    ("dsp", "material-symbols:graphic-eq-rounded", 70),
    ("mixer", "material-symbols:tune-rounded", 300),
    ("dac", "material-symbols:headphones-outline-rounded", 200),
];

/// Les termes du glossaire.
pub const TERMS: [&str; 12] = ["dac", "pcm", "sample_rate", "bit_depth", "dsd", "dop", "shared", "exclusive", "resampling", "dsp", "replay_gain", "bit_perfect"];

/// Les pastilles du lecteur.
pub const BADGES: [&str; 6] = ["bit-perfect", "exclusive", "dop", "resampled", "dsd", "shared"];

/// Libellé et explication d'une pastille.
pub fn badge_keys(mode: &str) -> (&'static str, &'static str) {
    match mode {
        "bit-perfect" => ("pipeline.badge_bit_perfect", "pipeline.bit_perfect_desc"),
        "exclusive" => ("pipeline.badge_exclusive", "pipeline.exclusive_desc"),
        "dop" => ("pipeline.badge_dop", "pipeline.dop_desc"),
        "resampled" => ("pipeline.badge_resampled", "pipeline.resampled_desc"),
        "dsd" => ("pipeline.badge_dsd_pcm", "pipeline.dsd_pcm_desc"),
        _ => ("pipeline.badge_shared", "pipeline.shared_desc"),
    }
}

/// `technologies` : catégorie, puis (nom, version, adresse, icône).
#[allow(clippy::type_complexity)]
pub const STACK: [(&str, &[(&str, &str, &str, &str)]); 4] = [
    ("about.stack_app", &[("karbon-ui", "0.3", "https://github.com/larevuegeek/karbon-ui", "material-symbols:deployed-code-outline")]),
    ("about.stack_ui", &[("Tailwind CSS", "4.3", "https://tailwindcss.com", "simple-icons:tailwindcss")]),
    (
        "about.stack_engine",
        &[
            ("Rust", "2021", "https://rust-lang.org", "simple-icons:rust"),
            ("SQLx", "0.8", "https://github.com/launchbadge/sqlx", "material-symbols:database-outline"),
            ("SQLite", "3", "https://sqlite.org", "simple-icons:sqlite"),
            ("Tokio", "1.52", "https://tokio.rs", "material-symbols:bolt-outline-rounded"),
        ],
    ),
    (
        "about.stack_audio",
        &[
            ("Symphonia", "0.6", "https://github.com/pdeljanov/Symphonia", "material-symbols:music-note-rounded"),
            ("CPAL", "0.18", "https://github.com/RustAudio/cpal", "material-symbols:speaker-outline-rounded"),
            ("Rubato", "3.0", "https://github.com/HEnquist/rubato", "material-symbols:graphic-eq-rounded"),
            ("WASAPI", "0.23", "https://github.com/HEnquist/wasapi-rs", "material-symbols:settings-input-component-outline-rounded"),
        ],
    ),
];

/// `soutiens` : adresse, icône, libellé, principal (Patreon d'abord).
pub const SUPPORT: [(&str, &str, &str, bool); 4] = [
    ("https://patreon.com/larevuegeek", "simple-icons:patreon", "support.patreon", true),
    ("https://ko-fi.com/larevuegeek", "simple-icons:kofi", "support.kofi", false),
    ("https://paypal.me/larevuegeek", "simple-icons:paypal", "support.paypal", false),
    ("https://github.com/larevuegeek/rustmusic-desktop", "lucide:star", "support.star", false),
];

/// Pour les gabarits : les pastilles, les termes, les soutiens.
pub fn guide_badges() -> [&'static str; 6] {
    BADGES
}

/// L'ampli piloté : « Denon · 192.168.1.20 ».
pub fn amp_line(amp: &crate::core::amp_remote::model::Amp) -> String {
    format!("{} · {}", super::brand_label(&amp.brand), amp.ip)
}

/// « RustMusic 0.5.0 bêta ».
pub fn version_line() -> String {
    format!("RustMusic {} {}", env!("CARGO_PKG_VERSION"), t("settings.beta"))
}

/// « v0.5.0 ».
pub fn version_tag() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

/// L'icône du choix actif d'un menu.
pub fn active_icon(choices: &[crate::models::app::Choice]) -> String {
    choices.iter().find(|c| c.active).map(|c| c.icon.clone()).unwrap_or_default()
}

impl Card {
    /// Infobulle, vide sans titre.
    pub fn tooltip(&self) -> &str {
        self.title.as_deref().unwrap_or("")
    }
}

impl Ghost {
    /// Infobulle, vide sans titre.
    pub fn tooltip(&self) -> &str {
        self.title.as_deref().unwrap_or("")
    }
}

impl Pill {
    /// Infobulle, vide sans titre.
    pub fn tooltip(&self) -> &str {
        self.title.as_deref().unwrap_or("")
    }
}

/// L'index du glossaire : (terme, nom traduit).
pub fn guide_terms() -> Vec<(&'static str, String)> {
    TERMS.iter().map(|id| (*id, t(&format!("glossary.{id}.name")))).collect()
}

/// Les cartes « Partagé » et « Exclusif » : le partagé a une limite, l'exclusif deux.
pub fn guide_modes() -> Vec<GuideMode> {
    [("shared", "material-symbols:group-outline-rounded", 230, 1), ("exclusive", "material-symbols:lock-outline-rounded", 150, 2)]
        .into_iter()
        .map(|(id, icon, hue, cons)| GuideMode {
            id,
            icon,
            hue,
            name: t(&format!("glossary.{id}.name")),
            for_whom: t(&format!("guide.mode_{id}_for")),
            pros: ["pro1", "pro2"].iter().map(|c| t(&format!("guide.mode_{id}_{c}"))).collect(),
            cons: ["con1", "con2"].iter().take(cons).map(|c| t(&format!("guide.mode_{id}_{c}"))).collect(),
        })
        .collect()
}

impl Step {
    /// « Source », « Mixeur »… : le nom de l'étape.
    pub fn title(&self) -> String {
        t(&format!("guide.step_{}_title", self.key))
    }

    /// Sa description courte, sous le nom.
    pub fn short(&self) -> String {
        t(&format!("guide.step_{}_short", self.key))
    }

    /// « 2/5 ».
    pub fn number(&self) -> String {
        format!("{}/5", self.index + 1)
    }
}

pub fn about_support() -> [(&'static str, &'static str, &'static str, bool); 4] {
    SUPPORT
}

impl Default for GuideState {
    fn default() -> Self {
        GuideState { exclusive: false, step: 1, term: None }
    }
}

impl MusicApp {
    pub fn guide_exclusive(&self) -> bool {
        self.views.settings.guide.exclusive
    }

    pub fn guide_steps(&self) -> Vec<Step> {
        let s = &self.views.settings;
        STEPS
            .iter()
            .enumerate()
            .map(|(i, (key, icon, hue))| Step { index: i, key, icon, hue: *hue, selected: s.guide.step == i, bypassed: s.guide.exclusive && *key == "mixer" })
            .collect()
    }

    /// L'étape choisie et son explication.
    pub fn guide_step(&self) -> (Step, String) {
        let step = self.guide_steps().swap_remove(self.views.settings.guide.step.min(4));
        let desc = if step.bypassed { t("guide.step_mixer_desc_exclusive") } else { t(&format!("guide.step_{}_desc", step.key)) };
        (step, desc)
    }

    /// Les pastilles du résultat : partagé (partagé, rééchantillonné), exclusif (bit-perfect, DoP).
    pub fn guide_result(&self) -> [&'static str; 2] {
        if self.views.settings.guide.exclusive {
            ["bit-perfect", "dop"]
        } else {
            ["shared", "resampled"]
        }
    }

    pub fn guide_term_read(&self, id: &str) -> bool {
        self.views.settings.guide.term.as_deref() == Some(id)
    }

    pub(crate) fn settings_guide(&self) -> Vec<OptionGroup> {
        let steps_list = STEPS.iter().map(|(k, ..)| t(&format!("guide.step_{k}_title"))).collect::<Vec<_>>().join(" ");
        let pills = BADGES.iter().map(|b| t(badge_keys(b).0)).collect::<Vec<_>>().join(" ");
        let mut glossary = vec![Some(OptionRow::block("guide_index", Control::Block("guide-index")))];
        glossary.extend(TERMS.iter().map(|id| {
            self.keep(
                OptionRow::new(id, t(&format!("glossary.{id}.name")), t(&format!("glossary.{id}.long")), Control::Block("guide-term")),
                &plain_text(&t(&format!("glossary.{id}.long"))),
            )
        }));
        vec![
            self.option_group(
                "path",
                Some(t("guide.path_title")),
                Some(t("guide.path_hint")),
                Some("guide.path_title"),
                vec![self.keep(OptionRow::block("guide_path", Control::Block("guide-path")), &format!("{} {steps_list}", t("guide.path_title")))],
            ),
            self.option_group(
                "modes",
                Some(t("guide.modes_title")),
                Some(t("guide.modes_hint")),
                Some("guide.modes_title"),
                vec![self.keep(
                    OptionRow::block("guide_modes", Control::Block("guide-modes")),
                    &format!("{} {} {}", t("guide.modes_title"), t("glossary.shared.name"), t("glossary.exclusive.name")),
                )],
            ),
            self.option_group(
                "badges",
                Some(t("guide.badges_title")),
                Some(t("guide.badges_hint")),
                Some("guide.badges_title"),
                vec![self.keep(OptionRow::block("guide_badges", Control::Block("guide-badges")), &format!("{} {pills}", t("guide.badges_title")))],
            ),
            self.option_group("glossary", Some(t("guide.glossary_title")), Some(t("guide.glossary_hint")), Some("guide.glossary_title"), glossary),
        ]
    }

    pub(crate) fn settings_about(&self) -> Vec<OptionGroup> {
        let sm = |m: SettingsMsg| Msg::Settings(m);
        let opened = self.views.settings.license_open;
        let mut licence = vec![self.keep(
            OptionRow::new(
                "license",
                t("about.license_title"),
                t("about.license_summary"),
                Control::Buttons(vec![if opened {
                    Ghost::new("material-symbols:expand-less-rounded", t("about.license_hide"), sm(SettingsMsg::LicenseToggle))
                } else {
                    Ghost::new("material-symbols:description-outline-rounded", t("about.license_read"), sm(SettingsMsg::LicenseToggle))
                }]),
            ),
            "licence copyright",
        )];
        if opened {
            licence.push(self.keep(OptionRow::block("license_text", Control::Block("about-license")), "licence"));
        }
        let mut stack = Vec::new();
        for (cat, libs) in STACK {
            stack.push(self.keep(
                OptionRow::new(cat, t(cat), String::new(), Control::Block("about-stack")),
                &format!("{} {}", t(cat), libs.iter().map(|l| l.0).collect::<Vec<_>>().join(" ")),
            ));
        }
        vec![
            self.option_group(
                "identity",
                None,
                None,
                None,
                vec![self.keep(
                    OptionRow::block("about_card", Control::Block("about-card")),
                    &format!("rustmusic version {} {}", t("about.tagline"), t("settings.update_unchecked")),
                )],
            ),
            self.option_group(
                "support",
                Some(t("support.about_title")),
                Some(t("support.about_hint")),
                Some("support.about_title"),
                vec![self.keep(OptionRow::block("about_support", Control::Block("about-support")), &format!("{} patreon ko-fi paypal github don", t("support.about_title")))],
            ),
            self.option_group("stack", Some(t("about.stack")), Some(t("about.stack_hint")), Some("about.stack"), stack),
            self.option_group("license", Some(t("about.license")), Some(t("about.license_hint")), Some("about.license"), licence),
            self.option_group(
                "credits",
                Some(t("about.credits")),
                None,
                Some("about.credits"),
                vec![
                    self.keep(OptionRow::new("made_with", t("about.made_with"), t("about.credits_text"), Control::None), ""),
                    self.keep(OptionRow::new("icons", t("about.icons"), t("about.icons_credits"), Control::None), ""),
                ],
            ),
        ]
    }

    /// Les bibliothèques d'une catégorie de la pile.
    pub fn about_stack(&self, cat: &str) -> &'static [(&'static str, &'static str, &'static str, &'static str)] {
        STACK.iter().find(|(c, _)| *c == cat).map(|(_, l)| *l).unwrap_or(&[])
    }
}
