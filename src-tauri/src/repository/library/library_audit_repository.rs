//! Ce que l'inventaire lit dans la bibliothèque.
//!
//! # Une seule requête, volontairement large
//! Chercher séparément les titres vides, les albums incohérents et les doublons
//! ferait autant de parcours de la table. Sur cinquante mille morceaux, c'est
//! ce qui sépare un écran instantané d'un écran qu'on n'ouvre plus.
//!
//! On projette donc les seules colonnes utiles, une fois, et l'analyse se fait
//! en mémoire — sans SQL, donc testable.

use sqlx::SqlitePool;

/// Une ligne telle que SQLite la rend.
///
/// Les champs de tags viennent de `library_cache` et non de `library_tracks` :
/// c'est le cache qui porte l'année, le genre et l'artiste d'album, que la
/// table des morceaux ne stocke pas.
#[derive(sqlx::FromRow)]
pub struct AuditRow {
    pub path: String,
    pub extension: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<String>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub duration: Option<f64>,
    pub thumbnail_path: Option<String>,
}

pub struct LibraryAuditRepository;

impl LibraryAuditRepository {
    /// Tous les morceaux disponibles d'une bibliothèque.
    ///
    /// `is_available = 1` : un fichier débranché avec son disque externe n'est
    /// pas un fichier à corriger. Le signaler enverrait chercher des morceaux
    /// que l'atelier ne saurait pas ouvrir.
    pub async fn scan(pool: &SqlitePool, library_id: i64) -> Result<Vec<AuditRow>, sqlx::Error> {
        sqlx::query_as::<_, AuditRow>(
            r#"
            SELECT
                lf.path                           AS path,
                lower(COALESCE(lf.extension, '')) AS extension,
                lc.title                          AS title,
                lc.artist                         AS artist,
                lc.album                          AS album,
                lc.album_artist                   AS album_artist,
                lc.year                           AS year,
                lc.track_number                   AS track_number,
                lc.disc_number                    AS disc_number,
                lc.duration                       AS duration,
                lc.thumbnail_path                 AS thumbnail_path
            FROM library_files lf
            JOIN library_cache lc ON lc.id = lf.cache_id
            WHERE lf.library_id = ? AND lf.is_available = 1
            "#,
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
    }
}
