//! Bibliothèque retrouvée ailleurs (partage réseau remonté sous un autre nom) :
//! réécrit le début des chemins enregistrés, d'un bloc.

use sqlx::SqlitePool;

/// Toutes les colonnes qui gardent un chemin absolu.
const COLONNES: [(&str, &str); 7] = [
    ("library_dirs", "path"),
    ("library_files", "path"),
    ("library_cache", "path"),
    ("library_albums", "album_dir"),
    ("queue_tracks", "path"),
    ("recent_files", "path"),
    ("track_liked", "path"),
];

/// Remplace `ancien` par `nouveau` en tête des chemins (le dossier lui-même et tout
/// ce qu'il contient), en une transaction. Rend le nombre de fichiers déplacés.
pub async fn relocate_prefix(pool: &SqlitePool, ancien: &str, nouveau: &str) -> Result<u64, String> {
    let ancien = ancien.trim_end_matches('/');
    let nouveau = nouveau.trim_end_matches('/');
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.map_err(|e| format!("transaction : {e}"))?;
    let mut fichiers = 0;
    for (table, colonne) in COLONNES {
        // File, récents et likes : un doublon déjà présent sous le nouveau chemin
        // l'emporte, l'ancienne ligne reste sans conséquence.
        let ignorer = matches!(table, "queue_tracks" | "recent_files" | "track_liked");
        let sql = format!(
            "UPDATE {} {table} SET {colonne} = ?2 || substr({colonne}, length(?1) + 1) \
             WHERE {colonne} = ?1 OR substr({colonne}, 1, length(?1) + 1) = ?1 || '/'",
            if ignorer { "OR IGNORE" } else { "" },
        );
        // Tables et colonnes viennent de la liste fixe ci-dessus ; les chemins sont liés.
        let fait = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(ancien)
            .bind(nouveau)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("{table}.{colonne} : {e}"))?;
        if table == "library_files" {
            fichiers = fait.rows_affected();
        }
    }
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
