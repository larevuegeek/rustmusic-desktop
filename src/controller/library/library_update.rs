//! La bibliothèque : chargement des données et réponse aux messages.

use crate::models::library::*;
use crate::app::music_app::MusicApp;
use crate::models::app::Route;
use crate::models::app::Section;
use crate::commands::library::library_command;
use crate::models::app::ScanProgress;

/// Le menu « Gérer » d'une bibliothèque ouvert depuis la barre latérale.
pub(crate) const LIBRARY_MANAGE: &str = "sidebar.library.manage";

impl MusicApp {
    /// Les dossiers de la bibliothèque choisie (poids, dernier scan, état).
    pub(crate) fn load_library_dirs(&mut self) {
        let Some(id) = self.data.library_id else {
            self.data.library_dirs.clear();
            return;
        };
        let app = self.backend.app.clone();
        self.backend.call(
            async move { library_command::get_library_dirs(app.state(), id).await },
            move |me, dirs| {
                if me.data.library_id == Some(id) {
                    me.data.library_dirs = dirs.unwrap_or_default();
                }
                true
            },
        );
    }

    /// Cent titres tirés au hasard dans la bibliothèque.
    pub(crate) fn random_mix(&mut self) {
        let Some(library) = self.data.library_id else { return };
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                library_command::get_mix_tracks(app.state(), library, "hasard".into(), None, None, None, 100)
                    .await
                    .unwrap_or_default()
            },
            |me, tracks| {
                if let Some(first) = tracks.first().map(|t| t.path.clone()) {
                    me.play_tracks_view(tracks, &first);
                }
                true
            },
        );
    }

    /// Messages de l'en-tête et du menu « Gérer ».
    pub(crate) fn library_update(&mut self, msg: LibraryMsg) -> bool {
        if !matches!(msg, LibraryMsg::Switch(_) | LibraryMsg::Manage(_)) {
            self.menu = None;
        }
        match msg {
            LibraryMsg::Switch(id) => {
                self.menu = None;
                if self.data.library_id != Some(id) {
                    self.data.library_id = Some(id);
                    self.load_library_data();
                    self.load_page();
                }
            }
            LibraryMsg::Primary => match self.route() {
                Route::Library(Section::Tracks) => self.tracks_play_all(),
                Route::Library(Section::Genres) | Route::Library(Section::Years) => self.random_mix(),
                _ => {
                    let Some(library) = self.data.library_id else { return false };
                    self.ask(crate::models::layout::Choice::ImportFolder { library, redirect: false });
                }
            },
            LibraryMsg::Manage(id) => return self.library_manage(id),
            LibraryMsg::CloseManage => {
                self.views.library_manage_nested = false;
                self.views.library_manage = None;
            }
            LibraryMsg::Rescan(id) => self.rescan_library(id),
            LibraryMsg::Folders(id) => self.open_library_folders(id),
            LibraryMsg::ImportExport => {
                self.history.push(Route::Settings(crate::models::app::SettingsPage::Storage));
                self.load_page();
            }
            LibraryMsg::Covers(id) => {
                let app = self.backend.app.clone();
                self.backend.call(
                    async move { library_command::fetch_all_album_covers(app.clone(), app.state(), id).await },
                    move |me, _| {
                        if me.data.library_id == Some(id) {
                            me.load_library_data();
                        }
                        true
                    },
                );
            }
            LibraryMsg::Portraits => {
                let app = self.backend.app.clone();
                self.backend.call(
                    async move { library_command::fetch_all_artist_images(app.clone(), app.state(), None).await },
                    |_, _| true,
                );
            }
            LibraryMsg::CheckArtists => return self.artist_check_update(crate::models::artists::ArtistCheckMsg::Start),
            LibraryMsg::MakeDefault(id) => {
                let app = self.backend.app.clone();
                self.backend.call(
                    async move { library_command::set_default_library(app.state(), id).await },
                    |me, _| {
                        me.load_profile_data();
                        true
                    },
                );
            }
            LibraryMsg::Rename(id) => return self.rename_library(id),
            LibraryMsg::Delete(id) => return self.ask_delete_library(id),
            LibraryMsg::AddFiles(library) => {
                if !self.data.libraries.iter().any(|l| l.id == library) {
                    return false;
                }
                self.ask(crate::models::layout::Choice::ImportFiles { library, redirect: false });
            }
        }
        true
    }
}

impl MusicApp {
    /// Le menu « Gérer » d'une bibliothèque de la liste de la barre latérale : ouvert à
    /// côté d'elle, ou refermé ; ses dossiers sont lus (ceux de la bibliothèque ouverte le
    /// sont déjà).
    fn library_manage(&mut self, id: i64) -> bool {
        let same = self.views.library_manage.as_ref().is_some_and(|(open, _)| *open == id);
        if self.open(crate::models::app::Menu::Library) {
            // Bouton « … » d'une ligne de la liste déroulante : le menu s'ouvre par-dessus,
            // la liste reste ouverte ; un second clic sur le même bouton le referme.
            if self.views.library_manage_nested && same {
                self.views.library_manage_nested = false;
                self.views.library_manage = None;
                return true;
            }
            self.views.library_manage_nested = true;
        } else {
            if self.popup(LIBRARY_MANAGE) && same {
                self.menu = None;
                return true;
            }
            self.views.library_manage_nested = false;
            self.menu = Some(crate::models::app::Menu::Popup(LIBRARY_MANAGE));
        }
        self.views.library_manage = Some((id, Vec::new()));
        if self.data.library_id == Some(id) {
            return true;
        }
        let app = self.backend.app.clone();
        self.backend.call(
            async move { library_command::get_library_dirs(app.state(), id).await },
            move |me, dirs| match me.views.library_manage.as_mut() {
                Some((open, list)) if *open == id => {
                    *list = dirs.unwrap_or_default();
                    true
                }
                _ => false,
            },
        );
        true
    }

    /// Scan de la bibliothèque ouverte.
    pub(crate) fn rescan(&mut self) {
        if let Some(id) = self.data.library_id {
            self.rescan_library(id);
        }
    }

    pub(crate) fn rescan_library(&mut self, id: i64) {
        if self.data.scan.is_some() {
            return;
        }
        self.data.scan = Some(ScanProgress {
            label: "Scan".into(),
            library_id: Some(id),
            started: Some(std::time::Instant::now()),
            ..Default::default()
        });
        let app = self.backend.app.clone();
        self.backend.call(
            async move { library_command::rescan_library(app.clone(), app.state(), id).await },
            |me, result| {
                if let Err(e) = result {
                    log::warn!("Scan : {e}");
                }
                me.data.scan = None;
                me.load_profile_data();
                true
            },
        );
    }
}
