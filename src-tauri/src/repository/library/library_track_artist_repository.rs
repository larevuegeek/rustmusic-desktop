use sqlx::Result;
use crate::mapper::library::artist::track_artist_view::TrackArtistView;
use crate::entity::library::library_track_artist::{
    LibraryTrackArtist,
    LibraryTrackArtistCreate,
};

pub struct LibraryTrackArtistRepository;

impl LibraryTrackArtistRepository {

    // ✅ Upsert (évite doublon UNIQUE(library_track_id, artist_id))
    pub async fn insert_library_track_artist<'e, E>(
        exec: E,
        data: LibraryTrackArtistCreate,
    ) -> Result<LibraryTrackArtist>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let record = sqlx::query_as::<_, LibraryTrackArtist>(
            r#"
            INSERT INTO library_track_artists (
                library_id,
                library_track_id,
                artist_id
            )
            VALUES (?, ?, ?)
            ON CONFLICT(library_track_id, artist_id)
            DO UPDATE SET
                library_id = excluded.library_id
            RETURNING *
            "#
        )
        .bind(data.library_id)
        .bind(data.library_track_id)
        .bind(data.artist_id)
        .fetch_one(exec)
        .await?;

        Ok(record)
    }

    // ✅ Trouver tous les artistes d'un track
    pub async fn find_by_track_id<'e, E>(
        exec: E,
        library_track_id: &str,
    ) -> Result<Vec<LibraryTrackArtist>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let records = sqlx::query_as::<_, LibraryTrackArtist>(
            r#"
            SELECT *
            FROM library_track_artists
            WHERE library_track_id = ?
            "#
        )
        .bind(library_track_id)
        .fetch_all(exec)
        .await?;

        Ok(records)
    }

    // ✅ Trouver tous les tracks d'un artiste
    pub async fn find_by_artist_id<'e, E>(
        exec: E,
        artist_id: &str,
    ) -> Result<Vec<LibraryTrackArtist>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let records = sqlx::query_as::<_, LibraryTrackArtist>(
            r#"
            SELECT *
            FROM library_track_artists
            WHERE artist_id = ?
            "#
        )
        .bind(artist_id)
        .fetch_all(exec)
        .await?;

        Ok(records)
    }

    /// Les artistes d'une piste, dans l'ordre du tag. `rowid` = ordre
    /// d'insertion ; `created_at` ne descend pas sous la seconde.
    pub async fn find_artists_of_track<'e, E>(
        exec: E,
        library_track_id: &str,
    ) -> Result<Vec<TrackArtistView>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, TrackArtistView>(
            r#"
            SELECT
                a.id     AS artist_id,
                lart.id  AS library_artist_id,
                a.name   AS name
            FROM library_track_artists lta
            INNER JOIN artists a ON a.id = lta.artist_id
            LEFT JOIN library_artists lart
                   ON lart.artist_id = a.id AND lart.library_id = lta.library_id
            WHERE lta.library_track_id = ?
            ORDER BY lta.rowid
            "#
        )
        .bind(library_track_id)
        .fetch_all(exec)
        .await
    }

    // ✅ Delete par track
    pub async fn delete_by_track_id<'e, E>(
        exec: E,
        library_track_id: &str,
    ) -> Result<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query(
            r#"
            DELETE FROM library_track_artists
            WHERE library_track_id = ?
            "#
        )
        .bind(library_track_id)
        .execute(exec)
        .await?;

        Ok(())
    }

    // ✅ Delete relation spécifique
    pub async fn delete_relation<'e, E>(
        exec: E,
        library_track_id: &str,
        artist_id: &str,
    ) -> Result<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query(
            r#"
            DELETE FROM library_track_artists
            WHERE library_track_id = ?
            AND artist_id = ?
            "#
        )
        .bind(library_track_id)
        .bind(artist_id)
        .execute(exec)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::SqlitePool;

    async fn base() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("base mémoire");
        sqlx::migrate!("./SQL/").run(&pool).await.expect("migrations");

        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')")
            .execute(&pool).await.expect("profil");
        sqlx::query("INSERT OR IGNORE INTO library (id, profil_id, name) VALUES (1, 1, 'test')")
            .execute(&pool).await.expect("bibliothèque");
        sqlx::query("INSERT INTO artists (id, name, name_normalized, sort_name) VALUES ('art-1', 'A', 'a', 'A')")
            .execute(&pool).await.expect("artiste");
        sqlx::query("INSERT INTO library_files (id, library_id, path, filename, extension, size, status) VALUES ('f-1', 1, '/x.flac', 'x', 'flac', 1, 'ok')")
            .execute(&pool).await.expect("fichier");
        sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, artist_id, title, title_normalized) VALUES ('t-1', 1, 'f-1', 'art-1', 'T', 't')")
            .execute(&pool).await.expect("piste");
        pool
    }

    fn lien() -> LibraryTrackArtistCreate {
        LibraryTrackArtistCreate {
            library_id: 1,
            library_track_id: "t-1".to_string(),
            artist_id: "art-1".to_string(),
        }
    }

    /// L'indexation ignore ce qu'elle renvoie : un échec ne se voit nulle part.
    #[tokio::test]
    async fn la_liaison_piste_artiste_s_insere() {
        let pool = base().await;

        let lien = LibraryTrackArtistRepository::insert_library_track_artist(&pool, lien())
            .await
            .expect("l'insertion doit réussir");

        assert_eq!(lien.library_track_id, "t-1");
        assert_eq!(lien.artist_id, "art-1");
    }

    /// La séquence de `save_analysed_to_db`, en transaction comme à l'import.
    #[tokio::test]
    async fn la_sequence_d_indexation_lie_tous_les_artistes() {
        use crate::entity::artist::artist::ArtistCreate;
        use crate::entity::library::library_artist::LibraryArtistCreate;
        use crate::entity::library::library_track::LibraryTrackCreate;
        use crate::repository::artist::artist_repository::ArtistRepository;
        use crate::repository::library::library_artist_repository::LibraryArtistRepository;
        use crate::repository::library::library_track_repository::LibraryTrackRepository;
        use crate::helper::string::string::split_artists;

        let pool = base().await;
        sqlx::query("INSERT INTO library_files (id, library_id, path, filename, extension, size, status) VALUES ('f-2', 1, '/y.flac', 'y', 'flac', 1, 'ok')")
            .execute(&pool).await.expect("fichier");

        let mut tx = pool.begin().await.expect("transaction");

        let noms = split_artists("Alan Walker;K-391;Sofia Carson");
        assert_eq!(noms.len(), 3, "le découpage doit rendre trois artistes");

        let mut ids: Vec<String> = Vec::new();
        for nom in &noms {
            let artiste = ArtistRepository::insert_artist(&mut *tx, ArtistCreate {
                name: nom.clone(),
                name_normalized: nom.to_lowercase(),
                sort_name: nom.clone(),
            }).await.expect("artiste");
            LibraryArtistRepository::insert_library_artist(&mut *tx, LibraryArtistCreate {
                library_id: 1, artist_id: artiste.id.clone(),
            }).await.expect("artiste de bibliothèque");
            ids.push(artiste.id);
        }

        let piste = LibraryTrackRepository::insert_library_track(&mut *tx, LibraryTrackCreate {
            library_id: 1, file_id: "f-2".to_string(), cache_id: None,
            artist_id: Some(ids[0].clone()), library_album_id: None,
            title: "Different World".to_string(),
            title_normalized: "different world".to_string(),
            track_number: Some(1), disc_number: 1, tags: None,
            duration: Some(200.0), bitrate: None, sample_rate: None, rating: None,
        }).await.expect("piste");

        for aid in &ids {
            LibraryTrackArtistRepository::insert_library_track_artist(&mut *tx, LibraryTrackArtistCreate {
                library_id: 1, artist_id: aid.clone(), library_track_id: piste.id.clone(),
            }).await.expect("liaison piste ↔ artiste");
        }

        tx.commit().await.expect("commit");

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM library_track_artists WHERE library_track_id = ?",
        )
        .bind(&piste.id)
        .fetch_one(&pool)
        .await
        .expect("comptage");
        assert_eq!(total, 3, "les trois artistes du tag doivent être liés");
    }

    /// Réindexer repasse sur les mêmes liaisons : ni échec, ni doublon.
    #[tokio::test]
    async fn reinserer_la_meme_liaison_ne_double_pas() {
        let pool = base().await;

        LibraryTrackArtistRepository::insert_library_track_artist(&pool, lien())
            .await
            .expect("première insertion");
        LibraryTrackArtistRepository::insert_library_track_artist(&pool, lien())
            .await
            .expect("seconde insertion");

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM library_track_artists")
            .fetch_one(&pool)
            .await
            .expect("comptage");
        assert_eq!(total, 1);
    }
}
