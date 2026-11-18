use crate::models::layout::*;
use crate::app::music_app::MusicApp;

impl MusicApp {
    pub(crate) fn open_popin(&mut self, mut popin: Popin) {
        self.menu = None;
        // Sans en-tête implique `flush` (le corps sans marge).
        popin.flush |= popin.bare;
        self.views.popin = Some(popin);
    }

    /// Le bouton principal : ferme, puis fait l'action confirmée.
    pub(crate) fn popin_confirm(&mut self) -> bool {
        let Some(popin) = self.views.popin.take() else { return false };
        match popin.body {
            PopinBody::Confirm { msg, .. } => {
                self.handle(*msg);
            }
            PopinBody::WhatsNew | PopinBody::ArtistCheck | PopinBody::History(_) => {}
            body @ (PopinBody::Rename(_) | PopinBody::Source(_)) => self.views.popin = Some(Popin { body, ..popin }),
            body @ PopinBody::BatchEdit(_) => {
                self.views.popin = Some(Popin { body, ..popin });
                return self.batch_editor_update(crate::models::tags::BatchEditorMsg::Submit);
            }
            body @ PopinBody::Clean(_) => {
                self.views.popin = Some(Popin { body, ..popin });
                return self.clean_update(crate::models::tags::CleanMsg::Apply);
            }
            body @ PopinBody::PodcastPreview(_) => self.views.popin = Some(Popin { body, ..popin }),
            body @ PopinBody::LibraryForm(_) => {
                self.views.popin = Some(Popin { body, ..popin });
                return self.library_form_update(crate::models::library::LibraryFormMsg::Submit);
            }
            body @ PopinBody::TagEdit(_) => {
                self.views.popin = Some(Popin { body, ..popin });
                return self.tag_editor_update(crate::models::tags::TagEditorMsg::Submit);
            }
            body @ (PopinBody::AddServer(_) | PopinBody::ServerFolders(_)) => {
                self.views.popin = Some(Popin { body, ..popin });
                return self.server_form_update(crate::models::servers::ServerFormMsg::Submit);
            }
            PopinBody::Smart(f) => {
                self.views.popin = Some(Popin { body: PopinBody::Smart(f), ..popin });
                return self.smart_update(crate::models::playlists::SmartMsg::Submit);
            }
            PopinBody::PlaylistForm(f) => {
                self.views.popin = Some(Popin { body: PopinBody::PlaylistForm(f), ..popin });
                return self.playlist_form_update(crate::models::playlists::PlaylistFormMsg::Submit);
            }
        }
        true
    }
}
