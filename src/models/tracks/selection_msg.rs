#[derive(Debug, Clone, PartialEq)]
pub enum SelectionMsg {
    /// Entrer ou sortir du mode sélection.
    Toggle,
    Stop,
    /// Cocher toute la liste de la page (la page Morceaux : toute la liste filtrée).
    SelectAll,
    DeselectAll,
    Play,
    Queue,
    PlaylistMenu,
    AddTo(i64),
    Workshop,
    EditTags,
}
