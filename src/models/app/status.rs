/// État de la lecture (`status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ready,
    Playing,
    Paused,
    Ended,
}
