use crate::models::app::Route;

/// Un chiffre de l'en-tête.
#[derive(Debug, Clone)]
pub struct Stat {
    pub n: String,
    pub label: &'static str,
    pub route: Route,
}
