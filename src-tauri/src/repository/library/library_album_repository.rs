// repository/album/album_repository.rs

use uuid::Uuid;

use crate::entity::library::library_album::LibraryAlbum;
use crate::entity::library::library_album::LibraryAlbumCreate;
use crate::mapper::library::album::album_detail_view::AlbumDetailView;
use crate::mapper::library::album::album_list_view::AlbumListView;

pub struct LibraryAlbumRepository;

/// L'unicité d'avant la 0.2.5, gardée pour qu'une version antérieure
/// réinstallée retrouve la cible de son `ON CONFLICT` et sache encore indexer.
/// Posée au démarrage, pas par une migration : si une bibliothèque contient
/// déjà des doublons, mieux vaut un avertissement qu'un refus de démarrer.
pub const INDEX_COMPAT_0_2_4: &str =
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_la_compat_0_2_4
     ON library_albums(library_id, artist_id, title_normalized)";

impl LibraryAlbumRepository {

    /// Pose l'album, ou rend celui qui existe déjà.
    ///
    /// Deux unicités cohabitent : la nôtre, `(bibliothèque, titre, dossier)`,
    /// et celle d'avant la 0.2.5, `(bibliothèque, artiste, titre)`, gardée
    /// pour qu'une version antérieure réinstallée sache encore indexer. La
    /// seconde ne peut être heurtée que par un même artiste ayant deux fois le
    /// même titre dans deux dossiers — un double rip. On réutilise alors sa
    /// fiche, exactement comme le faisait la 0.2.4.
    pub async fn insert_library_album(
        conn: &mut sqlx::SqliteConnection,
        album_data: LibraryAlbumCreate,
    ) -> Result<LibraryAlbum, sqlx::Error>
    {

        let id: String = Uuid::new_v4().to_string();

        let pose = sqlx::query_as::<_, LibraryAlbum>(
            r#"
            INSERT INTO library_albums (
                id,
                library_id,
                artist_id,
                title,
                title_normalized,
                album_dir,
                year,
                genre,
                cover_url,
                album_type,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)

            ON CONFLICT(library_id, title_normalized, album_dir)
            DO UPDATE SET
                id = library_albums.id

            RETURNING *
            "#
        )
        .bind(&id)
        .bind(album_data.library_id)
        .bind(&album_data.artist_id)
        .bind(&album_data.title)
        .bind(&album_data.title_normalized)
        .bind(&album_data.album_dir)
        .bind(album_data.year)
        .bind(&album_data.genre)
        .bind(&album_data.cover_url)
        .bind(album_data.album_type.as_deref().unwrap_or("album"))
        .fetch_one(&mut *conn)
        .await;

        match pose {
            Ok(album) => Ok(album),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                sqlx::query_as::<_, LibraryAlbum>(
                    "SELECT * FROM library_albums
                     WHERE library_id = ? AND artist_id = ? AND title_normalized = ?",
                )
                .bind(album_data.library_id)
                .bind(&album_data.artist_id)
                .bind(&album_data.title_normalized)
                .fetch_one(&mut *conn)
                .await
            }
            Err(e) => Err(e),
        }
    }


    pub async fn update_library_album_cover<'e, E>(
            exec: E,
            new_path_str: &str,
            old_path_str: &str,
        ) -> Result<(), sqlx::Error>
        where
            E: sqlx::Executor<'e, Database = sqlx::Sqlite>
        {

        sqlx::query("UPDATE library_albums SET cover_url = ? WHERE cover_url = ?")
            .bind(new_path_str)
            .bind(old_path_str)
            .execute(exec)
            .await?;

            Ok(())
    }

    pub async fn update_cover_url_by_id<'e, E>(
        exec: E,
        album_id: &str,
        cover_url: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query("UPDATE library_albums SET cover_url = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(cover_url)
            .bind(album_id)
            .execute(exec)
            .await?;
        Ok(())
    }

    pub async fn find_albums_without_cover<'e, E>(
        exec: E,
        library_id: i64,
    ) -> Result<Vec<LibraryAlbum>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, LibraryAlbum>(
            "SELECT * FROM library_albums WHERE library_id = ? AND (cover_url IS NULL OR cover_url = '') ORDER BY title"
        )
        .bind(library_id)
        .fetch_all(exec)
        .await
    }

    pub async fn find_by_normalized_title<'e, E>(
        exec: E,
        title_normalized: &str,
        library_id: i64,
        artist_id: Option<&str>,
        year: Option<i32>,
    ) -> Result<Option<LibraryAlbum>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, LibraryAlbum>(
            r#"
            SELECT * FROM library_albums
            WHERE title_normalized = ? AND library_id = ?
            AND artist_id IS ?
            AND year IS ?
            LIMIT 1
            "#
        )
        .bind(title_normalized)
        .bind(library_id)
        .bind(artist_id)
        .bind(year)
        .fetch_optional(exec)
        .await
    }

    pub async fn find_all_by_normalized_title<'e, E>(
        exec: E,
        title_normalized: &str,
        library_id: i64,
    ) -> Result<Vec<LibraryAlbum>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, LibraryAlbum>(
            "SELECT * FROM library_albums WHERE title_normalized = ? AND library_id = ?"
        )
        .bind(title_normalized)
        .bind(library_id)
        .fetch_all(exec)
        .await
    }

    pub async fn find_by_artist<'e, E>(
        exec: E,
        artist_id: &str,
        library_id: i64,
    ) -> Result<Vec<LibraryAlbum>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, LibraryAlbum>(
            "SELECT * FROM library_albums WHERE artist_id = ? AND library_id = ? ORDER BY year DESC, title"
        )
        .bind(artist_id)
        .bind(library_id)
        .fetch_all(exec)
        .await
    }

    pub async fn find_all_albums_by_library_id<'e, E>(
        exec: E,
        library_id: i64,
        missing_cover: Option<bool>,
    ) -> Result<Vec<AlbumListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let where_query = if missing_cover.unwrap_or(false) {
            " AND (la.cover_url IS NULL OR la.cover_url = '')"
        } else {
            ""
        };

        let query_sql = format!(r#"
            SELECT
                -- =========================
                -- Identité Album
                -- =========================
                la.id,
                la.library_id,
                la.title,
                la.title_normalized,
                la.album_type,
                la.musicbrainz_id,
                la.artist_id,
                a.name                         AS artist,
                la.year,
                la.genre,
                la.cover_url,
                COUNT(lt.id)                   AS total_tracks,
                COALESCE(SUM(lt.duration), 0.0)  AS total_duration,
                la.notes,

                -- =========================
                -- Timestamps
                -- =========================
                la.created_at,
                la.updated_at

            FROM library_albums la
            LEFT JOIN artists a ON a.id = la.artist_id
            LEFT JOIN library_tracks lt ON lt.library_album_id = la.id
            WHERE la.library_id = ? {}
            GROUP BY la.id
            ORDER BY la.title_normalized ASC
            "#, where_query);


        let albums: Vec<AlbumListView> = sqlx::query_as::<_, AlbumListView>(&query_sql)
        .bind(library_id)
        .fetch_all(exec)
        .await?;

        Ok(albums)
    }

    /// Les albums d'un artiste. `participations` y ajoute ceux où il n'a que
    /// des pistes — compilations, bandes originales, featurings — sans quoi un
    /// invité affiche zéro album alors que ses morceaux sont bien rangés.
    pub async fn find_albums_by_artist_id<'e, E>(
        exec: E,
        library_id: i64,
        artist_id: &str,
        participations: bool,
    ) -> Result<Vec<AlbumListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        // Les deux branches d'union ne filtrent pas sur `library_id` : le
        // planificateur préférait alors son index, mille fois moins sélectif
        // que celui sur l'artiste (8,3 ms contre 0,08 ms). La bibliothèque est
        // retenue en sortie, ce qui donne le même résultat.
        sqlx::query_as::<_, AlbumListView>(
            r#"
            WITH ses_albums(id) AS (
                SELECT id FROM library_albums
                    WHERE library_id = ?1 AND artist_id = ?2
                UNION
                SELECT t.library_album_id FROM library_tracks t
                    WHERE t.artist_id = ?2 AND t.library_album_id IS NOT NULL AND ?3
                UNION
                SELECT t.library_album_id FROM library_track_artists ta
                    JOIN library_tracks t ON t.id = ta.library_track_id
                    WHERE ta.artist_id = ?2 AND t.library_album_id IS NOT NULL AND ?3
            )
            SELECT
                la.id,
                la.library_id,
                la.title,
                la.title_normalized,
                la.album_type,
                la.musicbrainz_id,
                la.artist_id,
                a.name                         AS artist,
                la.year,
                la.genre,
                la.cover_url,
                COUNT(lt.id)                   AS total_tracks,
                COALESCE(SUM(lt.duration), 0.0)  AS total_duration,
                la.notes,
                (la.artist_id IS NOT ?2)       AS participation,
                la.created_at,
                la.updated_at
            FROM library_albums la
            JOIN ses_albums s ON s.id = la.id
            LEFT JOIN artists a ON a.id = la.artist_id
            LEFT JOIN library_tracks lt ON lt.library_album_id = la.id
            WHERE la.library_id = ?1
            GROUP BY la.id
            ORDER BY participation ASC, la.year DESC, la.title_normalized ASC
            "#
        )
        .bind(library_id)
        .bind(artist_id)
        .bind(participations)
        .fetch_all(exec)
        .await
    }

    pub async fn find_album_by_id<'e, E>(
        exec: E,
        library_album_id: String,
    ) -> Result<AlbumDetailView, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let album: AlbumDetailView = sqlx::query_as::<_, AlbumDetailView>(
            r#"
            SELECT
                la.id,
                la.library_id,
                la.title,
                la.title_normalized,
                la.album_type,
                la.musicbrainz_id,

                la.artist_id,
                a.name AS artist,

                la.year,
                la.genre,
                la.notes,

                la.cover_url,

                -- fallback éventuel depuis cache
                MAX(lc.thumbnail_path) AS thumbnail_path,
                -- =========================
                COUNT(lt.id) AS total_tracks,
                COALESCE(SUM(lt.duration), 0) AS total_duration,
                la.created_at,
                la.updated_at

            FROM library_albums la
            LEFT JOIN artists a ON a.id = la.artist_id
            LEFT JOIN library_tracks lt ON lt.library_album_id = la.id
            LEFT JOIN library_cache lc ON lc.id = lt.cache_id

            WHERE la.id = ?

            GROUP BY la.id
            "#
        )
        .bind(library_album_id)
        .fetch_one(exec)
        .await?;

        Ok(album)
    }

    pub async fn search<'e, E>(
        exec: E,
        search_term: &str,
        limit: i64,
    ) -> Result<Vec<crate::entity::search::search_result::SearchResult>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, crate::entity::search::search_result::SearchResult>(
            r#"
            SELECT
                la.id AS id,
                'album' AS result_type,
                la.title AS title,
                a.name AS subtitle,
                la.cover_url AS thumbnail_path,
                NULL AS path,
                la.library_id AS library_id
            FROM library_albums la
            LEFT JOIN artists a ON la.artist_id = a.id
            WHERE LOWER(la.title) LIKE ?1
            LIMIT ?2
            "#
        )
        .bind(search_term)
        .bind(limit)
        .fetch_all(exec)
        .await
    }








}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

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
        pool
    }

    async fn poser_artiste(pool: &SqlitePool, id: &str, nom: &str) {
        sqlx::query("INSERT INTO artists (id, name, name_normalized, sort_name) VALUES (?, ?, ?, ?)")
            .bind(id).bind(nom).bind(nom.to_lowercase()).bind(nom)
            .execute(pool).await.expect("artiste");
    }

    async fn poser_album(pool: &SqlitePool, id: &str, titre: &str, proprietaire: &str) {
        sqlx::query("INSERT INTO library_albums (id, library_id, artist_id, title, title_normalized, year) VALUES (?, 1, ?, ?, ?, 2000)")
            .bind(id).bind(proprietaire).bind(titre).bind(titre.to_lowercase())
            .execute(pool).await.expect("album");
    }

    /// Une piste rangée dans un album, avec son artiste principal.
    async fn poser_piste(pool: &SqlitePool, id: &str, album: &str, artiste: &str) {
        sqlx::query("INSERT INTO library_files (id, library_id, path, filename, extension, size, status) VALUES (?, 1, ?, 'f', 'flac', 1, 'indexed')")
            .bind(format!("f-{id}")).bind(format!("/{id}.flac"))
            .execute(pool).await.expect("fichier");
        sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, library_album_id, artist_id, title, title_normalized) VALUES (?, 1, ?, ?, ?, 'T', 't')")
            .bind(id).bind(format!("f-{id}")).bind(album).bind(artiste)
            .execute(pool).await.expect("piste");
    }

    async fn lister(pool: &SqlitePool, artiste: &str, participations: bool) -> Vec<(String, bool)> {
        LibraryAlbumRepository::find_albums_by_artist_id(pool, 1, artiste, participations)
            .await.expect("requête")
            .into_iter().map(|a| (a.title, a.participation)).collect()
    }

    #[tokio::test]
    async fn un_invite_sur_une_compilation_voit_l_album() {
        let pool = base().await;
        poser_artiste(&pool, "va", "Various Artists").await;
        poser_artiste(&pool, "pj", "PJ Harvey").await;
        poser_album(&pool, "alb", "Peaky Blinders OST", "va").await;
        poser_piste(&pool, "t1", "alb", "pj").await;

        assert_eq!(lister(&pool, "pj", false).await, vec![]);
        assert_eq!(lister(&pool, "pj", true).await,
                   vec![("Peaky Blinders OST".into(), true)]);
    }

    #[tokio::test]
    async fn ses_propres_albums_passent_avant_ses_participations() {
        let pool = base().await;
        poser_artiste(&pool, "va", "Various Artists").await;
        poser_artiste(&pool, "dp", "Daft Punk").await;
        poser_album(&pool, "a1", "Discovery", "dp").await;
        poser_album(&pool, "a2", "NRJ Hits", "va").await;
        poser_piste(&pool, "t1", "a1", "dp").await;
        poser_piste(&pool, "t2", "a2", "dp").await;

        assert_eq!(lister(&pool, "dp", true).await,
                   vec![("Discovery".into(), false), ("NRJ Hits".into(), true)]);
    }

    #[tokio::test]
    async fn une_liaison_multi_artistes_suffit() {
        let pool = base().await;
        poser_artiste(&pool, "jb", "Jacques Brel").await;
        poser_artiste(&pool, "fr", "François Rauber").await;
        poser_album(&pool, "alb", "Ces gens-là", "jb").await;
        poser_piste(&pool, "t1", "alb", "jb").await;
        sqlx::query("INSERT INTO library_track_artists (id, library_id, library_track_id, artist_id) VALUES ('l1', 1, 't1', 'fr')")
            .execute(&pool).await.expect("liaison");

        // Il n'a aucune piste à son nom : seule la liaison le rattache.
        assert_eq!(lister(&pool, "fr", true).await,
                   vec![("Ces gens-là".into(), true)]);
    }

    #[tokio::test]
    async fn un_album_n_apparait_qu_une_fois() {
        let pool = base().await;
        poser_artiste(&pool, "dp", "Daft Punk").await;
        poser_album(&pool, "alb", "Homework", "dp").await;
        poser_piste(&pool, "t1", "alb", "dp").await;
        poser_piste(&pool, "t2", "alb", "dp").await;
        sqlx::query("INSERT INTO library_track_artists (id, library_id, library_track_id, artist_id) VALUES ('l1', 1, 't1', 'dp')")
            .execute(&pool).await.expect("liaison");

        let albums = lister(&pool, "dp", true).await;
        assert_eq!(albums, vec![("Homework".into(), false)]);
    }

    #[tokio::test]
    async fn la_bibliotheque_voisine_est_ignoree() {
        let pool = base().await;
        sqlx::query("INSERT INTO library (id, profil_id, name) VALUES (2, 1, 'autre')")
            .execute(&pool).await.expect("2e bibliothèque");
        poser_artiste(&pool, "dp", "Daft Punk").await;
        poser_album(&pool, "alb", "Homework", "dp").await;
        sqlx::query("INSERT INTO library_albums (id, library_id, artist_id, title, title_normalized) VALUES ('ailleurs', 2, 'dp', 'Ailleurs', 'ailleurs')")
            .execute(&pool).await.expect("album voisin");
        sqlx::query("INSERT INTO library_files (id, library_id, path, filename, extension, size, status) VALUES ('f2', 2, '/x.flac', 'f', 'flac', 1, 'indexed')")
            .execute(&pool).await.expect("fichier voisin");
        sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, library_album_id, artist_id, title, title_normalized) VALUES ('t2', 2, 'f2', 'ailleurs', 'dp', 'T', 't')")
            .execute(&pool).await.expect("piste voisine");
        poser_piste(&pool, "t1", "alb", "dp").await;

        assert_eq!(lister(&pool, "dp", true).await, vec![("Homework".into(), false)]);
    }

    /// Pose l'unicité d'avant la 0.2.5, exactement comme le démarrage.
    async fn poser_index_compat(pool: &SqlitePool) {
        sqlx::query(INDEX_COMPAT_0_2_4).execute(pool).await.expect("index de compatibilité");
    }

    #[tokio::test]
    async fn une_version_anterieure_sait_encore_indexer() {
        let pool = base().await;
        poser_artiste(&pool, "dp", "Daft Punk").await;
        poser_album(&pool, "alb", "Homework", "dp").await;
        poser_index_compat(&pool).await;

        // La requête textuelle de la 0.2.4, mot pour mot.
        let r = sqlx::query(
            "INSERT INTO library_albums (id, library_id, artist_id, title, title_normalized)
             VALUES ('neuf', 1, 'dp', 'Homework', 'homework')
             ON CONFLICT(library_id, artist_id, title_normalized)
             DO UPDATE SET id = library_albums.id",
        )
        .execute(&pool)
        .await;

        assert!(r.is_ok(), "la 0.2.4 ne saurait plus indexer : {:?}", r.err());
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM library_albums")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(n, 1, "elle aurait dû retrouver la fiche, pas en créer une");
    }

    #[tokio::test]
    async fn un_double_rip_reutilise_la_fiche_au_lieu_d_echouer() {
        let pool = base().await;
        poser_artiste(&pool, "ff", "Foo Fighters").await;
        poser_index_compat(&pool).await;

        let creer = |dossier: &'static str| LibraryAlbumCreate {
            library_id: 1,
            artist_id: "ff".into(),
            album_dir: dossier.into(),
            title: "Echoes".into(),
            title_normalized: "echoes".into(),
            year: None, genre: None, cover_url: None, album_type: None,
        };

        let mut conn = pool.acquire().await.unwrap();
        let un = LibraryAlbumRepository::insert_library_album(&mut conn, creer(r"S:\M\Echoes"))
            .await.expect("premier dossier");
        // Même artiste, même titre, autre dossier : l'unicité d'avant s'y oppose.
        let deux = LibraryAlbumRepository::insert_library_album(&mut conn, creer(r"S:\M\Echoes - Copie"))
            .await.expect("le double rip ne doit pas faire échouer l'indexation");

        assert_eq!(un.id, deux.id, "les deux dossiers doivent partager la fiche");

        drop(conn); // la base d'essai n'ouvre qu'une connexion
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM library_albums")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn deux_artistes_sous_un_meme_toit_restent_un_seul_album() {
        let pool = base().await;
        poser_artiste(&pool, "a1", "Ava Max").await;
        poser_artiste(&pool, "a2", "Soprano").await;
        poser_index_compat(&pool).await;

        let creer = |artiste: &'static str| LibraryAlbumCreate {
            library_id: 1,
            artist_id: artiste.into(),
            album_dir: r"S:\M\NRJ Hits".into(),
            title: "NRJ Hits".into(),
            title_normalized: "nrj hits".into(),
            year: None, genre: None, cover_url: None, album_type: None,
        };

        let mut conn = pool.acquire().await.unwrap();
        let un = LibraryAlbumRepository::insert_library_album(&mut conn, creer("a1"))
            .await.expect("premier artiste");
        let deux = LibraryAlbumRepository::insert_library_album(&mut conn, creer("a2"))
            .await.expect("second artiste");

        assert_eq!(un.id, deux.id, "un dossier, un album — c'est tout l'objet de la 0.2.5");
    }
}
