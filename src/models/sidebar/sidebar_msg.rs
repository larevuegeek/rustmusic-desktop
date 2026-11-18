use crate::models::sidebar::*;
use karbon_ui::ui::drag_drop::DropEvent;
use crate::models::tracks::TrackMenuAt;

#[derive(Debug, Clone, PartialEq)]
pub enum SidebarMsg {
    /// Menu d'une ligne : « … » (ancre nommée) ou clic droit (pointeur).
    RowMenu(TrackMenuAt, RowTarget),
    /// Menu d'un album ou d'un artiste épinglé.
    Item(ItemMsg),
    /// Menu d'une section d'épinglés : épingler l'album ou l'artiste en cours.
    PinPlaying(PinKind),
    Browse(PinKind),
    HideSection(PinKind),
    /// Ferme la carte « rien d'épinglé ».
    DismissHint,
    /// Mode « Organiser » : entrer, sortir.
    Organize(bool),
    /// Section `i` montée (`true`) ou descendue.
    Shift(usize, bool),
    DropSection(DropEvent),
    DropPlaylist(DropEvent),
}
