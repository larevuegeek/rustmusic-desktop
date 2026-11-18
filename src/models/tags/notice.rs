/// Un avis en bas de page, avec ou sans « Annuler ».
pub struct Notice {
    pub text: String,
    pub undoable: bool,
    pub id: u64,
}
