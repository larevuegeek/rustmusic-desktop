//! Compilations éclatées par le serveur : sans tag « artiste de l'album », Navidrome ou Jellyfin
//! font un album par artiste (« NRJ Hit Music Only » en 50 fiches). On réunit les fiches distantes
//! de même titre et même année dont les pistes se complètent, comme l'indexation locale le fait par dossier.
//! Les chemins des morceaux ne bougent pas : J'aime, playlists et récents restent intacts.

use std::collections::{HashMap, HashSet};

use sqlx::SqlitePool;

use crate::helper::string::string::{normalize_name, split_artists};
use crate::repository::settings::settings_repository::SettingsRepository;
use crate::service::library::album_merge::{merge_into, recount_album, recount_library, resolve_artist, ARTISTES_DIVERS};

#[derive(sqlx::FromRow)]
struct Row {
    album_id: String,
    title_normalized: String,
    year: Option<i64>,
    cover_url: Option<String>,
    disc: Option<i64>,
    track: Option<i64>,
    tag_album: Option<String>,
}

/// Une fiche d'album distante et ses pistes.
#[derive(Debug, Clone, Default)]
struct Piece {
    id: String,
    title: String,
    year: Option<i64>,
    cover: bool,
    /// (disque, piste) ; `None` : piste sans numéro.
    numbers: Vec<Option<(i64, i64)>>,
    /// Premier artiste du tag « artiste de l'album » de chaque piste.
    tags: Vec<String>,
}

/// Les groupes à réunir : même titre, même année connue, toutes les pistes numérotées, et soit aucun numéro
/// en double, soit la forme d'un coffret sans numéro de disque (beaucoup de petites fiches, chacune sans doublon).
/// Deux albums différents au même titre (« Greatest Hits ») ont chacun leur piste 1 : ils restent séparés.
fn plan(pieces: Vec<Piece>) -> Vec<Vec<Piece>> {
    let vague: HashSet<String> = ["", "unknown", "unknown album"].iter().map(|s| normalize_name(s)).collect();
    let mut groups: HashMap<(String, i64), Vec<Piece>> = HashMap::new();
    for p in pieces {
        let Some(year) = p.year.filter(|y| *y > 0) else { continue };
        if vague.contains(&p.title) {
            continue;
        }
        groups.entry((p.title.clone(), year)).or_default().push(p);
    }
    groups
        .into_values()
        .filter(|g| g.len() > 1)
        .filter(|g| {
            if !g.iter().flat_map(|p| &p.numbers).all(Option::is_some) {
                return false;
            }
            let mut seen = HashSet::new();
            g.iter().flat_map(|p| &p.numbers).all(|n| seen.insert(*n)) || multi_disc_compilation(g)
        })
        .collect()
}

/// Coffret de compilation sans numéro de disque : la piste 1 revient une fois par disque.
/// Au moins 4 fiches de 1 à 4 pistes chacune, sans doublon dans une même fiche.
fn multi_disc_compilation(group: &[Piece]) -> bool {
    group.len() >= 4
        && group.iter().all(|p| {
            let mut seen = HashSet::new();
            p.numbers.len() <= 4 && p.numbers.iter().all(|n| seen.insert(*n))
        })
}

/// L'artiste qui porte l'album : celui de plus de la moitié des pistes, sinon aucun (compilation).
fn owner(group: &[Piece]) -> Option<String> {
    let total: usize = group.iter().map(|p| p.numbers.len()).sum();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for t in group.iter().flat_map(|p| &p.tags) {
        *counts.entry(t.as_str()).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(name, n)| (*n, std::cmp::Reverse(*name)))
        .filter(|(_, n)| n * 2 > total)
        .map(|(name, _)| name.to_string())
}

