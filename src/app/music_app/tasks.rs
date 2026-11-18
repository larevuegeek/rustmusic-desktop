//! Tâches de fond : pochettes à charger, animation, dialogues, et l'ouverture d'un
//! fichier dans son dossier.

use chrono::Utc;
use karbon_ui::task::{TaskCommand, TaskResult};
use crate::models::app::Menu;
use crate::models::app::Route;
use crate::models::app::Section;
use crate::models::home::ViewModel;
use super::*;

impl MusicApp {
    pub(crate) fn task_commands(&mut self) -> Vec<TaskCommand> {
        let commands = std::mem::take(&mut self.tasks);
        if *self.history.current() == Route::Radios {
            self.radios_logos();
        }
        if *self.history.current() == Route::Podcasts {
            self.podcasts_images();
        }
        // Pochettes de la page : recalculées après une vue seulement (les données ont pu
        // changer) ; le chargeur garde sa file entre deux demandes.
        if self.covers_due.replace(false) {
            let album_covers = match self.history.current() {
                Route::Library(Section::Albums) => self.albums_covers(),
                Route::Album(_) => self.album_covers(),
                Route::Track(_) => self.track_covers(),
                Route::Tags(_) => self.workshop_covers(),
                Route::Artist(_) => self.artist_covers(),
                Route::Library(Section::Genres) => self.genres_covers(),
                Route::Genre(_) | Route::Decade(_) | Route::Composer(_) => self.genre_covers(),
                Route::Library(Section::Years) => self.years_covers(),
                Route::Library(Section::Folders) => self.folders_covers(),
                Route::Library(Section::Mixes) => self.mixes_covers(),
                Route::Playlists => self.playlists_covers(),
                Route::Playlist(_) | Route::Liked | Route::Recent => self.playlist_covers(),
                Route::Search(_) => self.search_covers(),
                Route::Radios => self.radios_covers(),
                Route::Podcasts | Route::PodcastSubscriptions => self.podcasts_covers(),
                Route::PodcastDownloads => self.downloads_view().groups.into_iter().filter_map(|g| g.cover).collect(),
                Route::Stats => self.data.stats.iter().flat_map(|s| s.top_played.iter().filter_map(|p| p.thumbnail_path.as_deref().map(crate::app::resources::assets::small_key))).collect(),
                Route::Podcast(_) => self.views.show.as_ref().and_then(|p| p.podcast.as_ref()).and_then(|p| p.cover_path.clone()).into_iter().collect(),
                Route::Library(Section::Artists) => self.artists_covers(),
                _ => Vec::new(),
            };
            let file_covers: Vec<String> = if self.menu == Some(Menu::File) {
                let current_item = self.data.current_track().and_then(|t| t.cover.clone());
                current_item.into_iter().chain(self.queue_upcoming().iter().filter_map(|(_, t)| t.cover.as_deref().map(crate::app::resources::assets::small_key))).collect()
            } else {
                Vec::new()
            };
            let mut overlay_covers = self.overlay_covers();
            overlay_covers.extend(self.sidebar_covers());
            if let Some(assets) = self.assets.as_mut() {
                let vm = ViewModel::build(&self.data, self.history.current(), self.source, Utc::now());
                assets.preload(vm.covers());
                if let Some(page) = self.data.page.as_ref().filter(|p| p.route == *self.history.current()) {
                    assets.preload(page.covers());
                }
                for path in self.views.covers(self.history.current()) {
                    assets.preload([path.as_str()]);
                }
                assets.preload(album_covers.iter().map(String::as_str));
                assets.preload(file_covers.iter().map(String::as_str));
                assets.preload(overlay_covers.iter().map(String::as_str));
            }
        }
        commands
    }

    pub(crate) fn on_task(&mut self, result: TaskResult) -> bool {
        if result.id == crate::controller::player::mini_window::ANIM_TASK {
            return self.mini_anim_step();
        }
        if result.id == crate::controller::layout::layout_dialogs::DIALOG_TASK {
            return self.dialog_closed(result);
        }
        // Les chargements de pochettes sont livrés par karbon-ui à ssets.covers.
        false
    }
}


/// Le fichier sélectionné dans son dossier.
pub(crate) fn reveal(path: &str) {
    if path.is_empty() {
        return;
    }
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer").arg(format!("/select,{path}")).spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").args(["-R", path]).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = {
        let dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
        std::process::Command::new("xdg-open").arg(dir).spawn()
    };
    if let Err(e) = result {
        log::warn!("Afficher dans le dossier ({path}) : {e}");
    }
}
