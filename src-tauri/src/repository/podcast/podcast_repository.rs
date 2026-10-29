use std::collections::HashSet;

use sqlx::SqlitePool;

use crate::core::podcast::download;
use crate::core::podcast::feed::{Feed, FeedEpisode};
use crate::entity::podcast::podcast::{DownloadView, EpisodeView, FeedState, LatestEpisode, Playable, PodcastView};

pub struct PodcastRepository;

// Colonnes constantes assemblées dans les requêtes : aucune saisie n'y entre (d'où `AssertSqlSafe`).
const PODCAST_COLUMNS: &str = r#"
    p.id, p.feed_url, p.title, p.author, p.description, p.image_url, p.cover_path, p.link, p.checked_at,
    p.auto_download, p.pinned,
    COUNT(e.id) AS episode_count,
    COALESCE(SUM(e.played = 0), 0) AS unplayed_count,
    COALESCE(SUM(e.is_new = 1 AND e.played = 0), 0) AS new_count,
    MAX(e.published_at) AS latest_at,
    p.created_at
"#;

const EPISODE_COLUMNS: &str = r#"
    id, podcast_id, title, summary, NULL AS description, published_at, duration, audio_url, audio_type,
    audio_size, image_url, season, episode, file_path, position, played, is_new, COALESCE(local_path, '') AS path
"#;

/// (id, titre, date, adresse, type, fichier) : de quoi nommer un épisode dans le cache.
type PathRow = (i64, String, Option<String>, String, Option<String>);

fn date(e: &FeedEpisode) -> Option<String> {
    e.published_at.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
}

impl PodcastRepository {

    // =========================================
    // PODCASTS
    // =========================================
    pub async fn find_by_profile(pool: &SqlitePool, profil_id: i64) -> Result<Vec<PodcastView>, sqlx::Error> {
        sqlx::query_as::<_, PodcastView>(sqlx::AssertSqlSafe(format!(
            "SELECT {PODCAST_COLUMNS} FROM podcasts p LEFT JOIN podcast_episodes e ON e.podcast_id = p.id
             WHERE p.profil_id = ? GROUP BY p.id ORDER BY latest_at DESC, p.title COLLATE NOCASE"
        )))
        .bind(profil_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_one(pool: &SqlitePool, id: i64) -> Result<Option<PodcastView>, sqlx::Error> {
        sqlx::query_as::<_, PodcastView>(sqlx::AssertSqlSafe(format!(
            "SELECT {PODCAST_COLUMNS} FROM podcasts p LEFT JOIN podcast_episodes e ON e.podcast_id = p.id
             WHERE p.id = ? GROUP BY p.id"
        )))
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_id_by_url(pool: &SqlitePool, profil_id: i64, feed_url: &str) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar("SELECT id FROM podcasts WHERE profil_id = ? AND feed_url = ?")
            .bind(profil_id)
            .bind(feed_url)
            .fetch_optional(pool)
            .await
    }

    pub async fn feed_state(pool: &SqlitePool, id: i64) -> Result<Option<FeedState>, sqlx::Error> {
        sqlx::query_as("SELECT id, feed_url, etag, last_modified, image_url, cover_path FROM podcasts WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Tous les flux, tous profils confondus : le rafraîchissement de fond.
    pub async fn all_feed_states(pool: &SqlitePool) -> Result<Vec<FeedState>, sqlx::Error> {
        sqlx::query_as("SELECT id, feed_url, etag, last_modified, image_url, cover_path FROM podcasts ORDER BY checked_at")
            .fetch_all(pool)
            .await
    }

    pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM podcasts WHERE id = ?").bind(id).execute(pool).await.map(|_| ())
    }

    pub async fn set_auto_download(pool: &SqlitePool, id: i64, on: bool) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE podcasts SET auto_download = ? WHERE id = ?").bind(on).bind(id).execute(pool).await.map(|_| ())
    }

    /// Les `limit` nouveaux épisodes d'un podcast en téléchargement automatique, pas encore téléchargés.
    pub async fn auto_download_candidates(pool: &SqlitePool, podcast_id: i64, limit: i64) -> Result<Vec<i64>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT e.id FROM podcast_episodes e JOIN podcasts p ON p.id = e.podcast_id
              WHERE e.podcast_id = ? AND p.auto_download = 1 AND e.is_new = 1 AND e.played = 0 AND e.file_path IS NULL
              ORDER BY e.published_at DESC LIMIT ?",
        )
        .bind(podcast_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Épisodes téléchargés et écoutés depuis au moins `hours` heures (0 : tout de suite).
    pub async fn played_downloads(pool: &SqlitePool, hours: i64) -> Result<Vec<i64>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT id FROM podcast_episodes
              WHERE played = 1 AND file_path IS NOT NULL AND played_at IS NOT NULL
                AND played_at <= datetime('now', '-' || ? || ' hours')",
        )
        .bind(hours)
        .fetch_all(pool)
        .await
    }

