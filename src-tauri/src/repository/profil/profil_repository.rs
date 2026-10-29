use sqlx::SqlitePool;

use crate::entity::profil::profil::Profil;


pub struct ProfilRepository;

impl ProfilRepository {

    pub async fn find_profile_by_id<'e, E>(
        exec: E,
        id: i64,
    ) -> Result<Profil, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let profil: Profil = sqlx::query_as::<_, Profil>(
            r#"
            SELECT id, name, avatar, color, bio, role, is_active, created_at, updated_at
            FROM profil
            WHERE id = ?
            LIMIT 1
            "#
        )
        .bind(id)
        .fetch_one(exec)
        .await?;

        Ok(profil)
    }

    pub async fn find_all_active<'e, E>(exec: E) -> Result<Vec<Profil>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let profils: Vec<Profil> = sqlx::query_as::<_, Profil>(
            r#"
            SELECT id, name, avatar, color, bio, role, is_active, created_at, updated_at
            FROM profil
            WHERE is_active = 1
            ORDER BY id ASC
            "#
        )
        .fetch_all(exec)
        .await?;

        Ok(profils)
    }

    pub async fn create<'e, E>(
        exec: E,
        pool: &SqlitePool,
        name: &str,
        avatar: Option<&str>,
        color: &str,
    ) -> Result<Profil, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO profil (name, avatar, color, role, is_active)
            VALUES (?, ?, ?, 'user', 1)
            RETURNING id
            "#
        )
        .bind(name)
        .bind(avatar)
        .bind(color)
        .fetch_one(exec)
        .await?;

        Self::find_profile_by_id(pool, id).await
    }

    pub async fn update<'e, E>(
        exec: E,
        pool: &SqlitePool,
        id: i64,
        name: &str,
        avatar: Option<&str>,
        color: &str,
    ) -> Result<Profil, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query(
            r#"
            UPDATE profil
            SET name = ?, avatar = ?, color = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#
        )
        .bind(name)
        .bind(avatar)
        .bind(color)
        .bind(id)
        .execute(exec)
        .await?;

        Self::find_profile_by_id(pool, id).await
    }

    pub async fn delete<'e, E>(exec: E, id: i64) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query("DELETE FROM profil WHERE id = ?")
            .bind(id)
            .execute(exec)
            .await?;

        Ok(())
    }
    /// Tous les profils, actifs ou non, pour un export : (id, nom, couleur, avatar, bio, rôle).
    #[allow(clippy::type_complexity)]
    pub async fn find_all_for_export<'e, E>(exec: E) -> Result<Vec<(i64, String, String, Option<String>, Option<String>, String)>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_as("SELECT id, name, color, avatar, bio, role FROM profil ORDER BY id").fetch_all(exec).await
    }

    pub async fn find_id_by_name<'e, E>(exec: E, name: &str) -> Result<Option<i64>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_scalar("SELECT id FROM profil WHERE name = ?").bind(name).fetch_optional(exec).await
    }

    /// Profil recréé depuis un export ; rend son id.
    pub async fn insert_from_export<'e, E>(
        exec: E,
        name: &str,
        color: &str,
        avatar: Option<&str>,
        bio: Option<&str>,
        role: &str,
    ) -> Result<i64, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_scalar("INSERT INTO profil (name, color, avatar, bio, role) VALUES (?, ?, ?, ?, ?) RETURNING id")
            .bind(name)
            .bind(color)
            .bind(avatar)
            .bind(bio)
            .bind(role)
            .fetch_one(exec)
            .await
    }

}
