use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlaylistTrackView {
    pub playlist_item_id: i64,
    pub playlist_id: i64,
    pub sort_index: i64,

    pub library_track_id: String,
    pub title: Option<String>,
    pub duration: Option<f64>,
    /// Nombre d'écoutes de la piste.
    ///
    /// Porté jusqu'ici parce qu'une playlist est justement l'endroit où l'on se
    /// demande ce qu'on a déjà usé — et où une liste « les plus écoutés » doit
    /// pouvoir se justifier ligne à ligne.
    pub play_count: i64,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,

    pub album_id: Option<String>,
    pub album_title: Option<String>,

    pub artist_id: Option<String>,
    pub artist_name: Option<String>,

    pub path: Option<String>,
    pub thumbnail_path: Option<String>,
}