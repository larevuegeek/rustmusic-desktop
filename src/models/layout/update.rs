/// Où en est la mise à jour de l'application.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Update {
    #[default]
    Idle,
    Available(String),
    Downloading(String, u64, Option<u64>),
    Installing,
    Ready(String),
    Error(String),
}
