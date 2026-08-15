//! Récupération de métadonnées depuis une source en ligne.
//!
//! # Ce que ces commandes ne font pas
//! Elles **n'écrivent rien**. Elles proposent : l'appariement rend un score et
//! un niveau de confiance par piste, et c'est l'interface qui décide, champ par
//! champ, ce qui est appliqué. Une correction de masse appliquée sur la foi
//! d'un appariement automatique écrirait des métadonnées fausses sur toute une
//! bibliothèque sans que rien ne le signale.

use serde::{Deserialize, Serialize};

use crate::core::metadata_source::deezer;
use crate::core::metadata_source::matching::{self, LocalTrack, RemoteTrack, TrackMatch};

/// Cherche des albums correspondant à la requête.
#[tauri::command]
pub async fn metadata_search_albums(
    query: String,
    limit: Option<u32>,
) -> Result<Vec<deezer::AlbumHit>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    deezer::search_albums(query.trim(), limit.unwrap_or(8)).await
}

/// Cherche des pistes correspondant à la requête.
///
/// Le pendant à l'unité de `metadata_search_albums` : quand on corrige **un**
/// fichier, chercher son album puis y désigner sa piste demande deux décisions
/// là où le titre suffit.
#[tauri::command]
pub async fn metadata_search_tracks(
    query: String,
    limit: Option<u32>,
) -> Result<Vec<deezer::TrackHit>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    deezer::search_tracks(query.trim(), limit.unwrap_or(8)).await
}

/// Tout ce que Deezer sait d'une piste, prêt à remplir un formulaire.
#[tauri::command]
pub async fn metadata_track_values(track_id: i64) -> Result<deezer::TrackValues, String> {
    deezer::track_values(track_id).await
}

/// Une piste locale telle que l'atelier la décrit.
#[derive(Debug, Deserialize)]
pub struct LocalTrackPayload {
    /// Position dans le jeu de travail — c'est par elle que l'interface
    /// retrouvera son fichier.
    pub index: usize,
    pub title: String,
    pub track_number: Option<u16>,
    pub duration: Option<u32>,
}

/// Ce que l'interface reçoit pour construire son écran de revue.
#[derive(Debug, Serialize)]
pub struct MatchProposal {
    pub album: deezer::AlbumDetail,
    /// Un résultat par piste locale, dans l'ordre reçu.
    pub matches: Vec<TrackMatch>,
}

/// Récupère un album et l'apparie au jeu de travail.
///
/// Les deux vont ensemble : proposer un album sans dire quelle piste
/// correspond à quel fichier laisserait tout le travail d'identification à
/// l'utilisateur, morceau par morceau.
#[tauri::command]
pub async fn metadata_match_album(
    album_id: i64,
    tracks: Vec<LocalTrackPayload>,
) -> Result<MatchProposal, String> {
    let album = deezer::album_detail(album_id).await?;

    let local: Vec<LocalTrack> = tracks
        .into_iter()
        .map(|t| LocalTrack {
            index: t.index,
            title: t.title,
            track_number: t.track_number,
            duration: t.duration,
        })
        .collect();

    let remote: Vec<RemoteTrack> = album
        .tracks
        .iter()
        .map(|t| RemoteTrack {
            position: t.position,
            title: t.title.clone(),
            duration: t.duration,
        })
        .collect();

    let matches = matching::match_tracks(&local, &remote);

    Ok(MatchProposal { album, matches })
}
