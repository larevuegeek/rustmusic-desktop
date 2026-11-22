//! Les infos cliquables de la barre du lecteur : la fiche du morceau, de son artiste, de
//! son album et de son année.

use crate::app::music_app::MusicApp;
use crate::models::app::{Msg, Route};
use crate::models::player::PlayingLink;
use crate::models::years::YearsMsg;
use crate::entity::library::library_track::TrackLocation;

impl MusicApp {
    /// Le message qui ouvre la fiche, ou `None` : morceau hors de la bibliothèque ouverte
    /// (fichier ouvert à la volée, autre bibliothèque), lieu pas encore connu, sans année.
    pub fn playing_link(&self, link: PlayingLink, year: &str) -> Option<Msg> {
        let route = match link {
            PlayingLink::Year => {
                let year: i64 = year.trim().parse().ok().filter(|y| *y > 0)?;
                return Some(Msg::Years(YearsMsg::Open(year - year % 10, Some(year))));
            }
            PlayingLink::Track => Route::Track(self.playing_place()?.library_track_id.clone()),
            PlayingLink::Artist => Route::Artist(self.playing_place()?.artist_id.clone()?),
            PlayingLink::Album => Route::Album(self.playing_place()?.library_album_id.clone()?),
        };
        Some(Msg::Go(route))
    }

    /// Le lieu du morceau en cours, s'il est dans la bibliothèque ouverte.
    fn playing_place(&self) -> Option<&TrackLocation> {
        let (path, place) = self.views.sidebar.place.as_ref()?;
        place
            .as_ref()
            .filter(|p| self.data.player.path.as_deref() == Some(path.as_str()) && self.data.library_id == Some(p.library_id))
    }
}
