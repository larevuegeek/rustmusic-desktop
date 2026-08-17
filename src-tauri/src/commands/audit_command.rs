//! Inventaire de ce qu'il y a à corriger dans une bibliothèque.
//!
//! # Une seule requête
//! Chercher séparément les titres vides, les années manquantes, les albums
//! incohérents et les doublons ferait autant de parcours de la table. Sur une
//! bibliothèque de cinquante mille morceaux, c'est ce qui sépare un écran
//! instantané d'un écran qu'on n'ouvre plus.
//!
//! On lit donc tout une fois, en projetant les seules colonnes utiles, et
//! `tag_audit::audit` fait le reste en mémoire. Ce module-ci ne décide rien :
//! il traduit des lignes SQL en structures neutres, et rend des constats.

use serde::Serialize;
use tauri::State;

use crate::core::tag_audit::audit::{self, TrackRow};
use crate::state::AppState;

/// Une catégorie de constat, telle que l'interface la reçoit.
#[derive(Debug, Serialize)]
pub struct AuditGroupView {
    /// Identifiant stable — c'est la clé de traduction côté interface.
    pub kind: String,
    pub severity: String,
    pub count: usize,
    /// Les fichiers concernés, à verser tels quels dans l'atelier.
    pub paths: Vec<String>,
    /// Quelques noms, pour que la ligne dise de quoi elle parle.
    pub samples: Vec<String>,
}

/// Ce que l'écran « à corriger » affiche.
#[derive(Debug, Serialize)]
pub struct AuditReport {
    /// Nombre de morceaux passés en revue — le dénominateur de tout le reste.
    pub scanned: usize,
    pub groups: Vec<AuditGroupView>,
}

/// Une ligne telle que SQLite la rend.
///
/// Les champs de tags viennent de `library_cache` et non de `library_tracks` :
/// c'est le cache qui porte l'année, le genre et l'artiste d'album, que la
/// table des morceaux ne stocke pas.
#[derive(sqlx::FromRow)]
struct Row {
    path: String,
    extension: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    year: Option<String>,
    track_number: Option<i64>,
    disc_number: Option<i64>,
    duration: Option<f64>,
    thumbnail_path: Option<String>,
}

/// Passe une bibliothèque en revue.
#[tauri::command]
pub async fn audit_library(
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<AuditReport, String> {
    // `is_available = 1` : un fichier débranché avec le disque externe n'est
    // pas un fichier à corriger. Le signaler enverrait chercher des morceaux
    // que l'atelier ne saurait pas ouvrir.
    let rows: Vec<Row> = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            lf.path                        AS path,
            lower(COALESCE(lf.extension, '')) AS extension,
            lc.title                       AS title,
            lc.artist                      AS artist,
            lc.album                       AS album,
            lc.album_artist                AS album_artist,
            lc.year                        AS year,
            lc.track_number                AS track_number,
            lc.disc_number                 AS disc_number,
            lc.duration                    AS duration,
            lc.thumbnail_path              AS thumbnail_path
        FROM library_files lf
        JOIN library_cache lc ON lc.id = lf.cache_id
        WHERE lf.library_id = ? AND lf.is_available = 1
        "#,
    )
    .bind(library_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| format!("Lecture de la bibliothèque : {e}"))?;

    let scanned = rows.len();

    let tracks: Vec<TrackRow> = rows
        .into_iter()
        .map(|r| TrackRow {
            path: r.path,
            title: r.title.unwrap_or_default(),
            artist: r.artist.unwrap_or_default(),
            album: r.album.unwrap_or_default(),
            album_artist: r.album_artist.unwrap_or_default(),
            year: r.year.unwrap_or_default(),
            track_number: r.track_number.and_then(|n| u32::try_from(n).ok()),
            disc_number: r.disc_number.and_then(|n| u32::try_from(n).ok()),
            duration: r.duration,
            // Une vignette vide vaut absence : la colonne est renseignée à
            // la chaîne vide par certains chemins d'import.
            has_cover: r.thumbnail_path.is_some_and(|p| !p.trim().is_empty()),
            extension: r.extension.unwrap_or_default(),
        })
        .collect();

    // Le calcul est purement local et peut porter sur cinquante mille lignes :
    // sur la boucle asynchrone, il figerait tout le reste de l'application le
    // temps de s'exécuter.
    let groups = tokio::task::spawn_blocking(move || audit::analyse(&tracks))
        .await
        .map_err(|e| format!("Analyse de la bibliothèque : {e}"))?;

    Ok(AuditReport {
        scanned,
        groups: groups
            .into_iter()
            .map(|g| AuditGroupView {
                kind: g.kind.to_string(),
                severity: g.severity.as_str().to_string(),
                count: g.paths.len(),
                paths: g.paths,
                samples: g.samples,
            })
            .collect(),
    })
}
