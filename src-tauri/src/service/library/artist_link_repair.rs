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
use crate::helper::string::string::{normalize_name, normalize_sort_name, split_artists};
use crate::repository::artist::artist_repository::ArtistRepository;
use crate::repository::library::library_artist_repository::LibraryArtistRepository;
use crate::repository::library::library_track_artist_repository::LibraryTrackArtistRepository;

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

#[derive(sqlx::FromRow)]
struct PisteATraiter {
    id: String,
    library_id: i64,
    artist_id: Option<String>,
    artist: Option<String>,
}

/// Toutes les bibliothèques si `library_id` est `None`. Idempotent : upsert sur
/// `(library_track_id, artist_id)`.
pub async fn repair(
    pool: &SqlitePool,
    library_id: Option<i64>,
) -> Result<ArtistLinkReport, String> {
    let pistes: Vec<PisteATraiter> = sqlx::query_as::<_, PisteATraiter>(
        r#"
        SELECT lt.id AS id, lt.library_id AS library_id,
               lt.artist_id AS artist_id, lc.artist AS artist
        FROM library_tracks lt
        LEFT JOIN library_cache lc ON lc.id = lt.cache_id
        WHERE (?1 IS NULL OR lt.library_id = ?1)
        "#,
    )
    .bind(library_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Lecture des pistes : {e}"))?;

    let mut rapport = ArtistLinkReport {
        tracks_seen: pistes.len() as i64,
        ..Default::default()
    };

    // Cache par nom normalisé, la clé de l'index unique de `artists` : évite un
    // aller-retour par piste.
    let mut connus: HashMap<String, String> = sqlx::query_as::<_, (String, String)>(
        "SELECT name_normalized, id FROM artists",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Lecture des artistes : {e}"))?
    .into_iter()
    .collect();

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Ouverture de la transaction : {e}"))?;

    for piste in &pistes {
        let brut = match piste.artist.as_deref() {
            Some(s) if !s.trim().is_empty() => s,
            _ => continue,
        };

        let noms = split_artists(brut);
        if noms.is_empty() {
            continue;
        }
        if noms.len() > 1 {
            rapport.tracks_multi += 1;
        }

        let mut premier: Option<String> = None;

        for nom in &noms {
            let cle = normalize_name(nom);
            if cle.is_empty() {
                continue;
            }

            // Créé comme le ferait l'indexation s'il manque.
            let artist_id = match connus.get(&cle) {
                Some(id) => id.clone(),
                None => {
                    let cree = ArtistRepository::insert_artist(
                        &mut *tx,
                        ArtistCreate {
                            name: nom.clone(),
                            name_normalized: cle.clone(),
                            sort_name: normalize_sort_name(nom),
                        },
                    )
                    .await
                    .map_err(|e| format!("Création de l'artiste « {nom} » : {e}"))?;
                    rapport.artists_created += 1;
                    connus.insert(cle.clone(), cree.id.clone());
                    cree.id
                }
            };

            // Sans ça la liste des artistes ne le montre pas : elle passe par
            // `library_artists`.
            LibraryArtistRepository::insert_library_artist(
                &mut *tx,
                LibraryArtistCreate {
                    library_id: piste.library_id,
                    artist_id: artist_id.clone(),
                },
            )
            .await
            .map_err(|e| format!("Rattachement de « {nom} » à la bibliothèque : {e}"))?;

            // `changes()` dirait « 1 » même sur un upsert qui n'ajoute rien.
            let avant: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM library_track_artists
                 WHERE library_track_id = ? AND artist_id = ?",
            )
            .bind(&piste.id)
            .bind(&artist_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| format!("Vérification de la liaison : {e}"))?;

            LibraryTrackArtistRepository::insert_library_track_artist(
                &mut *tx,
                LibraryTrackArtistCreate {
                    library_id: piste.library_id,
                    artist_id: artist_id.clone(),
                    library_track_id: piste.id.clone(),
                },
            )
            .await
            .map_err(|e| format!("Liaison piste ↔ artiste : {e}"))?;

            if avant == 0 {
                rapport.links_created += 1;
            }
            if premier.is_none() {
                premier = Some(artist_id.clone());
            }
        }

        // L'artiste d'album écrasait celui de la piste : une compilation taguée
        // « Various Artists » donnait ce nom à chacun de ses morceaux. Le tag
        // fait foi.
        if let Some(attendu) = premier {
            if piste.artist_id.as_deref() != Some(attendu.as_str()) {
                sqlx::query("UPDATE library_tracks SET artist_id = ? WHERE id = ?")
                    .bind(&attendu)
                    .bind(&piste.id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Correction de l'artiste : {e}"))?;
                rapport.main_artist_fixed += 1;
            }
        }
    }

    tx.commit()
        .await
        .map_err(|e| format!("Validation de la transaction : {e}"))?;

    log::info!(
        "🔗 Liaisons d'artistes reprises : {} pistes, dont {} multi-artistes — \
         {} liaison(s), {} fiche(s) d'artiste, {} artiste(s) principal(aux) corrigé(s)",
        rapport.tracks_seen,
        rapport.tracks_multi,
        rapport.links_created,
        rapport.artists_created,
        rapport.main_artist_fixed
    );

    Ok(rapport)
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
    async fn poser_piste(pool: &SqlitePool, id: &str, tag: &str) {
        poser_piste_avec(pool, id, tag, None).await;
    }

    /// Variante qui fixe l'artiste déjà enregistré sur la piste.
    async fn poser_piste_avec(pool: &SqlitePool, id: &str, tag: &str, artist_id: Option<&str>) {
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
    async fn un_tag_a_plusieurs_artistes_donne_autant_de_liaisons() {
        let pool = base().await;
        poser_piste(&pool, "t1", "Alan Walker;K-391;Sofia Carson").await;

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
    async fn le_featuring_est_reconnu() {
        let pool = base().await;
        poser_piste(&pool, "t2", "Adele Feat. Darius Rucker").await;

        let r = repair(&pool, Some(1)).await.expect("reprise");

        assert_eq!(r.links_created, 2);
        let noms: Vec<String> = sqlx::query_scalar(
            "SELECT a.name FROM library_track_artists lta
             JOIN artists a ON a.id = lta.artist_id
             WHERE lta.library_track_id = 't2' ORDER BY a.name",
        )
        .fetch_all(&pool).await.expect("noms");
        assert_eq!(noms, vec!["Adele".to_string(), "Darius Rucker".to_string()]);
    }

    /// Le bouton doit pouvoir être appuyé deux fois sans y penser.
    #[tokio::test]
    async fn relancer_ne_cree_pas_de_doublon() {
        let pool = base().await;
        poser_piste(&pool, "t3", "Daft Punk;Pharrell Williams").await;

        let premier = repair(&pool, Some(1)).await.expect("première reprise");
        let second = repair(&pool, Some(1)).await.expect("seconde reprise");

        assert_eq!(premier.links_created, 2);
        assert_eq!(second.links_created, 0, "rien de neuf à la seconde passe");
        assert_eq!(second.artists_created, 0);

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM library_track_artists")
            .fetch_one(&pool).await.expect("comptage");
        assert_eq!(total, 2);
    }

    /// « AC/DC » ne doit pas devenir deux artistes.
    #[tokio::test]
    async fn un_nom_qui_contient_une_barre_reste_entier() {
        let pool = base().await;
        poser_piste(&pool, "t4", "AC/DC").await;

        let r = repair(&pool, Some(1)).await.expect("reprise");

        assert_eq!(r.tracks_multi, 0);
        assert_eq!(r.links_created, 1);
        let nom: String = sqlx::query_scalar(
            "SELECT a.name FROM library_track_artists lta
             JOIN artists a ON a.id = lta.artist_id
             WHERE lta.library_track_id = 't4'",
        )
        .fetch_one(&pool).await.expect("nom");
        assert_eq!(nom, "AC/DC");
    }

    /// Sans tag : ni échec, ni fiche inventée.
    #[tokio::test]
    async fn une_piste_sans_artiste_est_ignoree() {
        let pool = base().await;
        poser_piste(&pool, "t5", "").await;

        let r = repair(&pool, Some(1)).await.expect("reprise");

        assert_eq!(r.tracks_seen, 1);
        assert_eq!(r.links_created, 0);
        assert_eq!(r.artists_created, 0);
    }

    /// L'artiste d'album écrasait celui de la piste : une compilation taguée
    /// « Various Artists » donnait ce nom à chacun de ses morceaux.
    #[tokio::test]
    async fn l_artiste_de_la_piste_l_emporte_sur_celui_de_l_album() {
        let pool = base().await;
        sqlx::query("INSERT INTO artists (id, name, name_normalized, sort_name) VALUES ('va', 'Various Artists', 'various artists', 'Various Artists')")
            .execute(&pool).await.expect("artiste d'album");

        // Le tag nomme PJ Harvey, mais la piste porte « Various Artists ».
        poser_piste_avec(&pool, "t6", "PJ Harvey", Some("va")).await;

        let r = repair(&pool, Some(1)).await.expect("reprise");
        assert_eq!(r.main_artist_fixed, 1);

        let nom: String = sqlx::query_scalar(
            "SELECT a.name FROM library_tracks lt JOIN artists a ON a.id = lt.artist_id
             WHERE lt.id = 't6'",
        )
        .fetch_one(&pool).await.expect("nom");
        assert_eq!(nom, "PJ Harvey");
    }

    /// Une piste déjà juste ne doit pas être comptée comme corrigée.
    #[tokio::test]
    async fn une_piste_juste_n_est_pas_touchee() {
        let pool = base().await;
        poser_piste(&pool, "t7", "Radiohead").await;

        let premier = repair(&pool, Some(1)).await.expect("première");
        let second = repair(&pool, Some(1)).await.expect("seconde");

        assert_eq!(premier.main_artist_fixed, 1, "l'artiste était absent, il se pose");
        assert_eq!(second.main_artist_fixed, 0, "rien à corriger la seconde fois");
    }
}
