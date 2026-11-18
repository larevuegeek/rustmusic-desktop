/// Un niveau du fil d'Ariane : `None` pour les niveaux repliés (« … »).
pub struct Crumb {
    pub name: String,
    pub path: String,
    pub last: bool,
    pub cache: Option<String>,
}
