//! Ce que l'on fait dans RustMusic, renvoyé au serveur (favoris, notes, écoutes, « en cours »),
//! et l'inverse à la synchronisation. Serveur injoignable : l'action attend dans `server_outbox`.

use sqlx::SqlitePool;

use crate::core::subsonic::catalog::RemoteSong;
use crate::core::remote::Backend;
use crate::core::subsonic::client::{is_unreachable, Server};
use crate::repository::library::library_track_repository::LibraryTrackRepository;
use crate::repository::playlist::playlist_item_repository::PlaylistItemRepository;
use crate::repository::playlist::playlist_repository::PlaylistRepository;
use crate::repository::playlist::track_liked_repository::TrackLikedRepository;
use crate::repository::server::server_outbox_repository::ServerOutboxRepository;
use crate::repository::server::server_repository::ServerRepository;
use crate::service::external_server::sync_service::song_path;
use crate::service::external_server::track_service::parse;

#[derive(Debug, Clone, PartialEq)]
pub enum Feedback {
    /// Favori posé (`true`) ou retiré.
    Star(bool),
    /// Note de 0 (aucune) à 5.
    Rating(u8),
    /// Écoute comptée, à l'instant donné (millisecondes).
    Scrobble(i64),
    /// Lecture qui commence : jamais mise en attente, elle n'a de sens que maintenant.
    NowPlaying,
    /// Contenu de la playlist locale `id` (playlist du serveur), envoyé en entier.
    PlaylistSongs(i64),
    /// Nom et description de la playlist locale `id`.
    PlaylistInfo(i64),
    /// Playlist du serveur supprimée dans RustMusic.
    PlaylistDelete,
}

impl Feedback {
    fn action(&self) -> &'static str {
        match self {
            Feedback::Star(true) => "star",
            Feedback::Star(false) => "unstar",
            Feedback::Rating(_) => "rating",
            Feedback::Scrobble(_) => "scrobble",
            Feedback::NowPlaying => "now_playing",
            Feedback::PlaylistSongs(_) => "playlist_songs",
            Feedback::PlaylistInfo(_) => "playlist_info",
            Feedback::PlaylistDelete => "playlist_delete",
        }
    }

    fn value(&self) -> Option<i64> {
        match self {
            Feedback::Rating(r) => Some(*r as i64),
            Feedback::Scrobble(at) => Some(*at),
            Feedback::PlaylistSongs(id) | Feedback::PlaylistInfo(id) => Some(*id),
            _ => None,
        }
    }

    fn from_row(action: &str, value: Option<i64>) -> Option<Self> {
        Some(match action {
            "star" => Feedback::Star(true),
            "unstar" => Feedback::Star(false),
            "rating" => Feedback::Rating(value?.clamp(0, 5) as u8),
            "scrobble" => Feedback::Scrobble(value?),
            "playlist_songs" => Feedback::PlaylistSongs(value?),
            "playlist_info" => Feedback::PlaylistInfo(value?),
            "playlist_delete" => Feedback::PlaylistDelete,
            _ => return None,
        })
    }

    /// Actions en attente qu'une plus récente sur le même morceau remplace.
    fn supersedes(&self) -> &'static [&'static str] {
        match self {
            Feedback::Star(_) => &["star", "unstar"],
            Feedback::Rating(_) => &["rating"],
            Feedback::PlaylistSongs(_) => &["playlist_songs"],
            Feedback::PlaylistInfo(_) => &["playlist_info"],
            Feedback::PlaylistDelete => &["playlist_songs", "playlist_info"],
            _ => &[],
        }
    }

    fn request(&self, song_id: &str) -> (&'static str, Vec<(&'static str, String)>) {
        let id = ("id", song_id.to_string());
        match self {
            Feedback::Star(true) => ("star", vec![id]),
            Feedback::Star(false) => ("unstar", vec![id]),
            Feedback::Rating(r) => ("setRating", vec![id, ("rating", r.to_string())]),
            Feedback::Scrobble(at) => ("scrobble", vec![id, ("time", at.to_string()), ("submission", "true".into())]),
            Feedback::NowPlaying => ("scrobble", vec![id, ("submission", "false".into())]),
            Feedback::PlaylistDelete => ("deletePlaylist", vec![id]),
            // Construits par `deliver` depuis la base : jamais demandés ici.
            Feedback::PlaylistSongs(_) | Feedback::PlaylistInfo(_) => ("ping", vec![]),
        }
    }
}

