#[derive(Debug, Clone, PartialEq)]
pub enum AlbumMsg {
    /// « n » (ordre du disque), `title`, `duration` ou une colonne.
    Sort(String),
    Covers,
    OthersByYear(bool),
    PlayAll,
    Shuffle,
    LikeAll,
}
