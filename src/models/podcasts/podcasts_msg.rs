use karbon_ui::ui::runtime::Key;

#[derive(Debug, Clone, PartialEq)]
pub enum PodcastsMsg {
    Input(String),
    Key(Key),
    Submit,
    Clear,
    Filter(String),
    Genre(Option<u32>),
    Country(&'static str),
    Sort(&'static str),
    PopularSort(&'static str),
    ShowAll,
    /// Carte de l'annuaire (`r` résultats, `p` populaires) : sa page, en s'abonnant si besoin.
    Open(char, usize),
    Follow(char, usize),
    PlayLatest(i64),
    Pin(i64, bool),
    CheckAll,
    EmptyListen,
    ListMenu,
}
