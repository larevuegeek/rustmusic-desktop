use crate::models::app::Msg;

/// Graves ou aigus : − valeur +.
pub struct AmpToneRow {
    pub label: String,
    /// « +2 dB ».
    pub value: String,
    pub zero: bool,
    /// `None` : butée atteinte.
    pub minus: Option<Msg>,
    pub plus: Option<Msg>,
    pub first: bool,
}
