/// Une racine importée.
pub struct RootItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub files_n: String,
    pub files_label: String,
    pub size: String,
    pub when: String,
    pub state: String,
    /// `ok`, `scanning`, `error`.
    pub class: &'static str,
}
