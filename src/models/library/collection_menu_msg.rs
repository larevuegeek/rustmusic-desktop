#[derive(Debug, Clone, PartialEq)]
pub enum CollectionMenuMsg {
    PlayAll,
    PlayNext,
    Enqueue,
    SubPlaylists,
    SubCover,
    AddTo(i64),
    Pin,
    FetchCover,
    SearchCover,
    EditTags,
    Close,
}
