/// Une carte « Partagé » ou « Exclusif » du guide : pour qui, avantages, limites.
pub struct GuideMode {
    pub id: &'static str,
    pub icon: &'static str,
    /// Teinte de l'icône (`--h`).
    pub hue: i32,
    pub name: String,
    pub for_whom: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
}
