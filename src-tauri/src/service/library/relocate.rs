//! Bibliothèque retrouvée ailleurs (partage réseau remonté sous un autre nom) :
//! réécrit le début des chemins enregistrés, d'un bloc.

use sqlx::SqlitePool;

use crate::repository::library::track_path_repository::TrackPathRepository;

/// Remplace `ancien` par `nouveau` en tête des chemins (le dossier lui-même et tout
/// ce qu'il contient), en une transaction. Rend le nombre de fichiers déplacés.
pub async fn relocate_prefix(pool: &SqlitePool, ancien: &str, nouveau: &str) -> Result<u64, String> {
    let ancien = ancien.trim_end_matches('/');
    let nouveau = nouveau.trim_end_matches('/');
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.map_err(|e| format!("transaction : {e}"))?;
    let fichiers = TrackPathRepository::replace_prefix(&mut tx, ancien, nouveau).await?;
    tx.commit().await.map_err(|e| format!("validation : {e}"))?;
    Ok(fichiers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn rewrites_the_folder_and_its_content_but_not_a_lookalike_neighbour() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO library (id, profil_id, name) VALUES (1, 1, 'test')").execute(&pool).await.unwrap();
        let ancien = "/run/user/1000/gvfs/smb-share:server=10.0.0.170,share=music";
        for (id, chemin) in [
            ("d1", ancien.to_string()),
            ("d2", format!("{ancien}/Mp3 - 320 kbits")),
            // Même début de nom, autre partage : ne doit pas bouger.
            ("d3", format!("{ancien}2/Flac")),
        ] {
            sqlx::query("INSERT INTO library_dirs (id, library_id, path, name) VALUES (?, 1, ?, 'x')")
                .bind(id).bind(chemin).execute(&pool).await.unwrap();
        }

        // Toutes les colonnes de la liste existent : sinon la requête échouerait ici.
        let nouveau = "/run/user/1000/gvfs/smb-share:server=nas.local,share=music";
        relocate_prefix(&pool, ancien, &format!("{nouveau}/")).await.unwrap();

        let chemins: Vec<String> = sqlx::query_scalar("SELECT path FROM library_dirs ORDER BY id")
            .fetch_all(&pool).await.unwrap();
        assert_eq!(chemins, vec![
            nouveau.to_string(),
            format!("{nouveau}/Mp3 - 320 kbits"),
            format!("{ancien}2/Flac"),
        ]);
    }
}
