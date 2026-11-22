use std::str::FromStr;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::core::batch::runner::BatchRegistry;
use crate::core::dlna_server::server::DlnaServer;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    /// DLNA server lifecycle. `None` when stopped, `Some(...)` when running.
    /// Wrapped in Mutex so start/stop commands serialize properly.
    pub dlna_server: Arc<Mutex<Option<DlnaServer>>>,
    /// Lots en cours. Point de rendez-vous entre la commande qui lance un
    /// traitement de masse et celle qui l'annule — elles arrivent par deux
    /// appels séparés.
    pub batch: Arc<BatchRegistry>,
}

impl AppState {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        // ═══════════════════════════════════════════════════════════════
        // PRAGMAS SQLite — Configuration performance
        // ═══════════════════════════════════════════════════════════════
        //
        // Ces commandes configurent le moteur SQLite pour une app desktop.
        // Elles s'appliquent à chaque connexion que le pool ouvre.
        //
        // WAL (Write-Ahead Logging) :
        //   - Les écritures vont dans un fichier .wal séparé
        //   - Les lectures continuent sur l'ancienne version pendant l'écriture
        //   - Résultat : lectures et écritures sont CONCURRENTES
        //   - L'UI ne freeze plus pendant un import massif
        //
        // synchronous = NORMAL :
        //   - Par défaut SQLite fait un fsync() à chaque COMMIT (mode FULL)
        //   - En NORMAL, fsync seulement aux checkpoints WAL
        //   - Si crash pendant un COMMIT : on perd 1 transaction mais la DB reste cohérente
        //   - Bon compromis perf/sécurité pour une app desktop
        //
        // cache_size = -64000 (64 MB) :
        //   - Le cache par défaut est 2 MB (2000 pages)
        //   - On le monte à 64 MB pour garder plus de données en RAM
        //   - Les requêtes répétées ne relisent pas le disque
        //   - Valeur négative = en kilobytes
        //
        // temp_store = MEMORY :
        //   - Les tables temporaires (ORDER BY, GROUP BY) vont en RAM
        //   - Au lieu de fichiers temporaires sur disque
        //   - Plus rapide pour les tris et agrégations
        //
        // Posés dans les options de connexion, ils valent pour CHAQUE connexion
        // du pool. Exécutés une fois sur le pool, seul `journal_mode` (enregistré
        // dans le fichier) s'appliquait partout ; les trois autres ne touchaient
        // que la connexion qui les avait reçus.
        let options = SqliteConnectOptions::from_str(database_url)?
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .pragma("cache_size", "-64000")
            .pragma("temp_store", "MEMORY");
        let pool: sqlx::Pool<sqlx::Sqlite> = SqlitePool::connect_with(options).await?;

        log::debug!("PRAGMAs SQLite configurés (WAL + NORMAL + 64MB cache)");

        // ═══════════════════════════════════════════════════════════════
        // Migrations — Création/mise à jour du schéma
        // ═══════════════════════════════════════════════════════════════
        // L'erreur remonte, elle ne tue pas le processus. Un `expect` ici est
        // invisible : une application graphique n'a pas de console sous
        // Windows, le panic part dans le vide, et l'utilisateur voit une
        // application qui « ne se lance pas » sans un mot d'explication.
        backup_before_migration(&pool, database_url).await;
        align_line_ending_checksums(&pool).await;

        sqlx::migrate!("./SQL/").run(&pool).await?;

        log::info!("✅ Migrations terminées avec succès !");

        // ═══════════════════════════════════════════════════════════════
        // INDEX — Accélérer les requêtes fréquentes
        // ═══════════════════════════════════════════════════════════════
        //
        // Sans index, SQLite fait un "full table scan" : il lit CHAQUE
        // ligne de la table pour trouver celles qui matchent le WHERE.
        // Avec 8000+ tracks, c'est lent.
        //
        // Un index est comme l'index d'un livre : au lieu de lire
        // toutes les pages, on va directement à la bonne page.
        //
        // CREATE INDEX IF NOT EXISTS → ne recrée pas l'index s'il existe
        //
        // Convention de nommage : idx_{table}_{colonne(s)}
        //
        // On indexe les colonnes utilisées dans les clauses WHERE et JOIN
        // les plus fréquentes :

