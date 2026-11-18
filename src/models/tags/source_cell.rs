/// Une cellule : état 0 diffère, 1 identique, 2 non fourni.
#[derive(Debug, Clone)]
pub struct SourceCell {
    pub field: &'static str,
    pub from: String,
    pub to: String,
    pub state: u8,
}
