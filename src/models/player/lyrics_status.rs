#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LyricsStatus {
    #[default]
    Idle,
    Loading,
    Ready,
    Empty,
    Error,
}
