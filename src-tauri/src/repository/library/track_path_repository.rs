//! Le chemin d'un morceau, partout où il est stocké.
//!
//! # Sept tables, une seule fonction
//! `library_files`, `library_cache`, `recent_files`, `track_liked` et
//! `queue_tracks` portent chacune le chemin absolu du fichier. En oublier une
//! **orpheline en silence** : l'historique d'écoute, les « j'aime », le cache
//! d'analyse — ou, pour la file d'attente, rend le morceau illisible sans que
//! rien ne l'explique.
//!
//! C'est exactement ce qui est arrivé : `queue_tracks` manquait à l'inventaire
//! d'origine. Rassembler les cinq mises à jour dans une seule fonction est ce
//! qui empêche l'oubli de se reproduire — il n'y a plus qu'un endroit à
//! compléter le jour où une table de plus stockera un chemin.
//!
//! `library_dirs` et la pochette de bibliothèque en portent aussi, mais ce sont
//! des dossiers : ils ne bougent pas quand un fichier est renommé.

/// Nom de fichier seul.
fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

pub struct TrackPathRepository;

impl TrackPathRepository {
    /// Fait suivre un chemin dans toutes les tables qui le stockent.
    ///
    /// Prend un exécuteur et non un pool : l'appelant travaille dans une
    /// transaction, et les cinq mises à jour doivent y vivre ou n'avoir pas
    /// lieu. Une base qui décrirait une arborescence à moitié déplacée serait
    /// pire que l'échec.
    pub async fn repoint(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        from: &str,
        to: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE library_files SET path = ?, filename = ? WHERE path = ?")
            .bind(to)
            .bind(file_name(to))
            .bind(from)
            .execute(&mut **tx)
            .await?;

        sqlx::query("UPDATE library_cache SET path = ? WHERE path = ?")
            .bind(to)
            .bind(from)
            .execute(&mut **tx)
            .await?;

        sqlx::query("UPDATE recent_files SET path = ? WHERE path = ?")
            .bind(to)
            .bind(from)
            .execute(&mut **tx)
            .await?;

        sqlx::query("UPDATE track_liked SET path = ? WHERE path = ?")
            .bind(to)
            .bind(from)
            .execute(&mut **tx)
            .await?;

        sqlx::query("UPDATE queue_tracks SET path = ? WHERE path = ?")
            .bind(to)
            .bind(from)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    /// Parmi ces chemins, ceux que la base connaît déjà.
    ///
    /// `library_cache.path` et `recent_files.path` sont UNIQUE : renommer vers
    /// un chemin déjà enregistré — souvent la trace d'un fichier supprimé du
    /// disque mais pas de la bibliothèque — échouerait au milieu du lot. On le
    /// détecte donc à l'aperçu.
    pub async fn known(
        pool: &sqlx::SqlitePool,
        paths: &[String],
    ) -> Result<std::collections::HashSet<String>, sqlx::Error> {
        let mut known = std::collections::HashSet::new();
        if paths.is_empty() {
            return Ok(known);
        }

        // Par lots de cinq cents : SQLite plafonne le nombre de paramètres
        // liés, et un album de trente morceaux n'est pas la limite — une
        // bibliothèque entière, si.
        for chunk in paths.chunks(500) {
            let holders = vec!["?"; chunk.len()].join(",");
            let sql = format!(
                "SELECT path FROM library_cache WHERE path IN ({holders})
                 UNION SELECT path FROM recent_files WHERE path IN ({holders})"
            );
            let mut query = sqlx::query_as::<_, (String,)>(&sql);
            for path in chunk {
                query = query.bind(path);
            }
            for path in chunk {
                query = query.bind(path);
            }
            known.extend(query.fetch_all(pool).await?.into_iter().map(|r| r.0));
        }
        Ok(known)
    }
}
