#[derive(Debug, Clone, PartialEq)]
pub enum MixSource {
    Kind { kind: &'static str, genre: Option<String>, decade: Option<i64> },
    Playlist(i64),
}
