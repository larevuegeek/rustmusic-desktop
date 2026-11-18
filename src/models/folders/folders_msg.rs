#[derive(Debug, Clone, PartialEq)]
pub enum FoldersMsg {
    Root,
    Parent,
    /// Ouvre ce dossier (chemin, nom).
    Open(String, String),
    /// Clic sur un fichier : lecture dans l'ordre du dossier.
    PlayFile(String),
    PlayFolder,
    Sort(&'static str),
    Covers,
    /// Défilement de la liste : décalage et décalage maximal (fenêtre qui glisse).
    Scrolled(f32, f32),
}
