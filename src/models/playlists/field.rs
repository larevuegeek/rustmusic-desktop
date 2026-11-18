/// Un champ de la bibliothèque, ou un tag recensé (toujours du texte).
#[derive(Debug, Clone)]
pub struct Field {
    pub key: String,
    pub label: String,
    pub kind: String,
}
