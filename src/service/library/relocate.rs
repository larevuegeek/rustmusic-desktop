//! Bibliothèque retrouvée ailleurs (partage réseau remonté sous un autre nom) :
//! réécrit le début des chemins enregistrés, d'un bloc.

use sqlx::SqlitePool;

use crate::repository::library::track_path_repository::TrackPathRepository;

/// Remplace `old_path` par `new_one` en tête des chemins (le dossier lui-même et tout
/// ce qu'il contient), en une transaction. Rend le nombre de fichiers déplacés.
pub async fn relocate_prefix(pool: &SqlitePool, old_path: &str, new_one: &str) -> Result<u64, String> {
    let old_path = old_path.trim_end_matches('/');
    let new_one = new_one.trim_end_matches('/');
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.map_err(|e| format!("transaction : {e}"))?;
    let files = TrackPathRepository::replace_prefix(&mut tx, old_path, new_one).await?;
    tx.commit().await.map_err(|e| format!("validation : {e}"))?;
    Ok(files)
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
        let old_path = "/run/user/1000/gvfs/smb-share:server=10.0.0.170,share=music";
        for (id, path_part) in [
            ("d1", old_path.to_string()),
            ("d2", format!("{old_path}/Mp3 - 320 kbits")),
            // Même début de nom, autre partage : ne doit pas bouger.
            ("d3", format!("{old_path}2/Flac")),
        ] {
            sqlx::query("INSERT INTO library_dirs (id, library_id, path, name) VALUES (?, 1, ?, 'x')")
                .bind(id).bind(path_part).execute(&pool).await.unwrap();
        }

        // Toutes les colonnes de la liste existent : sinon la requête échouerait ici.
        let new_one = "/run/user/1000/gvfs/smb-share:server=nas.local,share=music";
        relocate_prefix(&pool, old_path, &format!("{new_one}/")).await.unwrap();

        let paths: Vec<String> = sqlx::query_scalar("SELECT path FROM library_dirs ORDER BY id")
            .fetch_all(&pool).await.unwrap();
        assert_eq!(paths, vec![
            new_one.to_string(),
            format!("{new_one}/Mp3 - 320 kbits"),
            format!("{old_path}2/Flac"),
        ]);
    }
}
