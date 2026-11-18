#[derive(Debug, Clone, PartialEq)]
pub enum TagsMsg {
    Pick(String),
    Toggle(String),
    ToggleAll,
    Edit(String, usize),
    Draft(String),
    Commit,
    Sort(usize),
    /// Menu ouvert : « tags », « files », « issues », « columns », ou « col:<n> ».
    Menu(Option<String>),
    Column(usize),
    /// Action de colonne : 0 recopier, 1 majuscules, 2 effacer, 3 masquer.
    ColumnAction(usize, u8),
    OnlyIssues,
    Panel,
    Undo,
    RevertAll,
    Write,
    /// Panneau : saisie d'un champ, puis application à la sélection.
    InspectorDraft(usize, String),
    InspectorApply(usize),
    Revert(usize),
    Genre(String),
    Number,
    FillAlbumArtist,
    Leave,
    KeepEditing,
    NoticeEnd(u64),
    /// Nettoyage appliqué : n valeurs (`workshop.cleaned`).
    Cleaned(usize),
    /// Complété en ligne : valeurs, fichiers (`workshop.completed`).
    Completed(usize, usize),
}
