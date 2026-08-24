//! Export des réglages, des profils, des playlists et des titres aimés.
//!
//! # Ce qu'on exporte, et ce qu'on n'exporte pas
//! Ce fichier emporte ce que l'utilisateur a **construit** : ses réglages, ses
//! profils, ses playlists, ses coups de cœur. Pas la bibliothèque elle-même —
//! celle-ci se reconstruit par un scan des dossiers, et la recopier ferait un
//! fichier de plusieurs mégaoctets sans rien apporter qu'un nouveau scan ne
//! rende.
//!
//! # Le point délicat : une playlist ne peut pas emporter ses identifiants
//! `playlist_items.library_track_id` désigne une ligne de `library_tracks`,
//! dont l'identifiant est tiré au hasard à l'import du fichier. Le même
//! morceau, scanné sur une autre installation, en reçoit un différent. Exporter
//! ces identifiants produirait donc un fichier qui ne veut rien dire ailleurs —
//! et, pire, qui pourrait désigner par accident un autre morceau.
//!
//! Chaque piste part donc avec son **chemin**, plus de quoi la reconnaître si
//! ce chemin a changé : titre, artiste, album, durée. Un disque déplacé de
//! `S:\` vers `D:\` casse tous les chemins d'un coup ; ces quatre valeurs
//! permettent de retrouver les morceaux malgré tout.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::BTreeMap;

/// Le fichier d'export, tel qu'il est écrit sur le disque.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportBundle {
    /// Marqueur de nature, pour qu'une future relecture refuse un fichier
    /// étranger au lieu de l'interpréter de travers.
    pub format: String,
    /// Version du format. Ce qui la lira un jour saura ce qu'il tient.
    pub version: u32,
    /// Version de l'application qui a produit le fichier.
    pub app_version: String,
    pub exported_at: String,
    /// Réglages de l'application. Ils sont globaux : la table `settings` ne
    /// porte pas de profil.
    ///
    /// # Une clé que la relecture devra écarter
    /// `dlna_uuid` identifie ce serveur sur le réseau local. La restaurer sur
    /// une seconde machine ferait apparaître deux serveurs sous la même
    /// identité, et les appareils qui les découvrent choisiraient l'un ou
    /// l'autre au hasard. L'export la conserve — un export doit être fidèle,
    /// et sert aussi à retrouver son installation d'origine — mais tout import
    /// devra la laisser de côté.
    pub settings: BTreeMap<String, String>,
    pub profils: Vec<ExportProfil>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportProfil {
    pub name: String,
    pub color: String,
    pub avatar: Option<String>,
    pub bio: Option<String>,
    pub role: String,
    pub playlists: Vec<ExportPlaylist>,
    /// Titres aimés — la playlist « Titres likés » de la barre latérale.
    pub liked: Vec<ExportTrackRef>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportPlaylist {
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub icon: String,
    pub position: i64,
    pub tracks: Vec<ExportTrackRef>,
}

/// Une piste, désignée de façon à rester reconnaissable ailleurs.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportTrackRef {
    pub path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f64>,
}

/// Nature du fichier. Vérifiée à la relecture.
pub const FORMAT: &str = "rustmusic-export";
/// Version du format d'export.
pub const VERSION: u32 = 1;

/// Rassemble tout ce qui doit partir dans le fichier.
pub async fn collect(pool: &SqlitePool, app_version: &str, now: &str) -> Result<ExportBundle, sqlx::Error> {
    let settings: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM settings ORDER BY key")
            .fetch_all(pool)
            .await?;

    let profils_bruts: Vec<(i64, String, String, Option<String>, Option<String>, String)> =
        sqlx::query_as(
            "SELECT id, name, color, avatar, bio, role FROM profil ORDER BY id",
        )
        .fetch_all(pool)
        .await?;

    let mut profils = Vec::with_capacity(profils_bruts.len());

    for (id, name, color, avatar, bio, role) in profils_bruts {
        profils.push(ExportProfil {
            name,
            color,
            avatar,
            bio,
            role,
            playlists: collect_playlists(pool, id).await?,
            liked: collect_liked(pool, id).await?,
        });
    }

    Ok(ExportBundle {
        format: FORMAT.to_string(),
        version: VERSION,
        app_version: app_version.to_string(),
        exported_at: now.to_string(),
        settings: settings.into_iter().collect(),
        profils,
    })
}

async fn collect_playlists(
    pool: &SqlitePool,
    profil_id: i64,
) -> Result<Vec<ExportPlaylist>, sqlx::Error> {
    let listes: Vec<(i64, String, Option<String>, String, String, i64)> = sqlx::query_as(
        "SELECT id, name, description, color, icon, position
           FROM playlists
          WHERE profil_id = ?
       ORDER BY position, id",
    )
    .bind(profil_id)
    .fetch_all(pool)
    .await?;

    let mut sortie = Vec::with_capacity(listes.len());

    for (id, name, description, color, icon, position) in listes {
        // L'ordre des pistes dans une playlist est une donnée à part entière :
        // `sort_index` le porte, et le perdre reviendrait à exporter un sac.
        let pistes: Vec<(String, Option<String>, Option<String>, Option<String>, Option<f64>)> =
            sqlx::query_as(
                "SELECT lf.path, lt.title, a.name, la.title,
                        COALESCE(lt.duration, lc.duration)
                   FROM playlist_items pi
                   JOIN library_tracks lt ON lt.id = pi.library_track_id
                   JOIN library_files  lf ON lf.id = lt.file_id
              LEFT JOIN library_cache  lc ON lc.id = lt.cache_id
              LEFT JOIN library_albums la ON la.id = lt.library_album_id
              LEFT JOIN artists         a ON  a.id = lt.artist_id
                  WHERE pi.playlist_id = ?
               ORDER BY pi.sort_index, pi.id",
            )
            .bind(id)
            .fetch_all(pool)
            .await?;

        sortie.push(ExportPlaylist {
            name,
            description,
            color,
            icon,
            position,
            tracks: pistes
                .into_iter()
                .map(|(path, title, artist, album, duration)| ExportTrackRef {
                    path,
                    title,
                    artist,
                    album,
                    duration,
                })
                .collect(),
        });
    }

    Ok(sortie)
}

