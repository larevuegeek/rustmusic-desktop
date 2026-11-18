/// Une source.
#[derive(Debug, Clone)]
pub struct Source {
    /// `None` : toutes ; `Some(0)` : cet ordinateur ; sinon le serveur.
    pub id: Option<i64>,
    pub name: String,
    pub icon: &'static str,
    pub tint: &'static str,
    pub detail: String,
    pub online: Option<bool>,
}
