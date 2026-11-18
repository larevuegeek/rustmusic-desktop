use crate::models::app::Msg;

#[derive(Debug, Clone)]
pub struct Ghost {
    pub label: String,
    pub icon: &'static str,
    pub danger: bool,
    pub busy: bool,
    pub disabled: bool,
    pub title: Option<String>,
    pub msg: Msg,
}
