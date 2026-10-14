//! Récupération de métadonnées depuis une source en ligne.
//!
//! # Ce que ces commandes ne font pas
//! Elles **n'écrivent rien**. Elles proposent : l'appariement rend un score et
//! un niveau de confiance par piste, et c'est l'interface qui décide, champ par
//! champ, ce qui est appliqué. Une correction de masse appliquée sur la foi
//! d'un appariement automatique écrirait des métadonnées fausses sur toute une
//! bibliothèque sans que rien ne le signale.

use serde::{Deserialize, Serialize};

use crate::core::metadata_source::matching::{self, LocalTrack, RemoteTrack, TrackMatch};
use crate::core::metadata_source::model::{AlbumDetail, AlbumHit, Source, TrackHit, TrackValues};
use crate::core::metadata_source::{deezer, musicbrainz};

/// Cherche des albums correspondant à la requête.
#[tauri::command]
pub async fn metadata_search_albums(source: Source, query: String, limit: Option<u32>) -> Result<Vec<AlbumHit>, String> {
    let (query, limit) = (query.trim(), limit.unwrap_or(8));
    if query.is_empty() {
        return Ok(Vec::new());
    }
    match source {
        Source::Deezer => deezer::search_albums(query, limit).await,
        Source::Musicbrainz => musicbrainz::search_albums(query, limit).await,
    }
}

/// Cherche des pistes correspondant à la requête.
///
/// Le pendant à l'unité de `metadata_search_albums` : quand on corrige **un**
/// fichier, chercher son album puis y désigner sa piste demande deux décisions
/// là où le titre suffit.
#[tauri::command]
pub async fn metadata_search_tracks(source: Source, query: String, limit: Option<u32>) -> Result<Vec<TrackHit>, String> {
    let (query, limit) = (query.trim(), limit.unwrap_or(8));
    if query.is_empty() {
        return Ok(Vec::new());
    }
    match source {
        Source::Deezer => deezer::search_tracks(query, limit).await,
        Source::Musicbrainz => musicbrainz::search_tracks(query, limit).await,
    }
}

/// Tout ce que la source sait d'une piste, prêt à remplir un formulaire.
#[tauri::command]
pub async fn metadata_track_values(source: Source, track_id: String) -> Result<TrackValues, String> {
    match source {
        Source::Deezer => deezer::track_values(&track_id).await,
        Source::Musicbrainz => musicbrainz::track_values(&track_id).await,
    }
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
    pub album: AlbumDetail,
    /// Un résultat par piste locale, dans l'ordre reçu.
    pub matches: Vec<TrackMatch>,
}

/// Récupère un album et l'apparie au jeu de travail.
///
/// Les deux vont ensemble : proposer un album sans dire quelle piste
/// correspond à quel fichier laisserait tout le travail d'identification à
/// l'utilisateur, morceau par morceau.
#[tauri::command]
pub async fn metadata_match_album(source: Source, album_id: String, tracks: Vec<LocalTrackPayload>) -> Result<MatchProposal, String> {
    let album = match source {
        Source::Deezer => deezer::album_detail(&album_id).await?,
        Source::Musicbrainz => musicbrainz::album_detail(&album_id).await?,
    };

    let local: Vec<LocalTrack> = tracks
        .into_iter()
        .map(|t| LocalTrack { index: t.index, title: t.title, track_number: t.track_number, duration: t.duration })
        .collect();
    let remote: Vec<RemoteTrack> = album
        .tracks
        .iter()
        .map(|t| RemoteTrack { key: t.key, position: t.position, title: t.title.clone(), duration: t.duration })
        .collect();
    let matches = matching::match_tracks(&local, &remote);

    Ok(MatchProposal { album, matches })
}