/// Réunit les compilations éclatées de la bibliothèque distante `library_id` et retire les fiches vides.
/// Rend le nombre d'albums réunis.
pub async fn regroup(pool: &SqlitePool, library_id: i64) -> Result<usize, String> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT b.id AS album_id, b.title_normalized, b.year, b.cover_url,
                t.disc_number AS disc, t.track_number AS track, lc.album_artist AS tag_album
         FROM library_albums b
         JOIN library_tracks t ON t.library_album_id = b.id
         LEFT JOIN library_cache lc ON lc.id = t.cache_id
         WHERE b.library_id = ? AND b.album_dir LIKE ? || '%'",
    )
    .bind(library_id)
    .bind(crate::core::remote::SCHEME)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Lecture des albums distants : {e}"))?;

    let mut pieces: HashMap<String, Piece> = HashMap::new();
    for r in rows {
        let p = pieces.entry(r.album_id.clone()).or_insert_with(|| Piece {
            id: r.album_id.clone(),
            title: r.title_normalized.clone(),
            year: r.year,
            cover: r.cover_url.is_some(),
            ..Default::default()
        });
        p.numbers.push(r.track.map(|t| (r.disc.unwrap_or(1), t)));
        if let Some(first) = r.tag_album.as_deref().and_then(|t| split_artists(t).into_iter().next()) {
            p.tags.push(first);
        }
    }
    let groups = plan(pieces.into_values().collect());

    let mut tx = pool.begin().await.map_err(|e| format!("Ouverture de la transaction : {e}"))?;
    let mut known = if groups.is_empty() { HashMap::new() } else { known_by_norm(&mut tx).await? };
    for mut group in groups.iter().cloned() {
        // La fiche qui a une pochette, puis la mieux fournie.
        group.sort_by(|a, b| b.cover.cmp(&a.cover).then(b.numbers.len().cmp(&a.numbers.len())).then(a.id.cmp(&b.id)));
        let keep = group[0].id.clone();
        for p in &group[1..] {
            merge_into(&mut tx, &keep, &p.id).await?;
        }
        let (artist, kind) = match owner(&group) {
            Some(name) => (name, "album"),
            None => (ARTISTES_DIVERS.to_string(), "compilation"),
        };
        let artist_id = resolve_artist(&mut tx, &mut known, &artist, library_id).await?;
        sqlx::query("UPDATE library_albums SET artist_id = ?, album_type = ? WHERE id = ?")
            .bind(&artist_id)
            .bind(kind)
            .bind(&keep)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Propriétaire de l'album : {e}"))?;
        recount_album(&mut tx, &keep).await?;
    }

    // Une fiche distante sans piste (morceau réenregistré après une réunion) ne doit pas s'afficher.
    sqlx::query(
        "DELETE FROM library_albums WHERE library_id = ? AND album_dir LIKE ? || '%'
           AND NOT EXISTS (SELECT 1 FROM library_tracks t WHERE t.library_album_id = library_albums.id)",
    )
    .bind(library_id)
    .bind(crate::core::remote::SCHEME)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Fiches vides : {e}"))?;

    // Toujours : les synchros changent des artistes sans que les déclencheurs suivent.
    recount_library(&mut tx, library_id).await?;
    tx.commit().await.map_err(|e| format!("Validation : {e}"))?;
    if !groups.is_empty() {
        log::info!("💿 {} album(s) distant(s) réuni(s) (bibliothèque {library_id})", groups.len());
    }
    Ok(groups.len())
}

/// Au lancement, une seule fois : les bibliothèques distantes déjà synchronisées avant ce regroupement.
pub async fn regroup_once(pool: &SqlitePool) {
    const DONE_KEY: &str = "remote_albums_regrouped";
    if SettingsRepository::get(pool, DONE_KEY).await.ok().flatten().as_deref() == Some("1") {
        return;
    }
    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM library WHERE server_id IS NOT NULL").fetch_all(pool).await.unwrap_or_default();
    let mut ok = true;
    for id in ids {
        if let Err(e) = regroup(pool, id).await {
            log::warn!("💿 Albums distants non réunis (bibliothèque {id}) : {e}");
            ok = false;
        }
    }
    if ok {
        let _ = SettingsRepository::set(pool, DONE_KEY, "1").await;
    }
}

