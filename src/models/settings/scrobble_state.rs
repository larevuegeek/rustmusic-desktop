use crate::models::settings::*;

#[derive(Default)]
pub struct ScrobbleState {
    pub lb: Option<Account>,
    pub lb_token: String,
    pub lb_busy: bool,
    pub lb_error: Option<String>,
    pub lb_confirm: bool,
    pub lf: Option<Account>,
    pub lf_builtin: bool,
    pub lf_own: bool,
    pub lf_key: String,
    pub lf_secret: String,
    pub lf_busy: bool,
    /// Page d'autorisation ouverte, en attente.
    pub lf_wait: Option<String>,
    pub lf_error: Option<String>,
    pub lf_confirm: bool,
    /// Sondage en cours : un nouveau numéro arrête le précédent.
    pub lf_round: u64,
}
