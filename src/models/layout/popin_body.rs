use crate::models::app::Msg;

/// Le contenu d'une fenêtre.
#[derive(Debug, Clone)]
pub enum PopinBody {
    /// Confirmation : message, bouton principal, bouton d'annulation, rouge, action confirmée.
    Confirm { message: String, confirm: String, cancel: String, danger: bool, msg: Box<Msg> },
    /// Création ou modification d'une playlist.
    PlaylistForm(Box<crate::models::playlists::PlaylistForm>),
    /// Les nouveautés de la version (état dans `views.whats_new`).
    WhatsNew,
    /// Playlist intelligente ou mix.
    Smart(Box<crate::models::playlists::SmartForm>),
    /// Ajout d'un serveur.
    AddServer(Box<crate::models::servers::AddServer>),
    /// Dossiers suivis d'un serveur.
    ServerFolders(Box<crate::models::servers::ServerFolders>),
    /// Aperçu d'un podcast avant abonnement.
    PodcastPreview(Box<crate::models::podcasts::PodcastPreviewState>),
    /// Édition des tags d'un morceau.
    TagEdit(Box<crate::models::tags::TagEditor>),
    /// Ajout d'une bibliothèque.
    LibraryForm(Box<crate::models::library::LibraryForm>),
    /// Vérification des noms d'artistes (état dans `views.artist_check`).
    ArtistCheck,
    /// Nettoyage des tags.
    Clean(Box<crate::models::tags::CleanState>),
    /// Historique des modifications en lot.
    History(Box<crate::models::tags::HistoryState>),
    /// Renommage des fichiers d'après leurs tags.
    Rename(Box<crate::models::tags::RenameState>),
    /// Édition des tags de plusieurs morceaux.
    BatchEdit(Box<crate::models::tags::BatchEditor>),
    /// Tags complétés depuis une source en ligne.
    Source(Box<crate::models::tags::SourceState>),
}
