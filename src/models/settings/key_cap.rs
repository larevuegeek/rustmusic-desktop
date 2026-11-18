/// Une touche affichée : son libellé, ou l'icône d'une flèche (Figtree n'en a pas).
#[derive(Debug, Clone)]
pub struct KeyCap {
    pub label: String,
    pub arrow: Option<&'static str>,
}
