/// État de la page guide : mode du schéma, étape choisie (la deuxième d'abord), terme éclairé.
pub struct GuideState {
    pub exclusive: bool,
    pub step: usize,
    pub term: Option<String>,
}
