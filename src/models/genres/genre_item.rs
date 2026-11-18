use karbon_ui::ui::prelude::*;
use crate::models::app::Msg;

/// Une carte ou une ligne de genre.
#[derive(Clone)]
pub struct GenreItem {
    pub name: String,
    pub label: String,
    pub covers: [Option<String>; 4],
    /// Cases sans pochette : un dégradé dans la teinte du genre.
    pub tiles: [Vec<Background>; 4],
    /// `--gc` : couleur du genre (gris pour « Sans genre »).
    pub color: Color,
    /// Dégradé du bas de la mosaïque (`color-mix` de --gc et du noir).
    pub fade: Vec<Background>,
    pub albums_n: String,
    pub albums_label: String,
    pub tracks_n: String,
    pub tracks_label: String,
    pub artists: String,
    pub part: String,
    pub bar: f32,
    pub special: bool,
    pub open: Msg,
}
