//! Traitement des messages de la vue (`Msg`).

use chrono::Utc;
use karbon_ui::{
    ui::prelude::*,
    ui::runtime::Key,
    window::{WindowAction, WindowCommand, WindowId},
};
use crate::models::app::Menu;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::app::Section;
use crate::models::podcasts::PodcastsMsg;
use crate::models::home::ViewModel;
use crate::app::music_app::*;

impl MusicApp {
    /// Traite un message ; `true` reconstruit la vue.
    pub(crate) fn handle(&mut self, message: Msg) -> bool {
        match message {
            Msg::Noop => return false,
            Msg::Go(route) => {
                self.menu = None;
                // La sélection appartient à la page qu'on quitte.
                self.views.selection = None;
                if !self.history.push(route) {
                    return false;
                }
            }
            Msg::Back => {
                if !self.history.back() {
                    return false;
                }
            }
            Msg::SettingsBack => {
                if !self.history.back_past(|r| matches!(r, Route::Settings(_)), Route::Home) {
                    return false;
                }
            }
            Msg::Forward => {
                if !self.history.forward() {
                    return false;
                }
            }
            // `if (minimizeToTray) appWindow.hide(); else appWindow.close();`
            Msg::Close if self.closes_to_tray() => {
                self.window_commands
                    .push(WindowCommand::Change(WindowId::MAIN, karbon_ui::window::WindowChange::Visible(false)));
                return false;
            }
            Msg::Minimize | Msg::Maximize | Msg::Close => {
                let action = match message {
                    Msg::Minimize => WindowAction::Minimize,
                    Msg::Maximize => WindowAction::ToggleMaximize,
                    _ => WindowAction::Close,
                };
                self.window_commands.push(WindowCommand::Action(WindowId::MAIN, action));
                return false;
            }
            Msg::Quit => {
                self.window_commands.push(WindowCommand::Exit);
                return false;
            }
            Msg::Mini(m) => return self.mini_update(m),
            Msg::ToggleSidebar => {
                self.sidebar_collapsed = !self.sidebar_collapsed;
                self.set_setting("sidebar_collapsed", if self.sidebar_collapsed { "true" } else { "false" });
                if matches!(self.menu, Some(Menu::Open | Menu::Display)) {
                    self.menu = None;
                }
                // Repliée, la barre quitte le mode « Organiser ».
                if self.sidebar_collapsed {
                    self.views.sidebar.organize = false;
                }
            }
            Msg::ToggleTheme => {
                // Le bouton bascule entre clair et sombre.
                self.set_setting("theme", if self.dark.get() { "light" } else { "dark" });
            }
            Msg::ToggleFooter => self.footer_collapsed = !self.footer_collapsed,
            Msg::Menu(menu) => {
                self.menu = if self.open(menu) { None } else { Some(menu) };
                self.lyrics_sync();
            }
            Msg::CloseMenu => self.menu = None,

            Msg::Input(field, value) => {
                return match field {
                    crate::models::app::Field::TracksSearch => self.tracks_search(value),
                    crate::models::app::Field::AlbumsSearch => self.albums_search(value),
                    crate::models::app::Field::ArtistsSearch => self.artists_search(value),
                    crate::models::app::Field::GenresSearch => self.genres_search(value),
                    crate::models::app::Field::PlaylistsSearch => self.playlists_search(value),
                    crate::models::app::Field::PageSearch if *self.route() == Route::Library(Section::Folders) => self.folders_search(value),
                    crate::models::app::Field::PageSearch if *self.route() == Route::PodcastSubscriptions => self.podcasts_update(PodcastsMsg::Filter(value)),
                    _ => false,
                };
            }
            Msg::Library(m) => return self.library_update(m),
            Msg::Tracks(m) => return self.tracks_update(m),
            Msg::Albums(m) => return self.albums_update(m),
            Msg::Album(m) => return self.album_update(m),
            Msg::Artist(m) => return self.artist_update(m),
            Msg::Genres(m) => return self.genres_update(m),
            Msg::Genre(m) => return self.genre_update(m),
            Msg::Years(m) => return self.years_update(m),
            Msg::Folders(m) => return self.folders_update(m),
            Msg::Mixes(m) => return self.mixes_update(m),
            Msg::Playlists(m) => return self.playlists_update(m),
            Msg::Playlist(m) => return self.playlist_update(m),
            Msg::SearchPage(m) => return self.search_update(m),
            Msg::Radios(m) => return self.radios_update(m),
            Msg::Podcasts(m) => return self.podcasts_update(m),
            Msg::Show(m) => return self.show_update(m),
            Msg::Downloads(m) => return self.downloads_update(m),
            Msg::Settings(m) => return self.settings_update(m),
            Msg::AmpRemote(m) => return self.amp_remote_update(m),
            Msg::Preamp(v) => return self.settings_preamp(v, false),
            Msg::ClosePopin => {
                // L'éditeur de tags refuse de se fermer sur des changements.
                if self.views.popin.as_ref().is_some_and(|p| p.tag_edit().is_some()) {
                    return self.tag_editor_update(crate::models::tags::TagEditorMsg::Close);
                }
                if self.views.popin.as_ref().is_some_and(|p| p.batch_edit().is_some()) {
                    return self.batch_editor_update(crate::models::tags::BatchEditorMsg::Close);
                }
                self.views.popin = None;
            }
            Msg::PopinConfirm => return self.popin_confirm(),
            Msg::ToastClose(id) => return self.toast_remove(id),
            Msg::ImportName(v) => self.views.import_name = v,
            Msg::Track(m) => return self.track_update(m),
            Msg::Tags(m) => return self.tags_update(m),
            Msg::QueueClear => return self.queue_clear(),
            Msg::TrackMenu(m) => return self.track_menu_update(m),
            Msg::CollOpen(at, title, source) => return self.collection_menu_open(at, title, source),
            Msg::CollectionMenu(m) => return self.collection_menu_update(m),
            Msg::PodcastMenu(m) => return self.podcast_menu_update(m),
            Msg::PlaylistMenu(m) => return self.playlist_menu_update(m),
            Msg::Sidebar(m) => return self.sidebar_update(m),
            Msg::LyricsRefresh => {
                let Some(path) = self.views.lyrics.path.clone() else { return false };
                self.lyrics_fetch(path, true);
            }
            Msg::LyricsRetry => {
                let Some(path) = self.views.lyrics.path.clone() else { return false };
                self.lyrics_fetch(path, false);
            }
            Msg::BatchCancel => return self.batch_cancel(),
            Msg::PlaylistForm(m) => return self.playlist_form_update(m),
            Msg::WhatsNew(m) => return self.whats_new(m),
            Msg::Smart(m) => return self.smart_update(m),
            Msg::Profiles(m) => return self.profiles_update(m),
            Msg::Typed(m) => return self.typed_update(m),
            Msg::ServerForm(m) => return self.server_form_update(m),
            Msg::Dlna(m) => return self.dlna_update(m),
            Msg::PodcastPreview(m) => return self.podcast_preview_update(m),
            Msg::Device(m) => return self.device_update(m),
            Msg::TagEditor(m) => return self.tag_editor_update(m),
            Msg::Lookup(m) => return self.lookup_update(m),
            Msg::Cover(m) => return self.cover_search_update(m),
            Msg::LibraryForm(m) => return self.library_form_update(m),
            Msg::Selection(m) => return self.selection_update(m),
            Msg::Banner(m) => return self.banners_update(m),
            Msg::Columns(m) => return self.columns_update(m),
            Msg::LibraryFolders(m) => return self.library_folders_update(m),
            Msg::ArtistCheck(m) => return self.artist_check_update(m),
            Msg::Clean(m) => return self.clean_update(m),
            Msg::History(m) => return self.history_update(m),
            Msg::Rename(m) => return self.rename_update(m),
            Msg::BatchEditor(m) => return self.batch_editor_update(m),
            Msg::Source(m) => return self.source_update(m),
            Msg::BatchDismiss => self.views.batch = None,
            Msg::CoverZoom(open) => return self.cover_zoom(open),
            Msg::CoverMenu(at) => return self.cover_menu(at),
            Msg::CoverCopy => return self.cover_copy(),
            Msg::ChainMenu => return self.chain_menu(),
            Msg::CopyText(text) => return self.copy_text(text),
            Msg::QueueRemove(id) => return self.queue_remove(&id),
            Msg::QueueScrolled(offset, max) => {
                let len = crate::mapper::player::queue::upcoming_total(&self.data.queue);
                return self.views.queue_window.scrolled(offset, max, len, self.export);
            }
            Msg::PreampSet(v) => return self.settings_preamp(v, true),
            Msg::PlayArtist(id) => self.play_artist(id),
            Msg::Slide(id, forward, count) => return self.carousel_slide(id, forward, count),
            Msg::CarouselScrolled(id, geometry) => return self.carousel_scrolled(id, geometry),
            Msg::EnqueueArtist(id) => self.enqueue_artist(id),
            Msg::PlayComposer(name) => self.composer_tracks(name, false),
            Msg::EnqueueComposer(name) => self.composer_tracks(name, true),
            Msg::Artists(m) => return self.artists_update(m),
            Msg::Row(m) => return self.row_update(m),
            Msg::Search(value) => {
                self.search = value;
                self.do_search();
            }
            Msg::SearchKey(key) if self.open(Menu::Search) => {
                // Lignes affichées (bornées par section), pas tous les résultats reçus.
                let n = crate::mapper::search::search_autocomplete::results(&self.data).len();
                match key {
                    Key::Down => self.suggestion = Some(self.suggestion.map_or(0, |i| (i + 1).min(n.saturating_sub(1)))),
                    Key::Up => self.suggestion = self.suggestion.and_then(|i| i.checked_sub(1)),
                    Key::Enter => match self.suggestion {
                        Some(i) => return self.update(Msg::SearchPick(i)),
                        None => return self.update(Msg::SearchAll),
                    },
                    _ => return false,
                }
            }
            Msg::SearchKey(_) => return false,
            Msg::SearchPick(i) => {
                self.menu = None;
                self.suggestion = None;
                let vm = ViewModel::build(&self.data, self.route(), self.source, Utc::now());
                if let Some(r) = vm.results.get(i) {
                    if let Some(route) = r.route.clone() {
                        self.history.push(route);
                    } else if let Some(path) = r.path.clone() {
                        return self.update(Msg::PlayPath(path));
                    }
                }
            }
            Msg::SearchAll => {
                self.menu = None;
                self.history.push(Route::Search(self.search.trim().to_string()));
            }

            other => return self.handle_player(other),
        }
        true
    }
}