        let indexes = [
            // library_tracks : la table la plus requêtée
            // WHERE library_id = ? → utilisé dans TOUTES les pages
            "CREATE INDEX IF NOT EXISTS idx_lt_library_id ON library_tracks(library_id)",
            // WHERE file_id = ? → utilisé pour le skip unchanged à l'import
            "CREATE INDEX IF NOT EXISTS idx_lt_file_id ON library_tracks(file_id)",
            // WHERE cache_id = ? → JOIN avec library_cache
            "CREATE INDEX IF NOT EXISTS idx_lt_cache_id ON library_tracks(cache_id)",
            // WHERE library_id + ORDER BY play_count → page stats top écoutés
            "CREATE INDEX IF NOT EXISTS idx_lt_play_count ON library_tracks(library_id, play_count DESC)",

            // library_albums : WHERE library_id = ?
            "CREATE INDEX IF NOT EXISTS idx_la_library_id ON library_albums(library_id)",
            // library_albums : WHERE artist_id = ? → page artiste (albums de l'artiste)
            "CREATE INDEX IF NOT EXISTS idx_la_artist_id ON library_albums(artist_id)",
            // library_albums : genre lookup → artistes similaires
            "CREATE INDEX IF NOT EXISTS idx_la_genre ON library_albums(library_id, genre)",
            // Compatibilité descendante. Une version antérieure à la 0.2.5 vise
            // `ON CONFLICT(library_id, artist_id, title_normalized)`, que la
            // migration a supprimé : sans cet index elle ne peut plus indexer
            // un seul fichier. Il échoue sans bruit si des doublons existent.
            crate::repository::library::library_album_repository::INDEX_COMPAT_0_2_4,

            // library_tracks : WHERE artist_id = ? → page artiste (tracks)
            "CREATE INDEX IF NOT EXISTS idx_lt_artist_id ON library_tracks(artist_id)",
            // Compter les pistes d'un artiste dans une bibliothèque : sans cet
            // index le planificateur retombait sur `library_id` seul, des
            // milliers de lignes balayées par artiste.
            "CREATE INDEX IF NOT EXISTS idx_lt_library_artist ON library_tracks(library_id, artist_id)",
            // library_tracks : WHERE library_album_id = ? → page album
            "CREATE INDEX IF NOT EXISTS idx_lt_album_id ON library_tracks(library_album_id)",

            // library_artists : WHERE library_id = ?
            "CREATE INDEX IF NOT EXISTS idx_lart_library_id ON library_artists(library_id)",
            // library_artists : WHERE artist_id = ? → recherche, page artiste
            "CREATE INDEX IF NOT EXISTS idx_lart_artist_id ON library_artists(artist_id)",

            // library_track_artists : la table de liaison artiste ↔ track
            // C'est une table N:N, les JOINs dessus sont fréquents
            "CREATE INDEX IF NOT EXISTS idx_lta_library_track ON library_track_artists(library_track_id)",
            "CREATE INDEX IF NOT EXISTS idx_lta_artist ON library_track_artists(artist_id)",
            "CREATE INDEX IF NOT EXISTS idx_lta_library ON library_track_artists(library_id)",

            // library_files : WHERE path = ? → vérification d'existence à l'import
            "CREATE INDEX IF NOT EXISTS idx_lf_path ON library_files(path)",

            // library_cache : WHERE genre → page genres
            "CREATE INDEX IF NOT EXISTS idx_lc_genre ON library_cache(genre)",
        ];

        for idx in &indexes {
            if let Err(e) = sqlx::query(*idx).execute(&pool).await {
                log::warn!("Index creation warning: {}", e);
            }
        }

        log::debug!("Index SQLite créés/vérifiés");

        // Après les index, et non avant : il recompte les pistes de chaque
        // artiste, ce qui sans `idx_lt_library_artist` prenait 2,7 s.
        //
        // La migration laisse `album_dir` vide ; sans ce passage l'indexation
        // ne retrouverait plus les albums déjà en base. Idempotent et muet
        // quand il n'y a rien à faire.
        if let Err(e) = crate::service::library::album_merge::consolidate(&pool).await {
            log::error!("⚠️ Rassemblement des albums : {e}");
        }

        Ok(Self {
            pool,
            dlna_server: Arc::new(Mutex::new(None)),
            batch: Arc::new(BatchRegistry::default()),
        })
    }
}

/// Le fichier de base derrière une URL `sqlite:...?mode=rwc`.
fn base_path(url: &str) -> Option<String> {
    let without_scheme = url.strip_prefix("sqlite:")?;
    let path_part = without_scheme.split('?').next()?;
    if path_part.is_empty() || path_part == ":memory:" {
        return None;
    }
    Some(path_part.to_string())
}

