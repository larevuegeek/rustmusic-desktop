use crate::models::home::*;

/// Une carte du panneau « Ensuite » (`Card`).
#[derive(Debug, Clone)]
pub struct Card {
    pub key: String,
    pub title: String,
    pub artist_name: String,
    pub duration: f64,
    /// « 03:42 ».
    pub duration_label: String,
    pub cover_path: Option<String>,
    pub launch: Launch,
}