/// Artistes connus, par nom normalisé (pour `resolve_artist`).
async fn known_by_norm(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<HashMap<String, String>, String> {
    Ok(sqlx::query_as::<_, (String, String)>("SELECT name_normalized, id FROM artists")
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| format!("Lecture des artistes : {e}"))?
        .into_iter()
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(id: &str, title: &str, year: i64, tracks: &[i64], tag: &str) -> Piece {
        Piece {
            id: id.into(),
            title: normalize_name(title),
            year: Some(year),
            cover: true,
            numbers: tracks.iter().map(|t| Some((1, *t))).collect(),
            tags: tracks.iter().map(|_| tag.to_string()).collect(),
        }
    }

    #[test]
    fn a_split_compilation_comes_together_as_various_artists() {
        let g = plan(vec![
            piece("a", "NRJ Hits 2019", 2019, &[1, 4], "Ava Max"),
            piece("b", "NRJ Hits 2019", 2019, &[2], "Gaullin"),
            piece("c", "NRJ Hits 2019", 2019, &[3, 5], "Redbone"),
        ]);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].len(), 3);
        assert_eq!(owner(&g[0]), None);
    }

    #[test]
    fn same_title_albums_of_different_artists_stay_apart() {
        // Chacun a sa piste 1 ; une autre année ne se mêle pas non plus.
        assert!(plan(vec![
            piece("a", "Greatest Hits", 2004, &[1, 2, 3], "Aerosmith"),
            piece("b", "Greatest Hits", 2004, &[1, 2], "The Police"),
            piece("c", "Greatest Hits", 2009, &[4], "Foo Fighters"),
        ])
        .is_empty());
    }

    #[test]
    fn unknown_titles_missing_years_or_numbers_are_left_alone() {
        assert!(plan(vec![piece("a", "Unknown Album", 2001, &[1], "X"), piece("b", "Unknown Album", 2001, &[2], "Y")]).is_empty());
        let mut sans_annee = piece("a", "Live", 2001, &[1], "X");
        sans_annee.year = None;
        assert!(plan(vec![sans_annee, piece("b", "Live", 2001, &[2], "Y")]).is_empty());
        let mut sans_numero = piece("a", "Live", 2001, &[1], "X");
        sans_numero.numbers.push(None);
        sans_numero.tags.push("X".into());
        assert!(plan(vec![sans_numero, piece("b", "Live", 2001, &[2], "Y")]).is_empty());
    }

    #[test]
    fn a_box_set_without_disc_numbers_comes_together() {
        // Trois disques, piste 1 trois fois ; chaque artiste n'a que quelques titres.
        let g = plan(vec![
            piece("a", "NRJ 2019", 2019, &[1, 5], "Ava Max"),
            piece("b", "NRJ 2019", 2019, &[1], "Ariana Grande"),
            piece("c", "NRJ 2019", 2019, &[1, 2], "David Guetta"),
            piece("d", "NRJ 2019", 2019, &[2, 3], "Dua Lipa"),
            piece("e", "NRJ 2019", 2019, &[3], "Angèle"),
        ]);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].len(), 5);
    }

    #[test]
    fn several_real_albums_with_the_same_title_and_year_stay_apart() {
        let full: Vec<i64> = (1..=12).collect();
        assert!(plan(vec![
            piece("a", "Greatest Hits", 2004, &full, "A"),
            piece("b", "Greatest Hits", 2004, &full, "B"),
            piece("c", "Greatest Hits", 2004, &full, "C"),
            piece("d", "Greatest Hits", 2004, &full, "D"),
        ])
        .is_empty());
    }

    #[test]
    fn a_featuring_keeps_the_main_artist_as_owner() {
        let g = plan(vec![
            piece("a", "Songs for the Deaf", 2002, &[1, 2, 3, 5, 6], "Queens of the Stone Age"),
            piece("b", "Songs for the Deaf", 2002, &[4], "Queens of the Stone Age (H)"),
        ]);
        assert_eq!(owner(&g[0]).as_deref(), Some("Queens of the Stone Age"));
    }
}
