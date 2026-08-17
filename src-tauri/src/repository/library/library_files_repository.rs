use sqlx::Row;
use uuid::Uuid;

use crate::entity::library::library_files::LibraryFile;
use crate::entity::library::library_files::LibraryFileCreate;

pub struct LibraryFilesRepository;

impl LibraryFilesRepository {
    /// Identifiant d'un fichier, par son chemin.
    ///
    /// Volontairement limité à cette colonne : lire la ligne entière casse sur
    /// `modified_at`, stocké en INTEGER mais déclaré `Option<String>` dans
    /// l'entité. Tant que ce conflit de décodage n'est pas repris, la requête
    /// ciblée est la seule à passer.
    pub async fn find_id_by_path<'e, E>(exec: E, path: &str) -> Result<Option<String>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_scalar::<_, String>("SELECT id FROM library_files WHERE path = ? LIMIT 1")
            .bind(path)
            .fetch_optional(exec)
            .await
    }


    pub async fn insert_library_file<'e, E>(
        exec: E,
        data: LibraryFileCreate,
    ) -> Result<LibraryFile, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let id: String = Uuid::new_v4().to_string();

        let result: LibraryFile = sqlx::query_as::<_, LibraryFile>(
            r#"
            INSERT INTO library_files (
                id, library_id, library_dir_id, cache_id,
                path, filename, extension, size,
                file_hash, modified_at, status, is_available,
                error_message, created_at, updated_at, last_verified_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(library_id, path) DO UPDATE SET
                id = library_files.id
            RETURNING
                id, library_id, library_dir_id, cache_id,
                path, filename, extension, size,
                file_hash, CAST(modified_at AS TEXT) AS modified_at,
                status, is_available, error_message,
                created_at, updated_at, last_verified_at
            "#
        )
        .bind(&id)
        .bind(data.library_id)
        .bind(data.library_dir_id)
        .bind(data.cache_id)
        .bind(&data.path)
        .bind(&data.filename)
        .bind(&data.extension)
        .bind(data.size)
        .bind(data.file_hash)
        .bind(&data.modified_at)
        .bind(&data.status)
        .bind(data.is_available)
        .bind(data.error_message)
        .bind(&data.created_at)
        .bind(&data.updated_at)
        .bind(data.last_verified_at)
        .fetch_one(exec)
        .await?;

        Ok(result)
    }

    pub async fn find_by_path<'e, E>(
        exec: E,
        library_id: i64,
        path: &str,
    ) -> Result<Option<LibraryFile>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, LibraryFile>(
            r#"
            SELECT
                id, library_id, library_dir_id, cache_id,
                path, filename, extension, size,
                file_hash, CAST(modified_at AS TEXT) AS modified_at,
                status, is_available, error_message,
                created_at, updated_at, last_verified_at
            FROM library_files
            WHERE library_id = ? AND path = ?
            LIMIT 1
            "#
        )
        .bind(library_id)
        .bind(path)
        .fetch_optional(exec)
        .await
    }

    // Chercher un fichier par chemin dans toutes les bibliothèques
    pub async fn find_by_path_any<'e, E>(
        exec: E,
        path: &str,
    ) -> Result<Option<LibraryFile>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, LibraryFile>(
            r#"
            SELECT
                id, library_id, library_dir_id, cache_id,
                path, filename, extension, size,
                file_hash, CAST(modified_at AS TEXT) AS modified_at,
                status, is_available, error_message,
                created_at, updated_at, last_verified_at
            FROM library_files
            WHERE path = ?
            LIMIT 1
            "#
        )
        .bind(path)
        .fetch_optional(exec)
        .await
    }

    pub async fn find_by_id<'e, E>(
        exec: E,
        id: &str,
    ) -> Result<Option<LibraryFile>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, LibraryFile>(
            r#"
            SELECT
                id, library_id, library_dir_id, cache_id,
                path, filename, extension, size,
                file_hash, CAST(modified_at AS TEXT) AS modified_at,
                status, is_available, error_message,
                created_at, updated_at, last_verified_at
            FROM library_files
            WHERE id = ?
            LIMIT 1
            "#
        )
        .bind(id)
        .fetch_optional(exec)
        .await
    }

    // Marquer un fichier comme indexé avec sa date de modification
    pub async fn mark_as_indexed<'e, E>(
        exec: E,
        id: &str,
        modified_at: Option<&str>,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query(
            "UPDATE library_files SET status = 'indexed', modified_at = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        )
        .bind(modified_at)
        .bind(id)
        .execute(exec)
        .await?;
        Ok(())
    }

    fn map_row(row: sqlx::sqlite::SqliteRow) -> LibraryFile {
        LibraryFile {
            id: row.get("id"),
            library_id: row.get("library_id"),
            library_dir_id: row.get("library_dir_id"),
            cache_id: row.get("cache_id"),
            path: row.get("path"),
            filename: row.get("filename"),
            extension: row.get("extension"),
            size: row.get("size"),
            file_hash: row.get("file_hash"),
            modified_at: row.get("modified_at"),
            status: row.get("status"),
            is_available: row.get("is_available"),
            error_message: row.get("error_message"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            last_verified_at: row.get("last_verified_at"),
        }
    }
}
