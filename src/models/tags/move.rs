#[derive(Debug, Clone)]
pub struct Move {
    pub from: String,
    pub from_name: String,
    pub to: String,
    pub to_name: String,
    pub unchanged: bool,
    pub blocked: bool,
    /// La raison d'un blocage, traduite.
    pub reason: String,
}