/// Note locale (0,5 à 5 par demis) → note du serveur (entier, 0 = aucune).
pub fn server_rating(rating: Option<f64>) -> u8 {
    rating.map_or(0, |r| r.round().clamp(0.0, 5.0) as u8)
}

/// Morceaux de la playlist locale, dans l'ordre, en identifiants du serveur ; ceux du serveur
/// absents de RustMusic suivent. `None` : la playlist n'existe plus.
pub(crate) async fn playlist_song_ids(pool: &SqlitePool, playlist_id: i64) -> Result<Option<Vec<String>>, String> {
    let row = PlaylistRepository::server_and_extra(pool, playlist_id).await.map_err(|e| e.to_string())?;
    let Some((Some(server_id), extra)) = row else { return Ok(None) };
    let paths = PlaylistItemRepository::track_paths(pool, playlist_id).await.map_err(|e| e.to_string())?;
    let mut ids: Vec<String> = paths
        .iter()
        .filter_map(|p| parse(p).filter(|(s, _)| *s == server_id).map(|(_, id)| id.to_string()))
        .collect();
    ids.extend(extra.and_then(|j| serde_json::from_str::<Vec<String>>(&j).ok()).unwrap_or_default());
    Ok(Some(ids))
}

/// `target` : le morceau, ou la playlist côté serveur.
async fn deliver(pool: &SqlitePool, server: &Backend, target: &str, feedback: &Feedback) -> Result<(), String> {
    match server {
        Backend::Subsonic(s) => deliver_subsonic(pool, s, target, feedback).await,
        // Jellyfin : favoris, écoutes, « en cours », playlists ; pas de notes de 1 à 5.
        Backend::Jellyfin(j) => match feedback {
            Feedback::Star(on) => j.set_favorite(target, *on).await,
            Feedback::Scrobble(at) => j.mark_played(target, *at).await,
            Feedback::NowPlaying => j.now_playing(target).await,
            Feedback::PlaylistSongs(local) => {
                let Some(ids) = playlist_song_ids(pool, *local).await? else { return Ok(()) };
                j.update_playlist(target, None, Some(&ids)).await
            }
            Feedback::PlaylistInfo(local) => {
                match PlaylistRepository::find_by_id(pool, *local).await.map_err(|e| e.to_string())? {
                    Some(p) => j.update_playlist(target, Some(&p.name), None).await,
                    None => Ok(()),
                }
            }
            Feedback::PlaylistDelete => j.delete_item(target).await,
            Feedback::Rating(_) => Ok(()),
        },
        // DLNA : rien ne remonte au serveur.
        Backend::Dlna(_) => Ok(()),
    }
}

async fn deliver_subsonic(pool: &SqlitePool, server: &Server, target: &str, feedback: &Feedback) -> Result<(), String> {
    match feedback {
        Feedback::PlaylistSongs(local) => {
            let Some(ids) = playlist_song_ids(pool, *local).await? else { return Ok(()) };
            // `createPlaylist` avec son identifiant : le contenu est remplacé en entier.
            let mut params = vec![("playlistId", target.to_string())];
            params.extend(ids.into_iter().map(|id| ("songId", id)));
            call(server, "createPlaylist", params).await
        }
        Feedback::PlaylistInfo(local) => {
            let Some(p) = PlaylistRepository::find_by_id(pool, *local).await.map_err(|e| e.to_string())? else { return Ok(()) };
            let params = vec![("playlistId", target.to_string()), ("name", p.name), ("comment", p.description.unwrap_or_default())];
            call(server, "updatePlaylist", params).await
        }
        _ => {
            let (endpoint, params) = feedback.request(target);
            call(server, endpoint, params).await
        }
    }
}

async fn call(server: &Server, endpoint: &str, params: Vec<(&str, String)>) -> Result<(), String> {
    let params: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    server.call(endpoint, &params).await.map(|_| ())
}

async fn queue(pool: &SqlitePool, server_id: i64, song_id: &str, feedback: &Feedback) {
    let _ = ServerOutboxRepository::insert(pool, server_id, feedback.action(), song_id, feedback.value()).await;
}

/// Renvoie l'action au serveur du morceau `path` ; rien pour un morceau local.
pub async fn send(pool: &SqlitePool, path: &str, feedback: Feedback) {
    let Some((server_id, song_id)) = parse(path) else { return };
    send_to(pool, server_id, song_id, feedback).await;
}

