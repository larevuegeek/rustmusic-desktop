//! La page Feedback : les informations sur l'installation à joindre à un rapport de bug.

use crate::app::music_app::MusicApp;
use crate::models::feedback::FeedbackView;
use karbon_ui::intl::{self, t, t_with};

/// Adresse des tickets du dépôt.
pub const ISSUES_URL: &str = "https://github.com/larevuegeek/rustmusic-desktop/issues/new";

/// Le dépôt.
pub const REPOSITORY_URL: &str = "https://github.com/larevuegeek/rustmusic-desktop";

/// L'adresse de contact.
pub const CONTACT_EMAIL: &str = "contact@rustmusic.dev";

/// « Windows · x86_64 ».
fn system_line() -> String {
    let os = match std::env::consts::OS {
        "windows" => "Windows",
        "macos" => "macOS",
        "linux" => "Linux",
        other => other,
    };
    format!("{os} · {}", std::env::consts::ARCH)
}

impl MusicApp {
    pub fn feedback_view(&self) -> FeedbackView {
        let (outputs, selected) = crate::mapper::player::outputs::outputs(&self.data);
        let output = outputs.get(selected).map(|o| o.label.clone()).unwrap_or_else(|| t("feedback.diag_none"));
        let library = self
            .data
            .library()
            .map(|l| {
                let tracks = if l.total_tracks == 1 { t("home.track_one") } else { t_with("home.tracks_n", &[("n", &intl::number(l.total_tracks))]) };
                format!("{} · {tracks}", l.name)
            })
            .unwrap_or_else(|| t("feedback.diag_none"));
        let diagnostics = vec![
            (t("feedback.diag_version"), crate::mapper::settings::version_line()),
            (t("feedback.diag_system"), system_line()),
            (t("feedback.diag_output"), output),
            (t("feedback.diag_library"), library),
        ];
        let clipboard = diagnostics.iter().map(|(k, v)| format!("{k} : {v}")).collect::<Vec<_>>().join("\n");
        FeedbackView { diagnostics, clipboard }
    }
}