/// Réaligne les empreintes de migrations qui ne diffèrent que par les fins de ligne.
///
/// sqlx compare l'empreinte SHA-384 du texte exact de chaque migration déjà appliquée.
/// Le même fichier compilé sur Windows (CRLF) ou ailleurs (LF) n'a pas la même
/// empreinte, et une base passée d'une compilation à l'autre refuserait de démarrer.
/// Quand l'écart ne tient qu'aux fins de ligne, l'empreinte enregistrée prend celle
/// de cette version ; tout autre écart reste signalé par sqlx.
async fn align_line_ending_checksums(pool: &SqlitePool) {
    use sha2::{Digest, Sha384};
    // Table absente : installation neuve, rien à réaligner.
    let Ok(applied) = sqlx::query_as::<_, (i64, Vec<u8>)>("SELECT version, checksum FROM _sqlx_migrations")
        .fetch_all(pool)
        .await
    else {
        return;
    };
    let migrator = sqlx::migrate!("./SQL/");
    for (version, stored) in applied {
        let Some(migration) = migrator
            .migrations
            .iter()
            .find(|m| m.version == version && !m.migration_type.is_down_migration())
        else {
            continue;
        };
        if *migration.checksum == *stored {
            continue;
        }
        let lf = migration.sql.as_str().replace("\r\n", "\n");
        let crlf = lf.replace('\n', "\r\n");
        let same_text = [&lf, &crlf].iter().any(|text| Sha384::digest(text.as_bytes()).as_slice() == stored.as_slice());
        if !same_text {
            continue;
        }
        let updated = sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?")
            .bind(&*migration.checksum)
            .bind(version)
            .execute(pool)
            .await;
        match updated {
            Ok(_) => log::info!("🗄️  Migration {version} : empreinte réalignée (fins de ligne)"),
            Err(e) => log::warn!("🗄️  Migration {version} : empreinte non réalignée : {e}"),
        }
    }
}

/// Première migration propre à la 0.5 : une base qui ne l'a pas encore reçue vient de la 0.4.
const FIRST_0_5_MIGRATION: i64 = 202611160001;

/// Copie la base dans `copy` (`VACUUM INTO`) ; `true` si elle est faite.
async fn snapshot(pool: &SqlitePool, copy: &str) -> bool {
    // `VACUUM INTO` n'accepte pas de paramètre lié ; le chemin vient de notre
    // propre dossier de données, pas de l'utilisateur.
    let escaped = copy.replace('\'', "''");
    match sqlx::query(sqlx::AssertSqlSafe(format!("VACUUM INTO '{escaped}'")))
        .execute(pool)
        .await
    {
        Ok(_) => true,
        Err(e) => {
            log::error!("⚠️ Sauvegarde impossible ({e}) : {copy}");
            false
        }
    }
}