/// Renvoie l'action au serveur `server_id`, sur `song_id` (morceau ou playlist côté serveur).
pub async fn send_to(pool: &SqlitePool, server_id: i64, song_id: &str, feedback: Feedback) {
    // La plus récente fait foi : une action en attente sur le même morceau n'a plus d'objet.
    for action in feedback.supersedes() {
        let _ = ServerOutboxRepository::delete_action(pool, server_id, song_id, action).await;
    }
    let enabled = ServerRepository::find_one(pool, server_id).await.ok().flatten().map(|s| s.enabled);
    let result = match enabled {
        None => return,
        Some(false) => Err("serveur injoignable : désactivé".to_string()),
        Some(true) => match ServerRepository::server(pool, server_id).await {
            Ok(server) => deliver(pool, &server, song_id, &feedback).await,
            Err(e) => Err(e),
        },
    };
    match result {
        Ok(()) => log::debug!("☁️  {} envoyé ({song_id})", feedback.action()),
        Err(e) if is_unreachable(&e) && feedback != Feedback::NowPlaying => {
            log::info!("☁️  {} mis en attente ({song_id}) : {e}", feedback.action());
            queue(pool, server_id, song_id, &feedback).await;
        }
        Err(e) => log::warn!("☁️  {} refusé par le serveur ({song_id}) : {e}", feedback.action()),
    }
}

/// Vide la boîte d'envoi du serveur, dans l'ordre ; s'arrête au premier échec de connexion.
pub async fn flush(pool: &SqlitePool, server_id: i64, server: &Backend) -> Result<usize, String> {
    let rows = ServerOutboxRepository::find_by_server(pool, server_id).await.map_err(|e| e.to_string())?;
    let mut sent = 0;
    for (id, action, song_id, value) in rows {
        if let Some(feedback) = Feedback::from_row(&action, value) {
            match deliver(pool, server, &song_id, &feedback).await {
                Ok(()) => sent += 1,
                Err(e) if is_unreachable(&e) => return Err(e),
                Err(e) => log::warn!("☁️  {action} en attente refusé ({song_id}) : {e}"),
            }
        }
        let _ = ServerOutboxRepository::delete(pool, id).await;
    }
    if sent > 0 {
        log::info!("☁️  {sent} action(s) en attente renvoyée(s) au serveur {server_id}");
    }
    Ok(sent)
}

