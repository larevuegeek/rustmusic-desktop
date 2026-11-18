/// Une proposition : fichier, champ (indice), avant, après.
#[derive(Debug, Clone, PartialEq)]
pub struct Proposal {
    pub path: String,
    pub field: usize,
    pub from: String,
    pub to: String,
}
