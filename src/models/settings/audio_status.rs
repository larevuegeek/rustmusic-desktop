/// Qualité de décodage et Replay Gain, lus à l'ouverture de la page.
#[derive(Debug, Clone)]
pub struct AudioStatus {
    pub setting: String,
    pub resolved: String,
    pub virt: Option<String>,
    pub cores: usize,
    pub rg_mode: String,
    pub preamp: f32,
}
