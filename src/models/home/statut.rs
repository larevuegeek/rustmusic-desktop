/// La ligne d'état du lecteur (format, qualité, débit, chemin).
#[derive(Debug, Clone, Default)]
pub struct Statut {
    pub format: String,
    pub quality: String,
    pub output: String,
    pub bitrate: String,
    pub folder: String,
    pub file: String,
}
