use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Les colonnes qui remplissent un `TrackListView`, et d'où elles viennent.
///
/// # Pourquoi ici, à côté de la structure
/// sqlx apparie colonnes et champs **à l'exécution** : une requête à qui il
/// manque une colonne compile parfaitement et échoue à l'usage. Le projet en a
/// déjà fait les frais — la colonne `tags`, ajoutée à la structure, n'avait
/// atteint que cinq des neuf requêtes concernées, et les quatre autres se
/// seraient cassées à la première ouverture d'une page d'album.
///
/// Poser la liste des colonnes contre la déclaration des champs est ce qui
/// donne une chance de les faire évoluer ensemble. Les requêtes écrites avant
/// gardent leur propre copie ; celles qu'on ajoute partent d'ici.
pub const TRACK_VIEW_SELECT: &str = "\
SELECT
    lt.id AS id, lt.title AS title, lt.title_normalized AS title_normalized,
    COALESCE(lt.track_number, lc.track_number) AS track_number,
    COALESCE(lt.disc_number, lc.disc_number, 1) AS disc_number,
    COALESCE(lt.duration, lc.duration) AS duration,
    COALESCE(lt.bitrate, lc.bitrate) AS bitrate,
    COALESCE(lt.sample_rate, lc.sample_rate) AS sample_rate,
    lt.play_count AS play_count, lt.last_played_at AS last_played_at,
    lt.rating AS rating, lt.favorite AS favorite,
    lt.tags AS tags,
    lt.created_at AS created_at, lt.updated_at AS updated_at,
    lf.path AS path, lf.filename AS filename, lf.extension AS extension,
    lf.size AS size, lf.status AS status, lf.is_available AS is_available,
    lf.error_message AS error_message,
    a.id AS artist_id, lat.id AS library_artist_id,
    a.name AS artist, la.id AS album_id, la.title AS album,
    lc.album_artist AS album_artist, lc.year AS year, lc.genre AS genre,
    lc.bits_per_sample AS bits_per_sample, lc.channels AS channels,
    lc.audio_format AS audio_format, lc.mime_type AS mime_type,
    lc.file_size AS file_size, lc.extra_tags AS extra_tags,
    lc.thumbnail_path AS thumbnail_path, lc.last_scanned_at AS last_scanned_at";

/// Les jointures qu'exigent les alias employés ci-dessus.
pub const TRACK_VIEW_FROM: &str = "\
FROM library_tracks lt
INNER JOIN library_files lf ON lf.id = lt.file_id
LEFT JOIN library_cache lc ON lc.id = lt.cache_id
LEFT JOIN library_albums la ON la.id = lt.library_album_id
LEFT JOIN artists a ON a.id = lt.artist_id
LEFT JOIN library_artists lat ON lat.artist_id = a.id AND lat.library_id = lt.library_id";

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrackListView {
    // ===== Identité Track =====
    pub id: String,

    // ===== Infos fichier =====
    pub path: String,
    pub filename: String,
    pub extension: String,
    pub size: i64,
    pub status: String,
    pub is_available: bool,
    pub error_message: Option<String>,

    // ===== Métadonnées audio =====
    pub title: String,
    pub title_normalized: String,
    pub artist_id: Option<String>,
    pub library_artist_id: Option<String>,
    pub album_id: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<i32>,
    pub disc_number: i32,

    // ===== Infos techniques =====
    pub duration: Option<f64>,
    pub bitrate: Option<i32>,
    pub bits_per_sample: Option<i32>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub audio_format: Option<String>,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,

    // ===== Cache enrichi =====
    pub extra_tags: Option<String>,

    /// Jeu complet des tags du fichier, sérialisé en JSON.
    ///
    /// Porté tel quel jusqu'à l'interface, qui y puise les colonnes que
    /// l'utilisateur a choisi d'afficher. Extraire côté SQL supposerait de
    /// connaître ces colonnes au moment de la requête ; les envoyer bruts
    /// laisse le choix à l'affichage, au prix d'une centaine de kilo-octets
    /// par page de cent pistes.
    pub tags: Option<String>,
    pub thumbnail_path: Option<String>,
    pub last_scanned_at: Option<DateTime<Utc>>,

    // ===== Stats utilisateur =====
    pub play_count: i64,
    pub last_played_at: Option<DateTime<Utc>>,
    pub rating: Option<f64>,
    pub favorite: bool,

    // ===== Timestamps =====
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Les champs de la structure, lus dans son propre code source.
    ///
    /// Se relire soi-même est inhabituel, mais c'est ce qui rend le test
    /// automatique : ajouter un champ suffit à le faire échouer si la liste de
    /// colonnes ne suit pas. Une liste de noms recopiée dans le test aurait
    /// simplement dérivé avec le reste.
    fn champs_de_la_structure() -> Vec<String> {
        let source = include_str!("track_list_item_view.rs");
        // Après l'accolade ouvrante : la ligne de déclaration commence elle
        // aussi par `pub`, et serait prise pour un champ nommé « struct ».
        let entete = "pub struct TrackListView {";
        let debut = source.find(entete).expect("déclaration de la structure") + entete.len();
        let corps = &source[debut..];
        let fin = corps.find("\n}").expect("fin de la structure");

        corps[..fin]
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                let reste = l.strip_prefix("pub ")?;
                let nom = reste.split(':').next()?.trim();
                (!nom.is_empty()).then(|| nom.to_string())
            })
            .collect()
    }

    #[test]
    fn la_liste_de_colonnes_couvre_tous_les_champs() {
        let champs = champs_de_la_structure();
        assert!(champs.len() > 30, "lecture de la structure ratée : {champs:?}");

        for champ in &champs {
            assert!(
                TRACK_VIEW_SELECT.contains(&format!("AS {champ}")),
                "le champ « {champ} » n'a pas de colonne dans TRACK_VIEW_SELECT"
            );
        }
    }

    #[test]
    fn chaque_alias_employe_est_joint() {
        // Un alias non joint produit une requête invalide, et l'erreur
        // n'apparaît qu'au moment où on l'exécute.
        for alias in ["lt.", "lf.", "lc.", "la.", "a.", "lat."] {
            if TRACK_VIEW_SELECT.contains(alias) {
                let declare = TRACK_VIEW_FROM.contains(&format!(" {}", alias.trim_end_matches('.')))
                    || TRACK_VIEW_FROM.contains(&format!("{} ", alias.trim_end_matches('.')));
                assert!(declare, "l'alias « {alias} » n'est joint nulle part");
            }
        }
    }
}
