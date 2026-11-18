/// Une sortie audio.
#[derive(Debug, Clone)]
pub struct Output {
    pub name: String,
    pub label: String,
    pub detail: String,
    pub icon: &'static str,
    pub hires: bool,
}
