#[derive(Debug, Clone, PartialEq)]
pub enum ArtistMsg {
    /// `year`, `album` (regroupés) ou `title` (à plat).
    Sort(String),
    Toggle(String),
    ToggleAll,
    /// Carte de la discographie : son groupe s'ouvre (l'album, s'il n'a pas de groupe).
    GoTo(String),
    Covers,
    PlayAll,
    Shuffle,
    LikeAll,
    PlayGroup(String),
}
