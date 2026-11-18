/// Une ligne de comparaison : clé, valeur du fichier, proposée, état (0 diffère, 1 même, 2 absent).
#[derive(Debug, Clone)]
pub struct CompareRow {
    pub key: &'static str,
    pub from: String,
    pub to: String,
    pub state: u8,
}
