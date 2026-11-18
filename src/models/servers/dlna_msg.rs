#[derive(Debug, Clone, PartialEq)]
pub enum DlnaMsg {
    /// Ouvre le dossier n de la liste.
    Open(usize),
    /// Revient à l'étape n du fil d'Ariane.
    Trail(usize),
    Toggle(usize),
    Unselect(usize),
}
