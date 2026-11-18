use crate::models::app::Route;

/// Une étape de l'accueil sans bibliothèque.
#[derive(Debug, Clone)]
pub struct WelcomeStep {
    pub number: usize,
    pub icon: &'static str,
    pub title: String,
    pub desc: String,
}

/// Une section à venir : tuile fantôme qui mène à l'import.
#[derive(Debug, Clone)]
pub struct WelcomeSection {
    pub icon: &'static str,
    pub label: String,
    pub route: Route,
    /// Largeurs des deux premières barres fantômes, en pourcentage.
    pub bars: [f32; 2],
}
