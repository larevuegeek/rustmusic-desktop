use crate::models::tags::*;

pub struct Inspector {
    pub title: String,
    pub sub: String,
    pub cover_state: String,
    pub cover: Option<String>,
    pub groups: Vec<(String, Vec<InspectorField>)>,
    pub genres: Vec<String>,
    /// « S'applique à 3 morceaux » : avant, nombre en gras, après.
    pub applies: (String, String, String),
}
