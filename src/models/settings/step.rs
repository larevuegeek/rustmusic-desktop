/// Une étape du schéma : clé, icône, teinte, choisie, effacée (mixer en exclusif).
pub struct Step {
    pub index: usize,
    pub key: &'static str,
    pub icon: &'static str,
    pub hue: i32,
    pub selected: bool,
    pub bypassed: bool,
}
