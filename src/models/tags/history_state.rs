use crate::models::tags::*;

#[derive(Debug, Clone, Default)]
pub struct HistoryState {
    pub entries: Vec<Entry>,
    pub loading: bool,
    pub error: Option<String>,
    pub working: Option<String>,
    pub confirming: Option<String>,
    /// Résultat de la dernière annulation : réussis, échecs, premier message.
    pub result: Option<(usize, usize, Option<String>)>,
}
