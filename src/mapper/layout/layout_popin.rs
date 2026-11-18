use crate::models::layout::*;
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;

impl Popin {
    pub fn confirm(title: String, message: String, confirm: String, cancel: String, danger: bool, msg: Msg) -> Self {
        Popin::new(title, PopinBody::Confirm { message, confirm, cancel, danger, msg: Box::new(msg) })
    }

    /// Taille `md`, avec en-tête.
    pub fn new(title: String, body: PopinBody) -> Self {
        Popin { title, icon: None, body, size: PopinSize::Md, bare: false, flush: false }
    }

    /// Largeur maximale en pixels.
    pub fn max_width(&self) -> f32 {
        match self.size {
            PopinSize::Md => 448.,
            PopinSize::Lg => 672.,
            PopinSize::Xl => 768.,
            PopinSize::Xxl => 980.,
        }
    }

    pub fn smart(&self) -> Option<&crate::models::playlists::SmartForm> {
        match &self.body {
            PopinBody::Smart(f) => Some(f),
            _ => None,
        }
    }

    pub fn podcast_preview(&self) -> Option<&crate::models::podcasts::PodcastPreviewState> {
        match &self.body {
            PopinBody::PodcastPreview(p) => Some(p),
            _ => None,
        }
    }

    pub fn is_whats_new(&self) -> bool {
        matches!(self.body, PopinBody::WhatsNew)
    }

    /// La confirmation : message, libellés, rouge.
    pub fn confirmation(&self) -> Option<(&String, &String, &String, bool)> {
        match &self.body {
            PopinBody::Confirm { message, confirm, cancel, danger, .. } => Some((message, confirm, cancel, *danger)),
            _ => None,
        }
    }

    pub fn playlist_form(&self) -> Option<&crate::models::playlists::PlaylistForm> {
        match &self.body {
            PopinBody::PlaylistForm(f) => Some(f),
            _ => None,
        }
    }
}

impl MusicApp {
    /// Images des fenêtres ouvertes : médias de l'éditeur de tags (adresses `data:`), pochette
    /// proposée, aperçu de podcast, avatars de l'écran des profils.
    pub(crate) fn overlay_covers(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(p) = &self.views.popin {
            match &p.body {
                PopinBody::TagEdit(e) => {
                    out.extend(e.slots.iter().map(|s| s.src.clone()));
                    out.extend(e.lookup.as_ref().and_then(|l| l.cover.as_ref()).map(|c| c.src.clone()));
                }
                PopinBody::PodcastPreview(pp) => out.extend(pp.image.clone()),
                PopinBody::ArtistCheck => out.extend(self.views.artist_check.images.values().cloned()),
                _ => {}
            }
        }
        if let Some(c) = &self.views.cover_search {
            out.extend(c.results.iter().filter_map(|h| h.thumb.clone()));
        }
        if self.views.profiles.is_some() {
            out.extend(self.data.profiles.iter().filter_map(|p| p.avatar.clone()));
            out.extend(self.views.profiles.as_ref().and_then(|s| s.avatar.clone()));
        }
        out
    }
}
