//! Propriétaire des albums après un scan.
//!
//! Pendant un scan, une piste dont l'artiste a changé (tag corrigé, ID3v2 lu à la place
//! de l'ID3v1) rejoint un album encore rangé sous l'ancien nom : deux artistes et aucun
//! tag « artiste de l'album », l'album passe sous « Various Artists ». À ce moment, on ne
//! sait pas si les autres pistes suivront ; à la fin du scan, si. Un album dont toutes
//! les pistes ont de nouveau le même artiste lui revient.

use sqlx::SqlitePool;

use crate::repository::library::library_album_repository::LibraryAlbumRepository;

/// Rend chaque album à l'artiste commun de ses pistes ; rend le nombre d'albums rendus.
pub async fn restore_album_owners(pool: &SqlitePool, library_id: i64) -> usize {
    let albums = match LibraryAlbumRepository::owners_to_restore(pool, library_id).await {
        Ok(albums) => albums,
        Err(e) => {
            log::warn!("💿 Propriétaires d'albums non vérifiés : {e}");
            return 0;
        }
    };
    let mut restored = 0;
    for (album, artist) in albums {
        // Refusé si l'artiste a déjà un album du même titre (index de compatibilité 0.2.4).
        match LibraryAlbumRepository::set_owner(pool, &album, &artist, "album").await {
            Ok(()) => restored += 1,
            Err(e) => log::warn!("💿 Album {album} non rendu à son artiste : {e}"),
        }
    }
    if restored > 0 {
        log::info!("💿 {restored} album(s) rendu(s) à l'artiste de leurs pistes");
    }
    restored
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn base() -> SqlitePool {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.expect("base mémoire");
        sqlx::migrate!("./SQL/").run(&pool).await.expect("migrations");
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO library (id, profil_id, name) VALUES (1, 1, 'test')").execute(&pool).await.unwrap();
        for (id, name) in [("old", "?ông Nhi"), ("new", "Đông Nhi"), ("va", "Various Artists"), ("other", "Hồ Ngọc Hà")] {
            sqlx::query("INSERT INTO artists (id, name, name_normalized, sort_name) VALUES (?, ?, ?, ?)")
                .bind(id).bind(name).bind(name.to_lowercase()).bind(name).execute(&pool).await.unwrap();
        }
        pool
    }

    async fn album(pool: &SqlitePool, id: &str, owner: &str, kind: &str) {
        sqlx::query("INSERT INTO library_albums (id, library_id, artist_id, title, title_normalized, album_type) VALUES (?, 1, ?, ?, ?, ?)")
            .bind(id).bind(owner).bind(id).bind(id).bind(kind).execute(pool).await.unwrap();
    }

    async fn track(pool: &SqlitePool, id: &str, album: &str, artist: &str, album_artist: Option<&str>) {
        let path = format!("/{id}.mp3");
        let cache = sqlx::query("INSERT INTO library_cache (path, album_artist) VALUES (?, ?)")
            .bind(&path).bind(album_artist).execute(pool).await.unwrap().last_insert_rowid();
        sqlx::query("INSERT INTO library_files (id, library_id, path, filename, extension, size, status) VALUES (?, 1, ?, 'f', 'mp3', 1, 'indexed')")
            .bind(format!("f-{id}")).bind(&path).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, cache_id, library_album_id, artist_id, title, title_normalized) VALUES (?, 1, ?, ?, ?, ?, 'T', 't')")
            .bind(id).bind(format!("f-{id}")).bind(cache).bind(album).bind(artist).execute(pool).await.unwrap();
    }

    async fn owner(pool: &SqlitePool, album: &str) -> (String, String) {
        sqlx::query_as("SELECT artist_id, album_type FROM library_albums WHERE id = ?").bind(album).fetch_one(pool).await.unwrap()
    }

    #[tokio::test]
    async fn an_album_whose_tracks_were_renamed_returns_to_their_artist() {
        let pool = base().await;
        // Passé sous « Various Artists » pendant le rescan, ou resté sous l'ancien nom.
        album(&pool, "khoc", "va", "compilation").await;
        track(&pool, "k1", "khoc", "new", None).await;
        track(&pool, "k2", "khoc", "new", None).await;
        album(&pool, "first-step", "old", "album").await;
        track(&pool, "f1", "first-step", "new", None).await;

        assert_eq!(restore_album_owners(&pool, 1).await, 2);
        assert_eq!(owner(&pool, "khoc").await, ("new".into(), "album".into()));
        assert_eq!(owner(&pool, "first-step").await, ("new".into(), "album".into()));
        assert_eq!(restore_album_owners(&pool, 1).await, 0, "rien à refaire");
    }

    #[tokio::test]
    async fn real_compilations_and_tagged_albums_are_left_alone() {
        let pool = base().await;
        // Deux artistes : une vraie compilation.
        album(&pool, "hits", "va", "compilation").await;
        track(&pool, "h1", "hits", "new", None).await;
        track(&pool, "h2", "hits", "other", None).await;
        // Un tag « artiste de l'album » décide du propriétaire.
        album(&pool, "duo", "other", "album").await;
        track(&pool, "d1", "duo", "new", Some("Hồ Ngọc Hà")).await;

        assert_eq!(restore_album_owners(&pool, 1).await, 0);
        assert_eq!(owner(&pool, "hits").await, ("va".into(), "compilation".into()));
        assert_eq!(owner(&pool, "duo").await, ("other".into(), "album".into()));
    }
}
