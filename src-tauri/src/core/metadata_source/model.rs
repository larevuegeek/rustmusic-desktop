//! Ce que rend une source en ligne, quelle qu'elle soit : l'interface et
//! l'appariement ne connaissent que ces structures.

use serde::{Deserialize, Serialize};

/// Base interrogée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Deezer,
    Musicbrainz,
}

/// Un album trouvé par la recherche.
#[derive(Debug, Clone, Serialize)]
pub struct AlbumHit {
    /// Identifiant propre à la source (nombre Deezer, UUID MusicBrainz).
    pub id: String,
    pub title: String,
    pub artist: String,
    /// Vignette, directement affichable.
    pub cover: Option<String>,
    /// Nombre de pistes annoncé — le premier indice qu'on tient le bon album.
    pub track_count: u32,
    /// Ce qui distingue deux éditions : « 1979 · GB · 2×CD ».
    pub detail: Option<String>,
}

/// Le détail d'un album, avec ses pistes.
#[derive(Debug, Clone, Serialize)]
pub struct AlbumDetail {
    pub id: String,
    pub title: String,
    pub artist: String,
    /// Année seule, extraite de la date de sortie.
    pub year: Option<String>,
    /// Genre principal, au niveau de l'album.
    pub genre: Option<String>,
    pub cover: Option<String>,
    pub tracks: Vec<AlbumTrack>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlbumTrack {
    /// Rang dans tout l'album (1…n) : unique même quand les disques se suivent.
    pub key: u16,
    /// Numéro sur son disque.
    pub position: u16,
    pub disc: u16,
    pub title: String,
    pub artist: String,
    /// En secondes.
    pub duration: u32,
    /// Compositeurs, quand la source les connaît.
    pub composer: Option<String>,
    /// Identifiant de la piste chez la source.
    pub id: String,
}

/// Une piste trouvée par la recherche, pour corriger un seul fichier.
#[derive(Debug, Clone, Serialize)]
pub struct TrackHit {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Vignette de l'album, directement affichable.
    pub cover: Option<String>,
    /// En secondes — le meilleur discriminant entre deux versions d'un titre.
    pub duration: u32,
}

/// Tout ce que la source sait d'une piste, prêt à remplir un formulaire.
///
/// Les champs absents sont `None` et non une chaîne vide : l'interface ne doit
/// jamais proposer d'effacer une valeur existante faute de mieux.
#[derive(Debug, Clone, Serialize)]
pub struct TrackValues {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: Option<String>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<u16>,
    pub total_tracks: Option<u32>,
    pub disc_number: Option<u16>,
    pub composer: Option<String>,
    pub cover: Option<String>,
}