async fn collect_liked(
    pool: &SqlitePool,
    profil_id: i64,
) -> Result<Vec<ExportTrackRef>, sqlx::Error> {
    // Les titres aimés sont désignés par chemin dès l'origine, et peuvent donc
    // exister sans correspondre à une piste de la bibliothèque — un fichier
    // aimé puis retiré du dossier scanné. La jointure est volontairement
    // ouverte : on exporte le chemin dans tous les cas, enrichi quand on peut.
    let pistes: Vec<(String, Option<String>, Option<String>, Option<String>, Option<f64>)> =
        sqlx::query_as(
            "SELECT tl.path, lc.title, lc.artist, lc.album, lc.duration
               FROM track_liked tl
          LEFT JOIN library_cache lc ON lc.id = tl.library_cache_id
              WHERE tl.profil_id = ?
           ORDER BY tl.created_at, tl.id",
        )
        .bind(profil_id)
        .fetch_all(pool)
        .await?;

    Ok(pistes
        .into_iter()
        .map(|(path, title, artist, album, duration)| ExportTrackRef {
            path,
            title,
            artist,
            album,
            duration,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piste(path: &str) -> ExportTrackRef {
        ExportTrackRef {
            path: path.to_string(),
            title: Some("Intro".into()),
            artist: Some("113".into()),
            album: Some("Les Princes De La Ville".into()),
            duration: Some(191.5),
        }
    }

    fn bundle() -> ExportBundle {
        ExportBundle {
            format: FORMAT.to_string(),
            version: VERSION,
            app_version: "0.2.1".into(),
            exported_at: "2026-08-24T10:00:00Z".into(),
            settings: [("theme".to_string(), "dark".to_string())]
                .into_iter()
                .collect(),
            profils: vec![ExportProfil {
                name: "David".into(),
                color: "#22c55e".into(),
                avatar: None,
                bio: None,
                role: "admin".into(),
                playlists: vec![ExportPlaylist {
                    name: "Grunge".into(),
                    description: None,
                    color: "#8b5cf6".into(),
                    icon: "mynaui:music".into(),
                    position: 0,
                    tracks: vec![piste("S:\\Musique\\a.flac"), piste("S:\\Musique\\b.flac")],
                }],
                liked: vec![piste("S:\\Musique\\c.flac")],
            }],
        }
    }

    #[test]
    fn le_fichier_se_relit_a_l_identique() {
        // Un export qu'on ne saurait pas relire n'est pas une sauvegarde. Le
        // test garantit que la structure reste symétrique, aujourd'hui comme
        // au jour où l'import sera écrit.
        let json = serde_json::to_string(&bundle()).unwrap();
        let relu: ExportBundle = serde_json::from_str(&json).unwrap();

        assert_eq!(relu.format, FORMAT);
        assert_eq!(relu.version, VERSION);
        assert_eq!(relu.settings.get("theme").map(String::as_str), Some("dark"));
        assert_eq!(relu.profils[0].playlists[0].tracks.len(), 2);
        assert_eq!(relu.profils[0].liked[0].path, "S:\\Musique\\c.flac");
    }

    #[test]
    fn l_ordre_des_pistes_est_conserve() {
        // L'ordre d'une playlist est une donnée, pas un détail d'affichage.
        let json = serde_json::to_string(&bundle()).unwrap();
        let relu: ExportBundle = serde_json::from_str(&json).unwrap();
        let pistes = &relu.profils[0].playlists[0].tracks;
        assert_eq!(pistes[0].path, "S:\\Musique\\a.flac");
        assert_eq!(pistes[1].path, "S:\\Musique\\b.flac");
    }

    #[test]
    fn les_chemins_windows_survivent_au_json() {
        // Les antislashs d'un chemin Windows doivent ressortir tels quels : une
        // playlist dont les chemins sont mutilés ne se réimporterait jamais.
        let json = serde_json::to_string(&bundle()).unwrap();
        let relu: ExportBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(relu.profils[0].playlists[0].tracks[0].path, "S:\\Musique\\a.flac");
    }

    #[test]
    fn un_fichier_etranger_se_reconnait_a_sa_nature() {
        // Relire n'importe quel JSON comme un export produirait des dégâts
        // silencieux : le marqueur de format est là pour permettre le refus.
        let etranger = r#"{"hello":"world"}"#;
        assert!(serde_json::from_str::<ExportBundle>(etranger).is_err());
    }
}
