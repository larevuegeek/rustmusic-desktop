use crate::app::music_app::MusicApp;

/// `SOURCES` : la clé de traduction d'une source, sinon son nom.
pub fn source_label(source: &str) -> String {
    match source {
        "embedded" => karbon_ui::intl::t("mini.src_embedded"),
        "sidecar" => karbon_ui::intl::t("mini.src_sidecar"),
        "lrclib" => karbon_ui::intl::t("mini.src_lrclib"),
        "manual" => karbon_ui::intl::t("mini.src_manual"),
        "server" => karbon_ui::intl::t("mini.src_server"),
        s => s.to_string(),
    }
}

/// Sans pochette, un dégradé choisi par le chemin (`bg-linear-to-br`,
/// trois teintes Tailwind à 40 %, 30 %, 40 %).
pub fn lyrics_fallback(path: Option<&str>) -> Vec<karbon_ui::ui::prelude::Background> {
    use karbon_ui::ui::prelude::{Color, GradientStop, LinearGradient};
    const PALETTES: [[(u8, u8, u8); 3]; 6] = [
        [(4, 120, 87), (19, 78, 74), (22, 78, 99)],
        [(109, 40, 217), (88, 28, 135), (112, 26, 117)],
        [(190, 18, 60), (131, 24, 67), (127, 29, 29)],
        [(180, 83, 9), (124, 45, 18), (127, 29, 29)],
        [(29, 78, 216), (49, 46, 129), (88, 28, 135)],
        [(4, 120, 87), (20, 83, 45), (54, 83, 20)],
    ];
    let p = match path {
        Some(p) => PALETTES[p.chars().map(|c| c as usize).sum::<usize>() % 6],
        None => [(6, 78, 59), (76, 29, 149), (30, 58, 138)],
    };
    let c = |(r, g, b): (u8, u8, u8), a: u8| Color::rgba(r, g, b, a);
    // `to bottom right` : 135° en CSS, 45° pour l'API (0 vers la droite).
    vec![LinearGradient::new(
        45.,
        [
            GradientStop { position: 0., color: c(p[0], 102) },
            GradientStop { position: 0.5, color: c(p[1], 77) },
            GradientStop { position: 1., color: c(p[2], 102) },
        ],
    )
    .into()]
}

impl crate::models::player::LyricsState {
    /// Le message d'erreur du chargement, vide sans erreur.
    pub fn error_text(&self) -> &str {
        self.error.as_deref().unwrap_or_default()
    }

    /// Les paroles non synchronisées, quand il y en a.
    pub fn plain_text(&self) -> Option<&str> {
        self.plain.as_deref().filter(|p| !p.is_empty())
    }
}

impl MusicApp {
    /// La dernière ligne commencée.
    pub fn lyrics_active(&self) -> Option<usize> {
        let ms = self.data.player.position * 1000.;
        let l = &self.views.lyrics.lines;
        let n = l.iter().take_while(|l| l.at <= ms).count();
        n.checked_sub(1)
    }
}
