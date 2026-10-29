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

use crate::entity::playlist::playlist::TrackRefRow;
use crate::repository::playlist::playlist_item_repository::PlaylistItemRepository;
use crate::repository::playlist::playlist_repository::PlaylistRepository;
use crate::repository::playlist::track_liked_repository::TrackLikedRepository;
use crate::repository::profil::profil_repository::ProfilRepository;
use crate::repository::settings::settings_repository::SettingsRepository;

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
    /// Playlist auto : ses règles (JSON).
    #[serde(default)]
    pub rules: Option<String>,
    #[serde(default)]
    pub is_mix: bool,
    #[serde(default = "default_true")]
    pub pinned: bool,
}

fn default_true() -> bool {
    true
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

impl From<TrackRefRow> for ExportTrackRef {
    fn from((path, title, artist, album, duration): TrackRefRow) -> Self {
        ExportTrackRef { path, title, artist, album, duration }
    }
}

/// Nature du fichier. Vérifiée à la relecture.
pub const FORMAT: &str = "rustmusic-export";
/// Version du format d'export.
pub const VERSION: u32 = 1;

/// Rassemble tout ce qui doit partir dans le fichier.
pub async fn collect(pool: &SqlitePool, app_version: &str, now: &str) -> Result<ExportBundle, sqlx::Error> {
    let settings = SettingsRepository::get_all(pool).await?;
    let profils_bruts = ProfilRepository::find_all_for_export(pool).await?;

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
    let listes = PlaylistRepository::find_for_export(pool, profil_id).await?;

    let mut sortie = Vec::with_capacity(listes.len());

    for l in listes {
        // L'ordre des pistes dans une playlist est une donnée à part entière :
        // `sort_index` le porte, et le perdre reviendrait à exporter un sac.
        let pistes = PlaylistItemRepository::track_refs(pool, l.id).await?;

        sortie.push(ExportPlaylist {
            name: l.name,
            description: l.description,
            color: l.color,
            icon: l.icon,
            position: l.position,
            rules: l.rules,
            is_mix: l.is_mix,
            pinned: l.pinned,
            tracks: pistes.into_iter().map(ExportTrackRef::from).collect(),
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
    let pistes = TrackLikedRepository::track_refs(pool, profil_id).await?;
    Ok(pistes.into_iter().map(ExportTrackRef::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track_ref(path: &str) -> ExportTrackRef {
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
                    rules: None,
                    is_mix: false,
                    pinned: true,
                    name: "Grunge".into(),
                    description: None,
                    color: "#8b5cf6".into(),
                    icon: "mynaui:music".into(),
                    position: 0,
                    tracks: vec![track_ref("S:\\Musique\\a.flac"), track_ref("S:\\Musique\\b.flac")],
                }],
                liked: vec![track_ref("S:\\Musique\\c.flac")],
            }],
        }
    }

    #[test]
    fn the_file_reads_back_identically() {
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
    fn the_track_order_is_kept() {
        // L'ordre d'une playlist est une donnée, pas un détail d'affichage.
        let json = serde_json::to_string(&bundle()).unwrap();
        let relu: ExportBundle = serde_json::from_str(&json).unwrap();
        let pistes = &relu.profils[0].playlists[0].tracks;
        assert_eq!(pistes[0].path, "S:\\Musique\\a.flac");
        assert_eq!(pistes[1].path, "S:\\Musique\\b.flac");
    }

    #[test]
    fn windows_paths_survive_json() {
        // Les antislashs d'un chemin Windows doivent ressortir tels quels : une
        // playlist dont les chemins sont mutilés ne se réimporterait jamais.
        let json = serde_json::to_string(&bundle()).unwrap();
        let relu: ExportBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(relu.profils[0].playlists[0].tracks[0].path, "S:\\Musique\\a.flac");
    }

    #[test]
    fn a_foreign_file_is_recognised_by_its_kind() {
        // Relire n'importe quel JSON comme un export produirait des dégâts
        // silencieux : le marqueur de format est là pour permettre le refus.
        let etranger = r#"{"hello":"world"}"#;
        assert!(serde_json::from_str::<ExportBundle>(etranger).is_err());
    }
}
