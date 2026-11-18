use crate::models::app::Msg;

/// Une entrée ou un mode d'écoute proposé.
pub struct AmpChoice {
    pub id: String,
    pub name: String,
    pub icon: &'static str,
    pub selected: bool,
    pub msg: Msg,
}
