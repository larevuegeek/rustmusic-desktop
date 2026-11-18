/// L'en-tête (liant, retirer) et le pied (ajouter) d'un groupe.
#[derive(Debug, Clone, Default)]
pub struct GroupHead {
    pub path: Vec<usize>,
    pub all: bool,
    /// Absent pour la racine, qui ne se supprime pas.
    pub removable: bool,
    /// « Groupe » n'est proposé que si la profondeur le permet.
    pub can_group: bool,
}
