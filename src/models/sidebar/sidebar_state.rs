use crate::models::sidebar::*;
use crate::entity::library::library_pin::LibraryPinView;
use crate::entity::library::library_track::TrackLocation;

/// État de la barre que `Data` ne garde pas.
#[derive(Default)]
pub struct SidebarState {
    /// La bibliothèque et ses épingles.
    pub pins_library: Option<i64>,
    pub pins: Vec<LibraryPinView>,
    /// `locateOne($player.pathFile)` : le chemin localisé et son lieu.
    pub place: Option<(String, Option<TrackLocation>)>,
    /// Mode « Organiser ».
    pub organize: bool,
    /// Le menu d'un épinglé ouvert.
    pub item_menu: Option<ItemMenu>,
}
