use std::sync::Arc;

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
        let pool: sqlx::Pool<sqlx::Sqlite> = SqlitePool::connect(database_url).await?;

        // ═══════════════════════════════════════════════════════════════
        // PRAGMAS SQLite — Configuration performance
        // ═══════════════════════════════════════════════════════════════
        //
        // Ces commandes configurent le moteur SQLite pour une app desktop.
        // Elles sont exécutées une seule fois au démarrage, sur la connexion.
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
        sqlx::query("PRAGMA journal_mode = WAL").execute(&pool).await?;
        sqlx::query("PRAGMA synchronous = NORMAL").execute(&pool).await?;
        sqlx::query("PRAGMA cache_size = -64000").execute(&pool).await?;
        sqlx::query("PRAGMA temp_store = MEMORY").execute(&pool).await?;

        log::debug!("PRAGMAs SQLite configurés (WAL + NORMAL + 64MB cache)");

        // ═══════════════════════════════════════════════════════════════
        // Migrations — Création/mise à jour du schéma
        // ═══════════════════════════════════════════════════════════════
        sqlx::migrate!("./SQL/")
                .run(&pool)
                .await
                .expect("Erreur fatale lors de l'exécution des migrations SQLX !");

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

            // library_tracks : WHERE artist_id = ? → page artiste (tracks)
            "CREATE INDEX IF NOT EXISTS idx_lt_artist_id ON library_tracks(artist_id)",
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

            // recent_files : ORDER BY last_played_at → récents
            "CREATE INDEX IF NOT EXISTS idx_rf_last_played ON recent_files(last_played_at DESC)",
        ];

        for idx in &indexes {
            if let Err(e) = sqlx::query(idx).execute(&pool).await {
                log::warn!("Index creation warning: {}", e);
            }
        }

        log::debug!("Index SQLite créés/vérifiés");

        Ok(Self {
            pool,
            dlna_server: Arc::new(Mutex::new(None)),
            batch: Arc::new(BatchRegistry::default()),
        })
    }
}