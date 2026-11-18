/// `Preset` : une recette, point de départ modifiable.
pub struct Preset {
    pub key: &'static str,
    pub icon: &'static str,
    pub color: &'static str,
    /// Conditions (champ, opérateur, valeur), toutes requises.
    pub rules: &'static [(&'static str, &'static str, i64)],
    pub count: Option<i64>,
    pub sort: &'static str,
    pub desc: bool,
}
