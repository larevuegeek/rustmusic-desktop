use crate::models::settings::*;
use crate::models::app::Msg;

/// Aperçu, intitulé et coche ronde.
#[derive(Debug, Clone)]
pub struct Card {
    pub label: String,
    pub title: Option<String>,
    pub selected: bool,
    pub msg: Msg,
    pub preview: Preview,
}
