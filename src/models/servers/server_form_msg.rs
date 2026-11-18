#[derive(Debug, Clone, PartialEq)]
pub enum ServerFormMsg {
    OpenAdd,
    OpenFolders(i64),
    Kind(usize),
    Url(String),
    User(String),
    Pass(String),
    Key(String),
    ToggleKey,
    Discover,
    /// Un serveur trouvé sur le réseau : sa page de connexion.
    Found(usize),
    Connect,
    /// « Retour » après la connexion, ou « Annuler ».
    Back,
    Name(String),
    Toggle(String),
    Submit,
}
