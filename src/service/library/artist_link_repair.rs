//! Reprise des liaisons piste ↔ artiste sur les bibliothèques déjà indexées.
//!
//! Les versions antérieures ne posaient pas `library_track_artists` : seul le
//! premier artiste d'un tag « X;Y » était crédité. Un rescan ne rattrape rien,
//! il saute les fichiers indexés. Ne relit aucun fichier — le tag brut est déjà
//! en base dans `library_cache.artist`.

use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::entity::artist::artist::ArtistCreate;
use crate::entity::library::library_artist::LibraryArtistCreate;
use crate::entity::library::library_track_artist::LibraryTrackArtistCreate;
use crate::helper::string::string::{normalize_name, normalize_sort_name};
use crate::repository::artist::artist_repository::ArtistRepository;
use crate::repository::library::library_artist_repository::LibraryArtistRepository;
use crate::repository::library::library_track_artist_repository::LibraryTrackArtistRepository;
use crate::repository::library::library_track_repository::LibraryTrackRepository;

/// Ce que la reprise a posé. `links_created` ne compte pas les existantes.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtistLinkReport {
    pub tracks_seen: i64,
    pub tracks_multi: i64,
    pub links_created: i64,
    pub artists_created: i64,
    /// Pistes dont l'artiste principal ne correspondait pas à leur tag.
    pub main_artist_fixed: i64,
}

