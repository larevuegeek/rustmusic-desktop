use crate::models::playlists::Group;
use crate::models::playlists::Limit;
use crate::models::playlists::Vocab;

#[derive(Debug, Clone)]
pub struct SmartForm {
    /// Renseigné pour modifier une playlist existante.
    pub playlist_id: Option<i64>,
    pub mix: bool,
    /// Source des morceaux retenus.
    pub source: i64,
    pub name: String,
    pub color: usize,
    pub icon: usize,
    pub group: Group,
    pub limit: Limit,
    /// `None` pendant le chargement.
    pub vocab: Option<Vocab>,
    pub saving: bool,
    pub error: Option<String>,
    pub count: Option<i64>,
    pub counting: bool,
    /// Le compte en attente : un changement plus récent annule le précédent.
    pub generation: u64,
    /// Nom posé par la dernière recette, tant qu'on n'y a pas touché.
    pub preset_name: Option<String>,
    /// La dernière recette appliquée, mise en évidence.
    pub preset: Option<&'static str>,
    /// Saisie de la coupe, gardée telle quelle.
    pub count_text: String,
}
