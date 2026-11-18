use karbon_ui::ui::prelude::Color;

/// Une part de la barre de stockage des téléchargements : une émission, ou « Autres ».
pub struct StorageSegment {
    pub key: String,
    pub title: String,
    /// « 1,2 Go ».
    pub size: String,
    /// Poids dans la barre (octets).
    pub weight: f32,
    pub color: Color,
}