/// Toutes les bibliothèques si `library_id` est `None`. Idempotent : upsert sur
/// `(library_track_id, artist_id)`.
pub async fn repair(
    pool: &SqlitePool,
    library_id: Option<i64>,
) -> Result<ArtistLinkReport, String> {
    let tracks_list = LibraryTrackRepository::find_artist_tags(pool, library_id)
        .await
        .map_err(|e| format!("Lecture des pistes : {e}"))?;

    let rules = crate::service::library::artist_split::rules();
    let mut report = ArtistLinkReport {
        tracks_seen: tracks_list.len() as i64,
        ..Default::default()
    };

    // Cache par nom normalisé, la clé de l'index unique de `artists` : évite un
    // aller-retour par piste.
    let mut known_items: HashMap<String, String> = ArtistRepository::find_ids_by_normalized_name(pool)
    .await
    .map_err(|e| format!("Lecture des artistes : {e}"))?
    .into_iter()
    .collect();

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Ouverture de la transaction : {e}"))?;

    for track in &tracks_list {
        let raw = match track.artist.as_deref() {
            Some(s) if !s.trim().is_empty() => s,
            _ => continue,
        };

        let names = crate::service::library::artist_split::resolve(raw, &[], &rules);
        if names.is_empty() {
            continue;
        }
        if names.len() > 1 {
            report.tracks_multi += 1;
        }

        let mut first: Option<String> = None;

        for name in &names {
            let key = normalize_name(name);
            if key.is_empty() {
                continue;
            }

            // Créé comme le ferait l'indexation s'il manque.
            let artist_id = match known_items.get(&key) {
                Some(id) => id.clone(),
                None => {
                    let created = ArtistRepository::insert_artist(
                        &mut *tx,
                        ArtistCreate {
                            name: name.clone(),
                            name_normalized: key.clone(),
                            sort_name: normalize_sort_name(name),
                        },
                    )
                    .await
                    .map_err(|e| format!("Création de l'artiste « {name} » : {e}"))?;
                    report.artists_created += 1;
                    known_items.insert(key.clone(), created.id.clone());
                    created.id
                }
            };

            // Sans ça la liste des artistes ne le montre pas : elle passe par
            // `library_artists`.
            LibraryArtistRepository::insert_library_artist(
                &mut *tx,
                LibraryArtistCreate {
                    library_id: track.library_id,
                    artist_id: artist_id.clone(),
                },
            )
            .await
            .map_err(|e| format!("Rattachement de « {name} » à la bibliothèque : {e}"))?;

            // `changes()` dirait « 1 » même sur un upsert qui n'ajoute rien.
            let already_linked = LibraryTrackArtistRepository::exists(&mut *tx, &track.id, &artist_id)
            .await
            .map_err(|e| format!("Vérification de la liaison : {e}"))?;

            LibraryTrackArtistRepository::insert_library_track_artist(
                &mut *tx,
                LibraryTrackArtistCreate {
                    library_id: track.library_id,
                    artist_id: artist_id.clone(),
                    library_track_id: track.id.clone(),
                },
            )
            .await
            .map_err(|e| format!("Liaison piste ↔ artiste : {e}"))?;

            if !already_linked {
                report.links_created += 1;
            }
            if first.is_none() {
                first = Some(artist_id.clone());
            }
        }

        // L'artiste d'album écrasait celui de la piste : une compilation taguée
        // « Various Artists » donnait ce nom à chacun de ses morceaux. Le tag
        // fait foi.
        if let Some(expected) = first {
            if track.artist_id.as_deref() != Some(expected.as_str()) {
                LibraryTrackRepository::set_artist(&mut *tx, &track.id, &expected)
                    .await
                    .map_err(|e| format!("Correction de l'artiste : {e}"))?;
                report.main_artist_fixed += 1;
            }
        }
    }

    tx.commit()
        .await
        .map_err(|e| format!("Validation de la transaction : {e}"))?;

    log::info!(
        "🔗 Liaisons d'artistes reprises : {} pistes, dont {} multi-artistes — \
         {} liaison(s), {} fiche(s) d'artiste, {} artiste(s) principal(aux) corrigé(s)",
        report.tracks_seen,
        report.tracks_multi,
        report.links_created,
        report.artists_created,
        report.main_artist_fixed
    );

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

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

    /// Une piste avec son tag brut, sans aucune liaison — l'état d'avant.
    async fn insert_track(pool: &SqlitePool, id: &str, tag: &str) {
        insert_track_with(pool, id, tag, None).await;
    }

    /// Variante qui fixe l'artiste déjà enregistré sur la piste.
    async fn insert_track_with(pool: &SqlitePool, id: &str, tag: &str, artist_id: Option<&str>) {
        sqlx::query("INSERT INTO library_cache (path, artist) VALUES (?, ?)")
            .bind(format!("/{id}.flac"))
            .bind(tag)
            .execute(pool).await.expect("cache");
        let cache_id: i64 = sqlx::query_scalar("SELECT id FROM library_cache WHERE path = ?")
            .bind(format!("/{id}.flac"))
            .fetch_one(pool).await.expect("id de cache");
        sqlx::query("INSERT INTO library_files (id, library_id, path, filename, extension, size, status) VALUES (?, 1, ?, 'f', 'flac', 1, 'indexed')")
            .bind(format!("f-{id}"))
            .bind(format!("/{id}.flac"))
            .execute(pool).await.expect("fichier");
        sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, cache_id, artist_id, title, title_normalized) VALUES (?, 1, ?, ?, ?, 'T', 't')")
            .bind(id)
            .bind(format!("f-{id}"))
            .bind(cache_id)
            .bind(artist_id)
            .execute(pool).await.expect("piste");
    }

    #[tokio::test]
    async fn a_multi_artist_tag_gives_as_many_links() {
        let pool = base().await;
        insert_track(&pool, "t1", "Alan Walker;K-391;Sofia Carson").await;

        let r = repair(&pool, Some(1)).await.expect("reprise");

        assert_eq!(r.tracks_multi, 1);
        assert_eq!(r.links_created, 3);
        assert_eq!(r.artists_created, 3);

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM library_track_artists WHERE library_track_id = 't1'",
        )
        .fetch_one(&pool).await.expect("comptage");
        assert_eq!(total, 3);
    }

    #[tokio::test]
    async fn featuring_is_recognised() {
        let pool = base().await;
        insert_track(&pool, "t2", "Adele Feat. Darius Rucker").await;

        let r = repair(&pool, Some(1)).await.expect("reprise");

        assert_eq!(r.links_created, 2);
        let names: Vec<String> = sqlx::query_scalar(
            "SELECT a.name FROM library_track_artists lta
             JOIN artists a ON a.id = lta.artist_id
             WHERE lta.library_track_id = 't2' ORDER BY a.name",
        )
        .fetch_all(&pool).await.expect("noms");
        assert_eq!(names, vec!["Adele".to_string(), "Darius Rucker".to_string()]);
    }

    /// Le bouton doit pouvoir être appuyé deux fois sans y penser.
    #[tokio::test]
    async fn rerunning_creates_no_duplicate() {
        let pool = base().await;
        insert_track(&pool, "t3", "Daft Punk;Pharrell Williams").await;

        let first = repair(&pool, Some(1)).await.expect("première reprise");
        let second = repair(&pool, Some(1)).await.expect("seconde reprise");

        assert_eq!(first.links_created, 2);
        assert_eq!(second.links_created, 0, "rien de neuf à la seconde passe");
        assert_eq!(second.artists_created, 0);

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM library_track_artists")
            .fetch_one(&pool).await.expect("comptage");
        assert_eq!(total, 2);
    }

    /// « AC/DC » ne doit pas devenir deux artistes.
    #[tokio::test]
    async fn a_name_containing_a_slash_stays_whole() {
        let pool = base().await;
        insert_track(&pool, "t4", "AC/DC").await;

        let r = repair(&pool, Some(1)).await.expect("reprise");

        assert_eq!(r.tracks_multi, 0);
        assert_eq!(r.links_created, 1);
        let name: String = sqlx::query_scalar(
            "SELECT a.name FROM library_track_artists lta
             JOIN artists a ON a.id = lta.artist_id
             WHERE lta.library_track_id = 't4'",
        )
        .fetch_one(&pool).await.expect("nom");
        assert_eq!(name, "AC/DC");
    }

    /// Sans tag : ni échec, ni fiche inventée.
    #[tokio::test]
    async fn a_track_without_artist_is_ignored() {
        let pool = base().await;
        insert_track(&pool, "t5", "").await;

        let r = repair(&pool, Some(1)).await.expect("reprise");

        assert_eq!(r.tracks_seen, 1);
        assert_eq!(r.links_created, 0);
        assert_eq!(r.artists_created, 0);
    }

    /// L'artiste d'album écrasait celui de la piste : une compilation taguée
    /// « Various Artists » donnait ce nom à chacun de ses morceaux.
    #[tokio::test]
    async fn the_track_artist_wins_over_the_album_artist() {
        let pool = base().await;
        sqlx::query("INSERT INTO artists (id, name, name_normalized, sort_name) VALUES ('va', 'Various Artists', 'various artists', 'Various Artists')")
            .execute(&pool).await.expect("artiste d'album");

        // Le tag nomme PJ Harvey, mais la piste porte « Various Artists ».
        insert_track_with(&pool, "t6", "PJ Harvey", Some("va")).await;

        let r = repair(&pool, Some(1)).await.expect("reprise");
        assert_eq!(r.main_artist_fixed, 1);

        let name: String = sqlx::query_scalar(
            "SELECT a.name FROM library_tracks lt JOIN artists a ON a.id = lt.artist_id
             WHERE lt.id = 't6'",
        )
        .fetch_one(&pool).await.expect("nom");
        assert_eq!(name, "PJ Harvey");
    }

    /// Une piste déjà juste ne doit pas être comptée comme corrigée.
    #[tokio::test]
    async fn a_correct_track_is_left_alone() {
        let pool = base().await;
        insert_track(&pool, "t7", "Radiohead").await;

        let first = repair(&pool, Some(1)).await.expect("première");
        let second = repair(&pool, Some(1)).await.expect("seconde");

        assert_eq!(first.main_artist_fixed, 1, "l'artiste était absent, il se pose");
        assert_eq!(second.main_artist_fixed, 0, "rien à corriger la seconde fois");
    }
}
