use crate::models::app::Msg;

#[derive(Debug, Clone)]
pub struct Pill {
    pub label: String,
    pub checked: bool,
    pub disabled: bool,
    pub title: Option<String>,
    pub msg: Msg,
}
