/// Le dessin d'une carte de choix : thème, disposition de la bibliothèque, boutons de
/// fenêtre.
#[derive(Debug, Clone, PartialEq)]
pub enum Preview {
    /// « auto », « light » ou « dark ».
    Theme(&'static str),
    /// « sidebar », « top » ou « both ».
    Library(&'static str),
    /// Style (« macos », « windows », « linux ») et boutons à gauche.
    Window(&'static str, bool),
}
