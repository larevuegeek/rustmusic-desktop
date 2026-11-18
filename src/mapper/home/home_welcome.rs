//! Accueil sans bibliothèque : les trois étapes et les sections à venir.
use crate::models::app::{Route, Section};
use crate::models::home::{WelcomeSection, WelcomeStep};
use karbon_ui::intl::t;

const STEPS: [(&str, &str, &str); 3] = [
    ("material-symbols:create-new-folder-outline-rounded", "home_welcome.step1_title", "home_welcome.step1_desc"),
    ("material-symbols:auto-awesome-outline-rounded", "home_welcome.step2_title", "home_welcome.step2_desc"),
    ("material-symbols:headphones-outline-rounded", "home_welcome.step3_title", "home_welcome.step3_desc"),
];

pub(crate) fn welcome_steps() -> Vec<WelcomeStep> {
    STEPS
        .iter()
        .enumerate()
        .map(|(i, (icon, title, desc))| WelcomeStep { number: i + 1, icon, title: t(title), desc: t(desc) })
        .collect()
}

/// Les onglets de la bibliothèque, chacun avec ses barres fantômes de longueurs variées.
pub(crate) fn welcome_sections() -> Vec<WelcomeSection> {
    Section::ALL
        .iter()
        .enumerate()
        .map(|(i, (_, _, label, icon))| WelcomeSection {
            icon,
            label: t(label),
            route: Route::Import,
            bars: [88. - (i % 3) as f32 * 12., 60. + (i % 2) as f32 * 18.],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_numbered_steps_and_one_tile_per_library_section() {
        let steps = welcome_steps();
        assert_eq!(steps.iter().map(|s| s.number).collect::<Vec<_>>(), [1, 2, 3]);
        let sections = welcome_sections();
        assert_eq!(sections.len(), Section::ALL.len());
        assert!(sections.iter().all(|s| s.bars.iter().all(|w| (40. ..=90.).contains(w))));
    }
}
