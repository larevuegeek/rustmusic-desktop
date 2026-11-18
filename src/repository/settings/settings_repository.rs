pub struct SettingsRepository;

impl SettingsRepository {
    pub async fn get<'e, E>(exec: E, key: &str) -> Result<Option<String>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_scalar::<_, String>(
            "SELECT value FROM settings WHERE key = ? LIMIT 1"
        )
        .bind(key)
        .fetch_optional(exec)
        .await
    }

    pub async fn set<'e, E>(exec: E, key: &str, value: &str) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query(
            r#"
            INSERT INTO settings (key, value) VALUES (?, ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#
        )
        .bind(key)
        .bind(value)
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn get_all<'e, E>(exec: E) -> Result<Vec<(String, String)>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT key, value FROM settings ORDER BY key"
        )
        .fetch_all(exec)
        .await?;

        Ok(rows)
    }

}
