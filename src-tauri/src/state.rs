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
        // L'erreur remonte, elle ne tue pas le processus. Un `expect` ici est
        // invisible : une application graphique n'a pas de console sous
        // Windows, le panic part dans le vide, et l'utilisateur voit une
        // application qui « ne se lance pas » sans un mot d'explication.
        sauvegarder_avant_migration(&pool, database_url).await;

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

            // recent_files : ORDER BY last_played_at → récents
            "CREATE INDEX IF NOT EXISTS idx_rf_last_played ON recent_files(last_played_at DESC)",
        ];

        for idx in &indexes {
            if let Err(e) = sqlx::query(idx).execute(&pool).await {
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
        if let Err(e) = crate::service::library::album_merge::consolider(&pool).await {
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
fn chemin_base(url: &str) -> Option<String> {
    let sans_schema = url.strip_prefix("sqlite:")?;
    let chemin = sans_schema.split('?').next()?;
    if chemin.is_empty() || chemin == ":memory:" {
        return None;
    }
    Some(chemin.to_string())
}

/// Une copie de la base avant d'appliquer une migration en attente.
///
/// Celles qui reconstruisent une table touchent au cœur de la bibliothèque, et
/// une transaction annulée ne protège pas d'un disque plein ou d'un schéma qui
/// a dérivé. `VACUUM INTO` prend un instantané cohérent, WAL compris, là où une
/// copie de fichier laisserait le journal de côté.
async fn sauvegarder_avant_migration(pool: &SqlitePool, database_url: &str) {
    let attendue = sqlx::migrate!("./SQL/")
        .migrations
        .last()
        .map(|m| m.version)
        .unwrap_or(0);

    // Table absente : installation neuve, il n'y a rien à perdre.
    let appliquee = match sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MAX(version) FROM _sqlx_migrations",
    )
    .fetch_one(pool)
    .await
    {
        Ok(Some(v)) => v,
        _ => return,
    };

    if appliquee >= attendue {
        return;
    }

    let Some(chemin) = chemin_base(database_url) else {
        return;
    };

    // Une seule sauvegarde à la fois : 83 Mo par copie sur une vraie
    // bibliothèque, et seule la plus récente sert à quelque chose.
    if let Some(dossier) = std::path::Path::new(&chemin).parent() {
        if let Ok(entrees) = std::fs::read_dir(dossier) {
            for entree in entrees.flatten() {
                let nom = entree.file_name().to_string_lossy().to_string();
                if nom.contains(".avant-") && nom.ends_with(".bak") {
                    let _ = std::fs::remove_file(entree.path());
                }
            }
        }
    }

    let copie = format!("{chemin}.avant-{appliquee}.bak");
    let _ = std::fs::remove_file(&copie);

    // `VACUUM INTO` n'accepte pas de paramètre lié ; le chemin vient de notre
    // propre dossier de données, pas de l'utilisateur.
    let echappe = copie.replace('\'', "''");
    match sqlx::query(&format!("VACUUM INTO '{echappe}'"))
        .execute(pool)
        .await
    {
        Ok(_) => log::info!("💾 Base sauvegardée avant migration : {copie}"),
        Err(e) => log::error!(
            "⚠️ Sauvegarde impossible avant migration ({e}) — on continue, \
             la migration reste transactionnelle"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_url_donne_le_fichier() {
        assert_eq!(
            chemin_base(r"sqlite:C:\Users\d\AppData\rustmusic.db?mode=rwc").as_deref(),
            Some(r"C:\Users\d\AppData\rustmusic.db")
        );
        assert_eq!(chemin_base("sqlite:/home/d/m.db").as_deref(), Some("/home/d/m.db"));
        assert_eq!(chemin_base("sqlite::memory:"), None);
        assert_eq!(chemin_base("postgres://x"), None);
    }

    #[tokio::test]
    async fn une_base_en_retard_est_sauvegardee() {
        let dossier = std::env::temp_dir().join(format!("rm-sauv-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).expect("dossier");
        let chemin = dossier.join("essai.db");
        let _ = std::fs::remove_file(&chemin);
        let url = format!("sqlite:{}?mode=rwc", chemin.display());

        let pool = SqlitePool::connect(&url).await.expect("base");
        sqlx::query("CREATE TABLE _sqlx_migrations (version BIGINT PRIMARY KEY)")
            .execute(&pool).await.expect("table");
        sqlx::query("INSERT INTO _sqlx_migrations VALUES (1)")
            .execute(&pool).await.expect("version en retard");

        sauvegarder_avant_migration(&pool, &url).await;
        let copie = format!("{}.avant-1.bak", chemin.display());
        assert!(std::path::Path::new(&copie).exists(), "sauvegarde attendue en {copie}");

        // À jour : plus de sauvegarde.
        let derniere = sqlx::migrate!("./SQL/").migrations.last().map(|m| m.version).unwrap_or(0);
        sqlx::query("UPDATE _sqlx_migrations SET version = ?").bind(derniere)
            .execute(&pool).await.expect("mise à jour");
        std::fs::remove_file(&copie).expect("retrait");
        sauvegarder_avant_migration(&pool, &url).await;
        assert!(!std::path::Path::new(&copie).exists(), "aucune sauvegarde attendue");

        pool.close().await;
        let _ = std::fs::remove_dir_all(&dossier);
    }
}