/// Favoris et notes alignés sur le serveur, qui fait foi (sa boîte d'envoi vient d'être vidée).
/// Une note locale à la demi-étoile est gardée tant qu'elle s'arrondit à celle du serveur ;
/// `ratings` faux (Jellyfin, sans notes) : les notes locales restent telles quelles.
pub async fn apply_server_state(pool: &SqlitePool, server_id: i64, songs: &[RemoteSong], ratings_on_server: bool) -> Result<(u64, u64), sqlx::Error> {
    let Some(profil_id) = ServerRepository::profile_of(pool, server_id).await? else {
        return Ok((0, 0));
    };
    let (mut likes, mut ratings) = (0, 0);
    let mut tx = pool.begin().await?;
    for song in songs {
        let path = song_path(server_id, song);
        likes += if song.starred {
            TrackLikedRepository::like_from_server(&mut *tx, &path, profil_id, server_id).await?
        } else {
            TrackLikedRepository::remove_track_liked(&mut *tx, &path, profil_id).await?
        };

        if !ratings_on_server {
            continue;
        }
        ratings += LibraryTrackRepository::set_rating_from_server(&mut *tx, &path, server_rating(song.user_rating) as i64).await?;
    }
    tx.commit().await?;
    Ok((likes, ratings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn ratings_round_to_the_server_scale() {
        assert_eq!(server_rating(None), 0);
        assert_eq!(server_rating(Some(3.5)), 4);
        assert_eq!(server_rating(Some(2.4)), 2);
        assert_eq!(server_rating(Some(5.0)), 5);
    }

    #[test]
    fn requests_follow_the_subsonic_api() {
        assert_eq!(Feedback::Rating(4).request("s1"), ("setRating", vec![("id", "s1".into()), ("rating", "4".into())]));
        assert_eq!(Feedback::NowPlaying.request("s1").1[1], ("submission", "false".into()));
        assert_eq!(Feedback::from_row("unstar", None), Some(Feedback::Star(false)));
        assert_eq!(Feedback::from_row("now_playing", None), None);
    }

    #[tokio::test]
    async fn server_state_wins_but_half_stars_survive() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO media_servers (id, profil_id, name, url, username) VALUES (1, 1, 'NAS', 'http://nas', 'd')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO library (id, profil_id, name, server_id) VALUES (5, 1, 'L', 1)").execute(&pool).await.unwrap();
        let song = |id: &str, starred: bool, rating: Option<f64>| RemoteSong {
            id: id.into(), album_id: Some("al".into()), starred, user_rating: rating, ..Default::default()
        };
        // Local : a aimé et noté 3,5 ; b aimé, noté 2 ; c ni l'un ni l'autre.
        for (i, (id, liked, rating)) in [("a", true, Some(3.5)), ("b", true, Some(2.0)), ("c", false, None)].iter().enumerate() {
            let path = format!("remote://1/al/{id}");
            sqlx::query("INSERT INTO library_files (id, library_id, path, filename) VALUES (?, 5, ?, 'x')").bind(format!("f{i}")).bind(&path).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, title, title_normalized, rating) VALUES (?, 5, ?, 't', 't', ?)")
                .bind(format!("t{i}")).bind(format!("f{i}")).bind(rating).execute(&pool).await.unwrap();
            if *liked {
                sqlx::query("INSERT INTO track_liked (path, profil_id) VALUES (?, 1)").bind(&path).execute(&pool).await.unwrap();
            }
        }
        // Serveur : a étoilé et noté 4 (= 3,5 arrondi) ; b plus étoilé, noté 5 ; c étoilé.
        let songs = [song("a", true, Some(4.0)), song("b", false, Some(5.0)), song("c", true, None)];
        apply_server_state(&pool, 1, &songs, true).await.unwrap();

        let liked: Vec<String> = sqlx::query_scalar("SELECT path FROM track_liked ORDER BY path").fetch_all(&pool).await.unwrap();
        assert_eq!(liked, vec!["remote://1/al/a", "remote://1/al/c"]);
        let ratings: Vec<Option<f64>> = sqlx::query_scalar("SELECT rating FROM library_tracks ORDER BY id").fetch_all(&pool).await.unwrap();
        assert_eq!(ratings, vec![Some(3.5), Some(5.0), None]);
    }

    #[tokio::test]
    async fn a_newer_action_replaces_the_queued_one() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        // Serveur désactivé : tout part en attente.
        sqlx::query("INSERT INTO media_servers (id, profil_id, name, url, username, enabled) VALUES (1, 1, 'NAS', 'http://nas', 'd', 0)").execute(&pool).await.unwrap();
        let path = "remote://1/al/s1";
        send(&pool, path, Feedback::Star(true)).await;
        send(&pool, path, Feedback::Rating(3)).await;
        send(&pool, path, Feedback::Star(false)).await;
        send(&pool, path, Feedback::Scrobble(1_000)).await;
        send(&pool, path, Feedback::NowPlaying).await;
        let rows: Vec<(String, Option<i64>)> = sqlx::query_as("SELECT action, value FROM server_outbox ORDER BY id").fetch_all(&pool).await.unwrap();
        assert_eq!(rows, vec![("rating".into(), Some(3)), ("unstar".into(), None), ("scrobble".into(), Some(1_000))]);
    }

    // Essai réel sur la démo Navidrome : étoile, note, écoute, puis lecture de l'état côté serveur.
    #[tokio::test]
    #[ignore = "réseau"]
    async fn real_demo_round_trip() {
        use crate::core::subsonic::catalog::parse_song;
        use crate::core::subsonic::client::Auth;
        let (server, _) = Server::connect("demo.navidrome.org", Auth::from_password("demo", "demo")).await.unwrap();
        let random = server.call("getRandomSongs", &[("size", "1")]).await.unwrap();
        let id = parse_song(&random["randomSongs"]["song"][0]).unwrap().id;
        let state = |server: Server, id: String| async move {
            let s = parse_song(&server.call("getSong", &[("id", &id)]).await.unwrap()["song"]).unwrap();
            (s.starred, s.user_rating)
        };
        let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await.unwrap();
        for f in [Feedback::Star(true), Feedback::Rating(4), Feedback::NowPlaying] {
            deliver(&pool, &Backend::Subsonic(server.clone()), &id, &f).await.unwrap();
        }
        let after = state(server.clone(), id.clone()).await;
        println!("{id} : étoilé et noté → {after:?}");
        assert_eq!(after, (true, Some(4.0)));
        deliver(&pool, &Backend::Subsonic(server.clone()), &id, &Feedback::Scrobble(chrono::Utc::now().timestamp_millis())).await.unwrap();
        for f in [Feedback::Star(false), Feedback::Rating(0)] {
            deliver(&pool, &Backend::Subsonic(server.clone()), &id, &f).await.unwrap();
        }
        let back = state(server.clone(), id.clone()).await;
        assert_eq!(back, (false, None));
    }
}