    pub async fn set_pinned(pool: &SqlitePool, id: i64, pinned: bool) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE podcasts SET pinned = ? WHERE id = ?").bind(pinned).bind(id).execute(pool).await.map(|_| ())
    }

    pub async fn set_cover(pool: &SqlitePool, id: i64, path: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE podcasts SET cover_path = ? WHERE id = ?")
            .bind(path)
            .bind(id)
            .execute(pool)
            .await
            .map(|_| ())
    }

    /// Flux vérifié sans changement.
    pub async fn touch(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE podcasts SET checked_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .map(|_| ())
    }

    // =========================================
    // ENREGISTREMENT D'UN FLUX
    // =========================================

    /// Nouvel abonnement : le podcast et tous ses épisodes, aucun marqué nouveau.
    pub async fn insert(
        pool: &SqlitePool,
        profil_id: i64,
        feed_url: &str,
        feed: &Feed,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<i64, sqlx::Error> {
        let mut tx = pool.begin().await?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO podcasts (profil_id, feed_url, title, author, description, image_url, link, language, etag, last_modified, checked_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP) RETURNING id",
        )
        .bind(profil_id)
        .bind(feed_url)
        .bind(&feed.title)
        .bind(&feed.author)
        .bind(&feed.description)
        .bind(&feed.image_url)
        .bind(&feed.link)
        .bind(&feed.language)
        .bind(etag)
        .bind(last_modified)
        .fetch_one(&mut *tx)
        .await?;
        for e in &feed.episodes {
            Self::upsert_episode(&mut tx, id, e, false).await?;
        }
        tx.commit().await?;
        Self::assign_paths(pool, id).await?;
        Ok(id)
    }

    /// Flux relu : métadonnées mises à jour, épisodes inconnus ajoutés et marqués nouveaux.
    /// Renvoie le nombre d'épisodes ajoutés. L'écoute (position, lu) n'est jamais touchée.
    pub async fn update(
        pool: &SqlitePool,
        id: i64,
        feed: &Feed,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<usize, sqlx::Error> {
        let known: HashSet<String> = sqlx::query_scalar("SELECT guid FROM podcast_episodes WHERE podcast_id = ?")
            .bind(id)
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect();
        let mut tx = pool.begin().await?;
        sqlx::query(
            "UPDATE podcasts SET title = ?, author = ?, description = ?, image_url = ?, link = ?, language = ?,
             etag = ?, last_modified = ?, checked_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(&feed.title)
        .bind(&feed.author)
        .bind(&feed.description)
        .bind(&feed.image_url)
        .bind(&feed.link)
        .bind(&feed.language)
        .bind(etag)
        .bind(last_modified)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        let mut added = 0;
        for e in &feed.episodes {
            let new = !known.contains(&e.guid);
            added += usize::from(new);
            Self::upsert_episode(&mut tx, id, e, new).await?;
        }
        tx.commit().await?;
        Self::assign_paths(pool, id).await?;
        Ok(added)
    }

    async fn upsert_episode(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        podcast_id: i64,
        e: &FeedEpisode,
        is_new: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO podcast_episodes
                (podcast_id, guid, title, description, summary, published_at, duration, audio_url, audio_type,
                 audio_size, image_url, season, episode, is_new, chapters_url, chapters)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (podcast_id, guid) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                summary = excluded.summary,
                published_at = COALESCE(excluded.published_at, podcast_episodes.published_at),
                duration = COALESCE(excluded.duration, podcast_episodes.duration),
                audio_url = excluded.audio_url,
                audio_type = excluded.audio_type,
                audio_size = excluded.audio_size,
                image_url = excluded.image_url,
                season = excluded.season,
                episode = excluded.episode,
                chapters_url = excluded.chapters_url,
                -- Liste du flux, sinon celle déjà trouvée (JSON, ID3) reste ; un lien changé la fait relire.
                chapters = CASE
                    WHEN excluded.chapters IS NOT NULL THEN excluded.chapters
                    WHEN excluded.chapters_url IS NOT podcast_episodes.chapters_url THEN NULL
                    ELSE podcast_episodes.chapters END
            "#,
        )
        .bind(podcast_id)
        .bind(&e.guid)
        .bind(&e.title)
        .bind(&e.description)
        .bind(&e.summary)
        .bind(date(e))
        .bind(e.duration)
        .bind(&e.audio_url)
        .bind(&e.audio_type)
        .bind(e.audio_size)
        .bind(&e.image_url)
        .bind(e.season)
        .bind(e.episode)
        .bind(is_new)
        .bind(&e.chapters_url)
        .bind((!e.chapters.is_empty()).then(|| serde_json::to_string(&e.chapters).unwrap_or_default()))
        .execute(&mut **tx)
        .await
        .map(|_| ())
    }

    // =========================================
    // ÉPISODES
    // =========================================
    pub async fn episodes(pool: &SqlitePool, podcast_id: i64) -> Result<Vec<EpisodeView>, sqlx::Error> {
        sqlx::query_as::<_, EpisodeView>(sqlx::AssertSqlSafe(format!(
            "SELECT {EPISODE_COLUMNS} FROM podcast_episodes WHERE podcast_id = ?
             ORDER BY published_at IS NULL, published_at DESC, id DESC"
        )))
        .bind(podcast_id)
        .fetch_all(pool)
        .await
    }

    pub async fn episode(pool: &SqlitePool, id: i64) -> Result<Option<EpisodeView>, sqlx::Error> {
        let columns = EPISODE_COLUMNS.replace("NULL AS description", "description");
        sqlx::query_as::<_, EpisodeView>(sqlx::AssertSqlSafe(format!("SELECT {columns} FROM podcast_episodes WHERE id = ?")))
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    // =========================================
    // CHEMINS DANS LE CACHE
    // =========================================

    /// Donne un chemin lisible aux épisodes qui n'en ont pas, une fois pour toutes.
    pub async fn assign_paths(pool: &SqlitePool, podcast_id: i64) -> Result<(), sqlx::Error> {
        let rows: Vec<PathRow> = sqlx::query_as(
            "SELECT id, title, published_at, audio_url, audio_type FROM podcast_episodes
             WHERE podcast_id = ? AND local_path IS NULL ORDER BY published_at, id",
        )
        .bind(podcast_id)
        .fetch_all(pool)
        .await?;
        if rows.is_empty() {
            return Ok(());
        }
        let podcast: String = sqlx::query_scalar("SELECT title FROM podcasts WHERE id = ?").bind(podcast_id).fetch_one(pool).await?;
        // Chemins déjà pris, sans la casse : Windows ne distingue pas « Titre » de « titre ».
        let mut taken: HashSet<String> = sqlx::query_scalar::<_, String>("SELECT local_path FROM podcast_episodes WHERE local_path IS NOT NULL")
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|p| p.to_lowercase())
            .collect();
        let mut tx = pool.begin().await?;
        for (id, title, published, url, mime) in rows {
            let mut path = download::episode_path(&podcast, published.as_deref(), &title, &url, mime.as_deref(), None);
            if taken.contains(&path.to_string_lossy().to_lowercase()) {
                path = download::episode_path(&podcast, published.as_deref(), &title, &url, mime.as_deref(), Some(&id.to_string()));
            }
            let path_str = path.to_string_lossy().into_owned();
            taken.insert(path_str.to_lowercase());
            sqlx::query("UPDATE podcast_episodes SET local_path = ? WHERE id = ?")
                .bind(&path_str)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await
    }

    /// L'épisode lu à `path` (chemin du cache, téléchargé ou non).
    pub async fn find_by_path(pool: &SqlitePool, path: &str) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar("SELECT id FROM podcast_episodes WHERE local_path = ?1 OR file_path = ?1 LIMIT 1")
            .bind(path)
            .fetch_optional(pool)
            .await
    }

    /// Chapitres connus (JSON) et lien du fichier de chapitres.
    pub async fn chapters_of(pool: &SqlitePool, episode_id: i64) -> Result<Option<(Option<String>, Option<String>)>, sqlx::Error> {
        sqlx::query_as("SELECT chapters, chapters_url FROM podcast_episodes WHERE id = ?").bind(episode_id).fetch_optional(pool).await
    }

    pub async fn set_chapters(pool: &SqlitePool, episode_id: i64, json: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE podcast_episodes SET chapters = ? WHERE id = ?").bind(json).bind(episode_id).execute(pool).await.map(|_| ())
    }

    /// Émission d'un épisode et sa vitesse propre.
    pub async fn speed_of_episode(pool: &SqlitePool, episode_id: i64) -> Result<Option<(i64, Option<f64>)>, sqlx::Error> {
        sqlx::query_as("SELECT p.id, p.playback_speed FROM podcast_episodes e JOIN podcasts p ON p.id = e.podcast_id WHERE e.id = ?")
            .bind(episode_id)
            .fetch_optional(pool)
            .await
    }

    pub async fn set_speed(pool: &SqlitePool, podcast_id: i64, speed: f64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE podcasts SET playback_speed = ? WHERE id = ?").bind(speed).bind(podcast_id).execute(pool).await.map(|_| ())
    }

    /// Fichiers d'un podcast (téléchargés ou prévus), à effacer quand on le quitte.
    pub async fn files_of(pool: &SqlitePool, podcast_id: i64) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT COALESCE(file_path, local_path) FROM podcast_episodes WHERE podcast_id = ? AND COALESCE(file_path, local_path) IS NOT NULL")
            .bind(podcast_id)
            .fetch_all(pool)
            .await
    }

    /// À écouter : un épisode par émission — celui qu'on a commencé, sinon le plus récent
    /// pas encore écouté. Les émissions en cours d'écoute d'abord ; une émission retirée
    /// ne revient qu'avec un épisode paru après.
    pub async fn latest(pool: &SqlitePool, profil_id: i64, limit: i64) -> Result<Vec<LatestEpisode>, sqlx::Error> {
        sqlx::query_as::<_, LatestEpisode>(
            "SELECT * FROM (
                SELECT e.id, e.podcast_id, e.title, e.summary, NULL AS description, e.published_at, e.duration, e.audio_url,
                       e.audio_type, e.audio_size, e.image_url, e.season, e.episode, e.file_path, e.position, e.played, e.is_new,
                       COALESCE(e.local_path, '') AS path, p.title AS podcast_title, p.cover_path,
                       SUM(e.is_new) OVER (PARTITION BY e.podcast_id) AS new_count,
                       ROW_NUMBER() OVER (
                           PARTITION BY e.podcast_id
                           ORDER BY e.position > 0 DESC, e.published_at DESC, e.id DESC
                       ) AS row_num
                FROM podcast_episodes e JOIN podcasts p ON p.id = e.podcast_id
                WHERE p.profil_id = ? AND e.played = 0
                  AND (p.listen_hidden_at IS NULL OR datetime(e.published_at) > datetime(p.listen_hidden_at))
             )
             WHERE row_num = 1
             ORDER BY position > 0 DESC, published_at DESC, id DESC
             LIMIT ?",
        )
        .bind(profil_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Épisodes téléchargés du profil, les plus récents d'abord ; `podcast_id` pour un seul podcast.
    pub async fn downloads(pool: &SqlitePool, profil_id: i64, podcast_id: Option<i64>) -> Result<Vec<DownloadView>, sqlx::Error> {
        sqlx::query_as::<_, DownloadView>(
            "SELECT e.id, e.podcast_id, p.title AS podcast_title, p.cover_path, e.title, e.published_at, e.duration,
                    e.file_path, e.position, e.played
             FROM podcast_episodes e JOIN podcasts p ON p.id = e.podcast_id
             WHERE p.profil_id = ?1 AND e.file_path IS NOT NULL AND (?2 IS NULL OR e.podcast_id = ?2)
             ORDER BY e.published_at DESC, e.id DESC",
        )
        .bind(profil_id)
        .bind(podcast_id)
        .fetch_all(pool)
        .await
    }

    /// Ce qu'il faut pour lire un épisode : son fichier distant et l'émission.
    pub async fn playable(pool: &SqlitePool, id: i64) -> Result<Option<Playable>, sqlx::Error> {
        sqlx::query_as::<_, Playable>(
            "SELECT e.title, e.audio_url, e.duration, e.position, e.played,
                    p.title AS podcast_title, p.author, p.cover_path
             FROM podcast_episodes e JOIN podcasts p ON p.id = e.podcast_id WHERE e.id = ?",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn set_file(pool: &SqlitePool, id: i64, path: Option<&str>) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE podcast_episodes SET file_path = ? WHERE id = ?")
            .bind(path)
            .bind(id)
            .execute(pool)
            .await
            .map(|_| ())
    }

    /// Position d'écoute, en secondes : l'épisode n'est plus « nouveau ».
    pub async fn set_position(pool: &SqlitePool, id: i64, position: f64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE podcast_episodes SET position = ?, is_new = 0 WHERE id = ?")
            .bind(position.max(0.0))
            .bind(id)
            .execute(pool)
            .await?;
        // Un épisode repris : son émission revient dans « À écouter ».
        sqlx::query("UPDATE podcasts SET listen_hidden_at = NULL WHERE id = (SELECT podcast_id FROM podcast_episodes WHERE id = ?) AND listen_hidden_at IS NOT NULL")
            .bind(id)
            .execute(pool)
            .await
            .map(|_| ())
    }

    /// Vide « À écouter » : toutes les émissions du profil retirées jusqu'à leur prochain épisode.
    pub async fn hide_all_from_listen(pool: &SqlitePool, profil_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE podcasts SET listen_hidden_at = CURRENT_TIMESTAMP WHERE profil_id = ?").bind(profil_id).execute(pool).await.map(|_| ())
    }

    /// Retire l'émission de « À écouter » jusqu'au prochain épisode.
    pub async fn hide_from_listen(pool: &SqlitePool, podcast_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE podcasts SET listen_hidden_at = CURRENT_TIMESTAMP WHERE id = ?").bind(podcast_id).execute(pool).await.map(|_| ())
    }

    /// Écouté (repart du début si on le relance) ou remis à écouter.
    pub async fn set_played(pool: &SqlitePool, id: i64, played: bool) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE podcast_episodes SET played = ?1, is_new = 0, position = 0,
             played_at = CASE WHEN ?1 THEN CURRENT_TIMESTAMP ELSE NULL END WHERE id = ?2",
        )
        .bind(played)
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
    }

    /// L'historique du profil effacé : épisodes non écoutés, sans position de reprise.
    pub async fn clear_history(pool: &SqlitePool, profil_id: i64) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query(
            "UPDATE podcast_episodes SET played = 0, played_at = NULL, position = 0
             WHERE podcast_id IN (SELECT id FROM podcasts WHERE profil_id = ?) AND (played = 1 OR position > 0)",
        )
        .bind(profil_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE podcasts SET listen_hidden_at = NULL WHERE profil_id = ?").bind(profil_id).execute(&mut *tx).await?;
        tx.commit().await
    }

    /// Tous les épisodes d'un podcast marqués écoutés.
    pub async fn set_all_played(pool: &SqlitePool, podcast_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE podcast_episodes SET played = 1, is_new = 0, position = 0, played_at = CURRENT_TIMESTAMP
             WHERE podcast_id = ? AND played = 0",
        )
        .bind(podcast_id)
        .execute(pool)
        .await
        .map(|_| ())
    }
    /// Abonnements du profil pour l'OPML : (titre, flux, site).
    pub async fn opml_items(pool: &SqlitePool, profil_id: i64) -> Result<Vec<(String, String, Option<String>)>, sqlx::Error> {
        sqlx::query_as("SELECT title, feed_url, link FROM podcasts WHERE profil_id = ? ORDER BY position, title")
            .bind(profil_id)
            .fetch_all(pool)
            .await
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::podcast::feed::FeedEpisode;
    use chrono::TimeZone;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn base() -> SqlitePool {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.expect("base mémoire");
        sqlx::migrate!("./SQL/").run(&pool).await.expect("migrations");
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle'), (2, 'Autre')")
            .execute(&pool)
            .await
            .expect("profils");
        pool
    }

    fn episode(guid: &str, day: u32) -> FeedEpisode {
        FeedEpisode {
            guid: guid.into(),
            title: format!("Épisode {guid}"),
            audio_url: format!("https://ex.com/{guid}.mp3"),
            published_at: Some(chrono::Utc.with_ymd_and_hms(2025, 10, day, 6, 0, 0).unwrap()),
            duration: Some(1800),
            ..Default::default()
        }
    }

    fn feed(episodes: Vec<FeedEpisode>) -> Feed {
        Feed { title: "Mon podcast".into(), episodes, ..Default::default() }
    }

    #[tokio::test]
    async fn subscribing_keeps_the_backlog_out_of_the_new_badge() {
        let pool = base().await;
        let id = PodcastRepository::insert(&pool, 1, "https://ex.com/rss", &feed(vec![episode("a", 1), episode("b", 2)]), Some("e1"), None)
            .await
            .unwrap();
        let p = PodcastRepository::find_one(&pool, id).await.unwrap().unwrap();
        assert_eq!((p.episode_count, p.unplayed_count, p.new_count), (2, 2, 0));
        assert_eq!(p.latest_at.as_deref(), Some("2025-10-02 06:00:00"));
        let eps = PodcastRepository::episodes(&pool, id).await.unwrap();
        assert_eq!(eps.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(), ["Épisode b", "Épisode a"]);
        assert!(eps[0].description.is_none());
    }

    #[tokio::test]
    async fn a_hidden_show_comes_back_with_a_new_episode_or_a_resume() {
        let pool = base().await;
        let id = PodcastRepository::insert(&pool, 1, "https://ex.com/rss", &feed(vec![episode("a", 1)]), None, None).await.unwrap();
        let shown = || async { PodcastRepository::latest(&pool, 1, 20).await.unwrap().len() };
        assert_eq!(shown().await, 1);
        PodcastRepository::hide_from_listen(&pool, id).await.unwrap();
        assert_eq!(shown().await, 0);
        // Reprise d'un épisode : l'émission revient.
        let a = PodcastRepository::episodes(&pool, id).await.unwrap()[0].id;
        PodcastRepository::set_position(&pool, a, 60.0).await.unwrap();
        assert_eq!(shown().await, 1);
        // Retirée de nouveau, puis un épisode paraît après : elle revient.
        PodcastRepository::hide_from_listen(&pool, id).await.unwrap();
        sqlx::query("UPDATE podcast_episodes SET published_at = datetime('now', '+1 hour')").execute(&pool).await.unwrap();
        assert_eq!(shown().await, 1);
    }

    #[tokio::test]
    async fn each_show_keeps_its_own_speed() {
        let pool = base().await;
        let id = PodcastRepository::insert(&pool, 1, "https://ex.com/rss", &feed(vec![episode("a", 1)]), None, None).await.unwrap();
        let a = PodcastRepository::episodes(&pool, id).await.unwrap()[0].id;
        assert_eq!(PodcastRepository::speed_of_episode(&pool, a).await.unwrap(), Some((id, None)));
        PodcastRepository::set_speed(&pool, id, 1.5).await.unwrap();
        assert_eq!(PodcastRepository::speed_of_episode(&pool, a).await.unwrap(), Some((id, Some(1.5))));
        assert_eq!(PodcastRepository::speed_of_episode(&pool, a + 99).await.unwrap(), None);
    }

    #[tokio::test]
    async fn refreshing_adds_only_unknown_episodes_and_keeps_listening_state() {
        let pool = base().await;
        let id = PodcastRepository::insert(&pool, 1, "https://ex.com/rss", &feed(vec![episode("a", 1)]), None, None).await.unwrap();
        let a = PodcastRepository::episodes(&pool, id).await.unwrap()[0].id;
        PodcastRepository::set_position(&pool, a, 600.0).await.unwrap();

        let mut retitled = episode("a", 1);
        retitled.title = "Titre corrigé".into();
        retitled.duration = None;
        let added = PodcastRepository::update(&pool, id, &feed(vec![retitled, episode("c", 3)]), None, None).await.unwrap();
        assert_eq!(added, 1);

        let eps = PodcastRepository::episodes(&pool, id).await.unwrap();
        let (c, a) = (&eps[0], &eps[1]);
        assert!(c.is_new && !a.is_new);
        assert_eq!(a.title, "Titre corrigé");
        assert_eq!(a.position, 600.0);
        assert_eq!(a.duration, Some(1800), "une durée absente du flux ne l'efface pas");
        assert_eq!(PodcastRepository::find_one(&pool, id).await.unwrap().unwrap().new_count, 1);

        PodcastRepository::set_played(&pool, c.id, true).await.unwrap();
        let p = PodcastRepository::find_one(&pool, id).await.unwrap().unwrap();
        assert_eq!((p.unplayed_count, p.new_count), (1, 0));
    }

    #[tokio::test]
    async fn gives_each_episode_a_readable_unique_path() {
        let pool = base().await;
        let mut twin = episode("b", 1);
        twin.title = "Épisode a".into();
        let id = PodcastRepository::insert(&pool, 1, "https://ex.com/rss", &feed(vec![episode("a", 1), twin]), None, None).await.unwrap();
        let eps = PodcastRepository::episodes(&pool, id).await.unwrap();
        let names: Vec<String> = eps.iter().map(|e| std::path::Path::new(&e.path).file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert!(names.contains(&"2025-10-01 Épisode a.mp3".to_string()), "{names:?}");
        assert!(names.iter().any(|n| n.starts_with("2025-10-01 Épisode a (")), "homonyme départagé : {names:?}");
        assert!(eps.iter().all(|e| std::path::Path::new(&e.path).parent().unwrap().ends_with("Mon podcast")));

        let first = &eps[0];
        assert_eq!(PodcastRepository::find_by_path(&pool, &first.path).await.unwrap(), Some(first.id));
        assert!(PodcastRepository::downloads(&pool, 1, None).await.unwrap().is_empty());
        PodcastRepository::set_file(&pool, first.id, Some(&first.path)).await.unwrap();
        let dl = PodcastRepository::downloads(&pool, 1, None).await.unwrap();
        assert_eq!((dl.len(), dl[0].podcast_title.as_str()), (1, "Mon podcast"));
        assert!(PodcastRepository::downloads(&pool, 2, None).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn to_listen_keeps_one_episode_per_show() {
        let pool = base().await;
        let a = PodcastRepository::insert(&pool, 1, "https://ex.com/a", &feed(vec![episode("a1", 1), episode("a2", 2), episode("a3", 3)]), None, None)
            .await
            .unwrap();
        PodcastRepository::insert(&pool, 1, "https://ex.com/b", &feed(vec![episode("b1", 5)]), None, None).await.unwrap();
        // Commencé sur l'émission A : c'est lui qu'on propose, et A passe devant.
        let started = PodcastRepository::episodes(&pool, a).await.unwrap().into_iter().find(|e| e.title == "Épisode a1").unwrap();
        PodcastRepository::set_position(&pool, started.id, 120.0).await.unwrap();

        let list = PodcastRepository::latest(&pool, 1, 20).await.unwrap();
        let titles: Vec<&str> = list.iter().map(|l| l.episode.title.as_str()).collect();
        assert_eq!(titles, ["Épisode a1", "Épisode b1"]);
        assert!(PodcastRepository::latest(&pool, 2, 20).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn subscriptions_belong_to_one_profile_and_cascade_on_delete() {
        let pool = base().await;
        let id = PodcastRepository::insert(&pool, 1, "https://ex.com/rss", &feed(vec![episode("a", 1)]), None, None).await.unwrap();
        assert!(PodcastRepository::insert(&pool, 1, "https://ex.com/rss", &feed(vec![]), None, None).await.is_err());
        assert!(PodcastRepository::insert(&pool, 2, "https://ex.com/rss", &feed(vec![]), None, None).await.is_ok());
        assert_eq!(PodcastRepository::find_id_by_url(&pool, 1, "https://ex.com/rss").await.unwrap(), Some(id));
        assert_eq!(PodcastRepository::find_by_profile(&pool, 1).await.unwrap().len(), 1);

        PodcastRepository::delete(&pool, id).await.unwrap();
        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM podcast_episodes WHERE podcast_id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(left, 0);
    }
}