/// Une copie de la base avant d'appliquer une migration en attente.
///
/// Celles qui reconstruisent une table touchent au cœur de la bibliothèque, et
/// une transaction annulée ne protège pas d'un disque plein ou d'un schéma qui
/// a dérivé. `VACUUM INTO` prend un instantané cohérent, WAL compris, là où une
/// copie de fichier laisserait le journal de côté.
async fn backup_before_migration(pool: &SqlitePool, database_url: &str) {
    let expected_value = sqlx::migrate!("./SQL/")
        .migrations
        .last()
        .map(|m| m.version)
        .unwrap_or(0);

    // Table absente : installation neuve, il n'y a rien à perdre.
    let applied = match sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MAX(version) FROM _sqlx_migrations",
    )
    .fetch_one(pool)
    .await
    {
        Ok(Some(v)) => v,
        _ => return,
    };

    if applied >= expected_value {
        return;
    }

    let Some(path_part) = base_path(database_url) else {
        return;
    };

    // Passage de la 0.4 à la 0.5 : une copie gardée à part, faite une seule fois et
    // jamais remplacée par les sauvegardes suivantes, pour revenir à la 0.4 au besoin.
    if applied < FIRST_0_5_MIGRATION {
        let copy = format!("{path_part}.0.4.bak");
        if !std::path::Path::new(&copy).exists() && snapshot(pool, &copy).await {
            log::info!("💾 Base de la 0.4 sauvegardée : {copy}");
            return;
        }
    }

    // Une seule sauvegarde à la fois : 83 Mo par copie sur une vraie
    // bibliothèque, et seule la plus récente sert à quelque chose.
    if let Some(folder) = std::path::Path::new(&path_part).parent() {
        if let Ok(entries) = std::fs::read_dir(folder) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains(".avant-") && name.ends_with(".bak") {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }

    let copy = format!("{path_part}.avant-{applied}.bak");
    let _ = std::fs::remove_file(&copy);
    // Sans copie, on continue : la migration reste transactionnelle.
    if snapshot(pool, &copy).await {
        log::info!("💾 Base sauvegardée avant migration : {copy}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_url_gives_the_file() {
        assert_eq!(
            base_path(r"sqlite:C:\Users\d\AppData\rustmusic.db?mode=rwc").as_deref(),
            Some(r"C:\Users\d\AppData\rustmusic.db")
        );
        assert_eq!(base_path("sqlite:/home/d/m.db").as_deref(), Some("/home/d/m.db"));
        assert_eq!(base_path("sqlite::memory:"), None);
        assert_eq!(base_path("postgres://x"), None);
    }

    #[tokio::test]
    async fn an_outdated_database_is_backed_up() {
        let folder = std::env::temp_dir().join(format!("rm-sauv-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("dossier");
        let path_part = folder.join("essai.db");
        let _ = std::fs::remove_file(&path_part);
        let url = format!("sqlite:{}?mode=rwc", path_part.display());

        let pool = SqlitePool::connect(&url).await.expect("base");
        sqlx::query("CREATE TABLE _sqlx_migrations (version BIGINT PRIMARY KEY)")
            .execute(&pool).await.expect("table");
        sqlx::query("INSERT INTO _sqlx_migrations VALUES (1)")
            .execute(&pool).await.expect("version en retard");

        // Base de la 0.4 : sa copie à part, une seule fois.
        backup_before_migration(&pool, &url).await;
        let v04 = format!("{}.0.4.bak", path_part.display());
        assert!(std::path::Path::new(&v04).exists(), "copie de la 0.4 attendue en {v04}");
        let copy = format!("{}.avant-1.bak", path_part.display());
        assert!(!std::path::Path::new(&copy).exists());
        // Encore en retard : la sauvegarde ordinaire, la copie de la 0.4 reste.
        backup_before_migration(&pool, &url).await;
        assert!(std::path::Path::new(&copy).exists(), "sauvegarde attendue en {copy}");
        assert!(std::path::Path::new(&v04).exists(), "copie de la 0.4 gardée");

        // À jour : plus de sauvegarde.
        let last_one = sqlx::migrate!("./SQL/").migrations.last().map(|m| m.version).unwrap_or(0);
        sqlx::query("UPDATE _sqlx_migrations SET version = ?").bind(last_one)
            .execute(&pool).await.expect("mise à jour");
        std::fs::remove_file(&copy).expect("retrait");
        backup_before_migration(&pool, &url).await;
        assert!(!std::path::Path::new(&copy).exists(), "aucune sauvegarde attendue");

        pool.close().await;
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[tokio::test]
    async fn every_connection_gets_pragmas() {
        let folder = std::env::temp_dir().join(format!("rm-pragmas-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("dossier");
        let url = format!("sqlite:{}?mode=rwc", folder.join("essai.db").display());
        let state = AppState::new(&url).await.expect("base");

        // Tenues en même temps : le pool ne peut pas resservir la même.
        let mut connections = Vec::new();
        for _ in 0..3 {
            connections.push(state.pool.acquire().await.expect("connexion"));
        }
        for c in connections.iter_mut() {
            let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous").fetch_one(&mut **c).await.expect("synchronous");
            let temp_store: i64 = sqlx::query_scalar("PRAGMA temp_store").fetch_one(&mut **c).await.expect("temp_store");
            let cache_size: i64 = sqlx::query_scalar("PRAGMA cache_size").fetch_one(&mut **c).await.expect("cache_size");
            assert_eq!((synchronous, temp_store, cache_size), (1, 2, -64000), "NORMAL, MEMORY, 64 Mo");
        }

        drop(connections);
        state.pool.close().await;
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[tokio::test]
    async fn line_endings_do_not_block_startup() {
        use sha2::{Digest, Sha384};
        let folder = std::env::temp_dir().join(format!("rm-crlf-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("dossier");
        let url = format!("sqlite:{}?mode=rwc", folder.join("essai.db").display());
        let migration = sqlx::migrate!("./SQL/").migrations.iter().find(|m| m.sql.as_str().contains('\n')).cloned().expect("migration");
        let sql = migration.sql.as_str();
        // L'autre fin de ligne que celle compilée ici.
        let other = if sql.contains("\r\n") { sql.replace("\r\n", "\n") } else { sql.replace('\n', "\r\n") };

        let set_checksum = |pool: SqlitePool, text: String| async move {
            sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?")
                .bind(Sha384::digest(text.as_bytes()).to_vec())
                .bind(migration.version)
                .execute(&pool)
                .await
                .expect("empreinte");
            pool.close().await;
        };

        let state = AppState::new(&url).await.expect("base");
        set_checksum(state.pool, other).await;
        let state = AppState::new(&url).await.expect("même texte, autre fin de ligne : la base démarre");

        // Un vrai changement de texte reste refusé.
        set_checksum(state.pool, format!("{sql}-- modifiée")).await;
        assert!(AppState::new(&url).await.is_err(), "migration modifiée : refus attendu");

        let _ = std::fs::remove_dir_all(&folder);
    }
}
