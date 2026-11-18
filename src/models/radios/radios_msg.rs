use karbon_ui::ui::runtime::Key;

#[derive(Debug, Clone, PartialEq)]
pub enum RadiosMsg {
    Input(String),
    Key(Key),
    Submit,
    Clear,
    Genre(Option<&'static str>),
    Country(&'static str),
    ShowAll,
    /// Lecture : (liste, rang) — `r` résultats, `p` populaires, `f` favorites, `h` récentes.
    Play(char, usize),
    Favorite(char, usize),
    TitlesAll,
    TitlesClear,
    /// Menu d'une station (liste, rang, position), puis ses actions.
    Menu(char, usize, crate::models::tracks::TrackMenuAt),
    MenuClose,
    MenuPlay,
    MenuFavorite,
    MenuSite,
    MenuVariant(usize),
}
