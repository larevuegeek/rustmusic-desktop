use crate::models::tracks::Tier;

/// Un résultat affiché ; `n` : son rang dans `results` (ouverture).
#[derive(Debug, Clone)]
pub struct Hit {
    pub n: usize,
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub cover: Option<String>,
    pub round: bool,
    pub avatar: Option<(String, f32)>,
    pub type_label: String,
    pub quality: Option<(Tier, String)>,
    pub duration: String,
}
