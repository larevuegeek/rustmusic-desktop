#[derive(Debug, Clone, PartialEq)]
pub enum FoldersMsg {
    Root,
    Parent,
    /// Ouvre ce dossier (chemin, nom).
    Open(String, String),
    /// Clic sur un fichier : lecture dans l'ordre du dossier.
    PlayFile(String),
    PlayFolder,
    /// En sélection : coche ou décoche le fichier de ce rang (parmi les fichiers du dossier).
    Pick(usize),
    /// Les fichiers du dossier dans l'atelier de tags.
    EditTags,
    Sort(&'static str),
    Covers,
    /// Défilement de la liste : décalage et décalage maximal (fenêtre qui glisse).
    Scrolled(f32, f32),
}
