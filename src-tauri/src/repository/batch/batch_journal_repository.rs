//! Le journal des lots, en base.
//!
//! Tout le SQL du journal vit ici, et nulle part ailleurs. Le service qui
//! déplace des fichiers décide **quoi** journaliser ; ce dépôt sait **comment**
//! l'écrire. Sans cette séparation, changer un jour de moteur de base voudrait
//! dire relire chaque service à la recherche de requêtes égarées.

use sqlx::SqlitePool;

/// Un lot du journal, tel que l'écran d'historique le montre.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct JournalEntry {
    pub id: String,
    pub kind: String,
    pub pattern: Option<String>,
    pub created_at: String,
    pub total: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub undone_at: Option<String>,
}

/// Un élément dont les tags sont à restaurer.
pub struct TagSnapshot {
    pub item_id: i64,
    pub path: String,
    pub blob: String,
}

/// Un déplacement à défaire.
pub struct MoveRecord {
    pub item_id: i64,
    pub before_path: String,
    pub after_path: String,
    /// Date de modification relevée juste après l'opération.
    pub modified_at: Option<i64>,
}

pub struct BatchJournalRepository;

impl BatchJournalRepository {
    /// Ouvre un lot et rend son identifiant.
    pub async fn open(
        pool: &SqlitePool,
        library_id: Option<i64>,
        kind: &str,
        pattern: Option<&str>,
        total: usize,
    ) -> Result<String, sqlx::Error> {
        let row: (String,) = sqlx::query_as(
            r#"
            INSERT INTO batch_journal (library_id, kind, pattern, total)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(library_id)
        .bind(kind)
        .bind(pattern)
        .bind(total as i64)
        .fetch_one(pool)
        .await?;
        Ok(row.0)
    }

    /// Consigne un déplacement réussi.
    pub async fn record_move(
        pool: &SqlitePool,
        batch_id: &str,
        from: &str,
        to: &str,
        modified_at: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO batch_journal_items (batch_id, before_path, after_path, modified_at, status)
            VALUES (?, ?, ?, ?, 'done')
            "#,
        )
        .bind(batch_id)
        .bind(from)
        .bind(to)
        .bind(modified_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Consigne un échec, avec sa raison.
    pub async fn record_failure(
        pool: &SqlitePool,
        batch_id: &str,
        from: &str,
        to: &str,
        error: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO batch_journal_items (batch_id, before_path, after_path, status, error)
            VALUES (?, ?, ?, 'failed', ?)
            "#,
        )
        .bind(batch_id)
        .bind(from)
        .bind(to)
        .bind(error)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Consigne l'état des tags d'un fichier avant réécriture.
    pub async fn record_tags(
        pool: &SqlitePool,
        batch_id: &str,
        path: &str,
        blob: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO batch_journal_items (batch_id, before_path, after_path, tags_before, status)
            VALUES (?, ?, ?, ?, 'done')
            "#,
        )
        .bind(batch_id)
        .bind(path)
        .bind(path)
        .bind(blob)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Met à jour les compteurs du lot.
    pub async fn set_counts(
        pool: &SqlitePool,
        batch_id: &str,
        succeeded: usize,
        failed: usize,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE batch_journal SET succeeded = ?, failed = ? WHERE id = ?")
            .bind(succeeded as i64)
            .bind(failed as i64)
            .bind(batch_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Nature du lot, et s'il a déjà été annulé.
    pub async fn kind(
        pool: &SqlitePool,
        batch_id: &str,
    ) -> Result<Option<(String, bool)>, sqlx::Error> {
        let row: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT kind, CAST(undone_at AS TEXT) FROM batch_journal WHERE id = ?")
                .bind(batch_id)
                .fetch_optional(pool)
                .await?;
        Ok(row.map(|(kind, undone)| (kind, undone.is_some())))
    }

    /// Les déplacements d'un lot, du dernier au premier.
    ///
    /// L'ordre inverse n'est pas cosmétique : défaire une chaîne `a → b → c`
    /// dans le sens de l'aller écraserait `b` avant de l'avoir libéré.
    pub async fn moves_to_undo(
        pool: &SqlitePool,
        batch_id: &str,
    ) -> Result<Vec<MoveRecord>, sqlx::Error> {
        let rows: Vec<(i64, String, String, Option<i64>)> = sqlx::query_as(
            r#"
            SELECT id, before_path, after_path, modified_at
            FROM batch_journal_items
            WHERE batch_id = ? AND status = 'done'
            ORDER BY id DESC
            "#,
        )
        .bind(batch_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(item_id, before_path, after_path, modified_at)| MoveRecord {
                item_id,
                before_path,
                after_path,
                modified_at,
            })
            .collect())
    }

    /// Les instantanés de tags d'un lot.
    pub async fn tags_to_restore(
        pool: &SqlitePool,
        batch_id: &str,
    ) -> Result<Vec<TagSnapshot>, sqlx::Error> {
        let rows: Vec<(i64, String, String)> = sqlx::query_as(
            r#"
            SELECT id, before_path, tags_before
            FROM batch_journal_items
            WHERE batch_id = ? AND status = 'done' AND tags_before IS NOT NULL
            ORDER BY id
            "#,
        )
        .bind(batch_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(item_id, path, blob)| TagSnapshot {
                item_id,
                path,
                blob,
            })
            .collect())
    }

    pub async fn mark_item_undone(pool: &SqlitePool, item_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE batch_journal_items SET status = 'undone' WHERE id = ?")
            .bind(item_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn mark_undone(pool: &SqlitePool, batch_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE batch_journal SET undone_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(batch_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Les derniers lots, du plus récent au plus ancien.
    pub async fn list(pool: &SqlitePool, limit: i64) -> Result<Vec<JournalEntry>, sqlx::Error> {
        sqlx::query_as::<_, JournalEntry>(
            r#"
            SELECT id, kind, pattern, CAST(created_at AS TEXT) AS created_at,
                   total, succeeded, failed, CAST(undone_at AS TEXT) AS undone_at
            FROM batch_journal
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Écarte les lots au-delà des `keep` plus récents.
    ///
    /// Les éléments sont supprimés **explicitement**, sans compter sur la
    /// cascade.
    ///
    /// Une note antérieure affirmait ici que `PRAGMA foreign_keys` n'était pas
    /// activé et que les `ON DELETE CASCADE` ne s'exécutaient jamais. C'est
    /// faux : sqlx pose `foreign_keys = ON` sur chaque connexion par défaut
    /// (`SqliteConnectOptions`), et le projet ne le désactive nulle part. Les
    /// cascades du schéma sont donc bien actives.
    ///
    /// La suppression explicite reste : elle est redondante mais exacte, et ne
    /// dépend pas d'un réglage de connexion qu'un changement de configuration
    /// pourrait retirer sans bruit.
    pub async fn purge(pool: &SqlitePool, keep: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM batch_journal_items WHERE batch_id IN (
                SELECT id FROM batch_journal ORDER BY created_at DESC LIMIT -1 OFFSET ?
            )
            "#,
        )
        .bind(keep)
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            DELETE FROM batch_journal WHERE id IN (
                SELECT id FROM batch_journal ORDER BY created_at DESC LIMIT -1 OFFSET ?
            )
            "#,
        )
        .bind(keep)
        .execute(pool)
        .await?;
        Ok(())
    }
}
