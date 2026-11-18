/// Ce que la connexion a trouvé : adresse retenue, nom, type, dossiers (identifiant, nom).
#[derive(Debug, Clone)]
pub struct Probe {
    pub url: String,
    pub name: String,
    pub kind_label: String,
    pub folders: Vec<(String, String)>,
}
