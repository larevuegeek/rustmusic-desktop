use sqlx::Row;
use uuid::Uuid;

use crate::entity::library::library_dirs::{LibraryDir, LibraryDirCreate};

pub struct LibraryDirRepository;

impl LibraryDirRepository {
    /// Les chemins des dossiers actifs d'une bibliothèque.
    ///
    /// Ce sont les **racines** au-delà desquelles aucun nettoyage ne remonte.
    /// Elles se lisent en base et ne transitent jamais par l'interface : un
    /// garde-fou qu'on peut passer en paramètre n'en est pas un.
    pub async fn active_paths<'e, E>(exec: E, library_id: i64) -> Result<Vec<String>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_scalar::<_, String>(
            "SELECT path FROM library_dirs WHERE library_id = ? AND is_active = 1",
        )
        .bind(library_id)
        .fetch_all(exec)
        .await
    }


    // =========================================
    // INSERT
    // =========================================
    pub async fn insert_library_dir<'e, E>(
        exec: E,
        data: LibraryDirCreate,
    ) -> Result<LibraryDir, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let id = Uuid::new_v4().to_string();

        let row: LibraryDir = sqlx::query_as::<_, LibraryDir>(
            r#"
            INSERT INTO library_dirs (
                id,
                library_id,
                path,
                name,
                is_recursive,
                is_active,
                watch_enabled,
                include_patterns,
                exclude_patterns,
                created_at,
                updated_at
            )
            VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?, ?,
                CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
            )
            ON CONFLICT(library_id, path) DO UPDATE SET
                id = library_dirs.id
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(data.library_id)
        .bind(&data.path)
        .bind(&data.name)
        .bind(data.is_recursive)
        .bind(data.is_active)
        .bind(data.watch_enabled)
        .bind(data.include_patterns)
        .bind(data.exclude_patterns)
        .fetch_one(exec)
        .await?;

        Ok(row)
    }

    // =========================================
    // FIND BY PATH (clé métier)
    // =========================================
    pub async fn find_by_path<'e, E>(
        exec: E,
        library_id: i64,
        path: &str,
    ) -> Result<Option<LibraryDir>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, LibraryDir>(
            r#"
            SELECT *
            FROM library_dirs
            WHERE library_id = ?
              AND path = ?
            LIMIT 1
            "#
        )
        .bind(library_id)
        .bind(path)
        .fetch_optional(exec)
        .await
    }

    // =========================================
    // FIND ACTIVE DIRS (pour scan)
    // =========================================
    pub async fn find_active<'e, E>(
        exec: E,
        library_id: i64,
    ) -> Result<Vec<LibraryDir>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, LibraryDir>(
            r#"
            SELECT *
            FROM library_dirs
            WHERE library_id = ?
              AND is_active = 1
            ORDER BY path
            "#
        )
        .bind(library_id)
        .fetch_all(exec)
        .await
    }

    // =========================================
    // UPDATE SCAN STATS
    // =========================================
    pub async fn update_scan_result<'e, E>(
        exec: E,
        id: &str,
        total_files: i64,
        total_size: i64,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query(
            r#"
            UPDATE library_dirs
            SET
                total_files = ?,
                total_size = ?,
                scan_status = ?,
                scan_error = ?,
                last_scan_at = CURRENT_TIMESTAMP,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#
        )
        .bind(total_files)
        .bind(total_size)
        .bind(status)
        .bind(error)
        .bind(id)
        .execute(exec)
        .await?;

        Ok(())
    }

    // =========================================
    // FIND ALL BY LIBRARY
    // =========================================
    pub async fn find_all_by_library_id<'e, E>(
        exec: E,
        library_id: i64,
    ) -> Result<Vec<LibraryDir>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, LibraryDir>(
            r#"
            SELECT
                d.*,
                COALESCE((SELECT COUNT(*) FROM library_files f WHERE f.library_dir_id = d.id), 0) AS total_files,
                COALESCE((SELECT SUM(f.size) FROM library_files f WHERE f.library_dir_id = d.id), 0) AS total_size
            FROM library_dirs d
            WHERE d.library_id = ?
            ORDER BY d.created_at DESC
            "#
        )
        .bind(library_id)
        .fetch_all(exec)
        .await
    }

    // =========================================
    // DELETE
    // =========================================
    pub async fn delete_library_dir<'e, E>(
        exec: E,
        id: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query("DELETE FROM library_dirs WHERE id = ?")
            .bind(id)
            .execute(exec)
            .await?;
        Ok(())
    }

    // =========================================
    // MAPPING CENTRALISÉ
    // =========================================
    fn map_row(row: sqlx::sqlite::SqliteRow) -> LibraryDir {
        LibraryDir {
            id: row.get("id"),
            library_id: row.get("library_id"),

            path: row.get("path"),
            name: row.get("name"),

            is_recursive: row.get("is_recursive"),
            is_active: row.get("is_active"),
            watch_enabled: row.get("watch_enabled"),

            include_patterns: row.get("include_patterns"),
            exclude_patterns: row.get("exclude_patterns"),

            total_files: row.get("total_files"),
            total_size: row.get("total_size"),
            last_scan_at: row.get("last_scan_at"),
            scan_status: row.get("scan_status"),
            scan_error: row.get("scan_error"),

            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}
