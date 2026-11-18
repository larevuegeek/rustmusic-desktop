//! `impl Application` : chargement, vue, messages, tâches et fenêtre.

use chrono::Utc;
use karbon_ui::{
    application::Resources,
    task::{TaskCommand, TaskResult},
    ui::prelude::*,
    ui::runtime::Key,
    window::WindowCommand,
};
use crate::app::resources::assets::Assets;
use karbon_ui::task::Inbox;
use crate::helper::color::hex;
use crate::helper::string::string::hue;
use crate::helper::string::string::initial;
use crate::helper::covers::mosaic;
use karbon_ui::intl::t_count;
// Tous les composants des gabarits (`<PlayButton>`, `<SidebarRow>`…).
use crate::component::*;
use crate::app::resources::css;
use crate::models::app::Menu;
use crate::models::app::SleepTimer;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::app::Section;
use crate::models::app::SettingsPage;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Choice;
use crate::models::app::Field;
use crate::models::library::LibraryMsg;
use crate::mapper::tracks::debug_key;
use crate::models::tracks::RowMsg;
use crate::models::tracks::Tier;
use crate::models::tracks::Widget;
use crate::models::albums::AlbumMsg;
use crate::models::artists::ArtistMsg;
use crate::models::genres::GenreMsg;
use crate::models::genres::GenresMsg;
use crate::models::years::YearsMsg;
use crate::models::folders::FoldersMsg;
use crate::models::amp_remote::{AmpRemoteMsg, AmpTab};
use crate::core::amp_remote::model::{Command, NavKey, Transport, Zone};
use crate::mapper::amp_remote::amp_remote_view::{wheel_nav, wheel_volume};
use karbon_ui::ui::drag_drop::PanEvent;
use crate::models::mixes::MixesMsg;
use crate::models::playlists::PlaylistsMsg;
use crate::models::playlists::PlaylistMsg;
use crate::models::search::SearchMsg;
use crate::models::radios::RadiosMsg;
use crate::models::podcasts::PodcastsMsg;
use crate::models::podcasts::ShowMsg;
use crate::models::podcasts::DownloadsMsg;
use crate::mapper::settings::note_color;
use crate::models::settings::Card;
use crate::models::settings::Ghost;
use crate::models::settings::Pill;
use crate::mapper::settings::about_support;
use crate::mapper::settings::badge_keys as guide_badge_keys;
use crate::mapper::settings::guide_badges;
use crate::mapper::settings::guide_terms;
use crate::mapper::settings::guide_modes;
use crate::mapper::tags::issue_label;
use crate::mapper::settings::amp_line;
use crate::mapper::settings::{active_icon, version_line, version_tag};
use crate::models::settings::ScrobbleMsg;
use crate::models::tracks::TrackMsg;
use crate::models::tracks::TrackMenuAt;
use crate::models::tracks::TrackMenuMsg;
use crate::models::library::CollectionMenuMsg;
use crate::models::library::CollectionSource;
use crate::models::podcasts::PodcastMenuMsg;
use crate::models::playlists::PlaylistMenuMsg;
use crate::models::sidebar::ItemMsg;
use crate::models::sidebar::PinKind;
use crate::models::sidebar::RowTarget;
use crate::models::sidebar::SidebarMsg;
use crate::models::sidebar::Section as SbSection;
use crate::mapper::sidebar::DRAG_PLAYLISTS;
use crate::mapper::sidebar::DRAG_SECTIONS;
use crate::mapper::playlists::playlist_form::colors as playlist_form_colors;
use crate::mapper::playlists::playlist_form::icon_label as playlist_form_icon_label;
use crate::mapper::playlists::playlist_form::icons as playlist_form_icons;
use crate::mapper::playlists::playlist_form::preview as playlist_form_preview;
use crate::models::playlists::PlaylistFormMsg;
use crate::mapper::updates::update_whats_new::art_background as whats_new_art;
use crate::mapper::updates::update_whats_new::art_blob as whats_new_blob;
use crate::mapper::updates::update_whats_new::art_shadows as whats_new_shadows;
use crate::mapper::updates::update_whats_new::art_spot as whats_new_spot;
use crate::mapper::updates::update_whats_new::logo_rows as whats_new_logo_rows;
use crate::mapper::updates::update_whats_new::next_background as whats_new_next;
use crate::mapper::updates::update_whats_new::track as whats_new_track;
use crate::models::updates::WhatsNewMsg;
use crate::mapper::updates::update_whats_new::VERSION as WN_VERSION;
use crate::mapper::playlists::playlist_smart::cta as smart_cta;
use crate::mapper::playlists::playlist_smart::presets as smart_presets;
use crate::mapper::playlists::playlist_smart::sizes as smart_sizes;
use crate::mapper::playlists::playlist_smart::tile_background as smart_tile;
use crate::models::playlists::SmartMsg;
use crate::mapper::profiles::profile_list::can_delete as profile_can_delete;
use crate::mapper::profiles::profile_list::initials as profile_initials;
use crate::mapper::profiles::profile_list::is_admin as profile_is_admin;
use crate::mapper::profiles::profile_list::palette as profile_palette;
use crate::mapper::profiles::profile_list::rule as profile_rule;
use crate::models::profiles::ProfileMode;
use crate::models::profiles::ProfileMsg;
use crate::models::profiles::ProfileScreen;
use crate::mapper::layout::layout_typed_confirm::word as typed_word;
use crate::models::layout::TypedMsg;
use crate::models::servers::DlnaMsg;
use crate::mapper::servers::found_detail as server_found_detail;
use crate::mapper::servers::kinds as server_kinds;
use crate::models::servers::ServerFormMsg;
use crate::mapper::podcasts::day_month as podcast_day_month;
use crate::models::podcasts::PodcastPreviewMsg;
use crate::mapper::podcasts::episode_date;
use crate::mapper::podcasts::episode_duration;
use crate::mapper::settings::failed as device_failed;
use crate::models::settings::Banner as DeviceBanner;
use crate::models::settings::DeviceMsg;
use crate::mapper::tags::changes_label as lookup_changes;
use crate::models::tags::LookupMsg;
use crate::mapper::tags::actions as tag_editor_actions;
use crate::mapper::tags::{field_label, tag_label};
use crate::mapper::tags::summary as tag_editor_summary;
use crate::mapper::tags::type_label as tag_editor_type;
use crate::models::tags::TagEditorMsg;
use crate::models::albums::CoverMsg;
use crate::models::library::LibraryFormMsg;
use crate::models::tracks::SelectionMsg;
use crate::models::layout::BannerMsg;
use crate::models::layout::Update;
use crate::models::tracks::ColumnsMsg;
use crate::models::library::LibraryFoldersMsg;
use crate::mapper::artists::artist_check::avatar as artist_avatar;
use crate::mapper::artists::artist_check::source_icon as artist_source_icon;
use crate::mapper::artists::artist_check::source_label as artist_source;
use crate::mapper::artists::artist_check::truncated as artist_truncated;
use crate::mapper::artists::artist_check::{pile_more, pile_shown};
use crate::models::artists::ArtistCheckMsg;
use crate::models::artists::RuleView as ArtistRule;
use crate::models::tags::CleanMsg;
use crate::models::tags::HistoryMsg;
use crate::mapper::tags::workshop_action;
use crate::mapper::tags::first_preset as rename_presets_first;
use crate::mapper::tags::pattern_fields as rename_fields;
use crate::models::tags::RenameMsg;
use crate::models::tags::BatchEditorMsg;
use crate::models::tags::BatchEditor;
use crate::models::tags::SourceMsg;
use crate::mapper::tags::plural as src_plural;
use crate::mapper::layout::layout_batch::failures_label as batch_failures_label;
use crate::mapper::layout::layout_batch::file_name as batch_file_name;
use crate::mapper::player::lyrics::lyrics_fallback;
use crate::mapper::player::lyrics::source_label as lyrics_source_label;
use crate::models::player::LyricsStatus;
use crate::models::player::MiniMsg;
use crate::models::player::MiniTab;
use crate::models::player::StepTone;
use crate::mapper::radios::variant_label as radio_variant_label;
use karbon_ui::ui::popover::ContextMenuRequest;
use karbon_ui::ui::scroll::ScrollMetrics;
use crate::models::shared::CarouselGeometry;
use crate::controller::tags::tracks_label as tags_tracks_label;
use crate::models::tags::TagsMsg;
use crate::mapper::settings::caps_of as shortcut_caps;
use crate::mapper::settings::describe as server_describe;
use crate::mapper::settings::quality_labels as server_quality_labels;
use crate::mapper::settings::quality_of as server_quality_of;
use crate::mapper::settings::synced_label as server_synced;
use crate::models::settings::ServerMsg;
use crate::mapper::settings::help_split;
use crate::models::settings::SettingsMsg;
use crate::models::artists::ArtistsMsg;
use crate::models::albums::AlbumsMsg;
use crate::models::albums::ViewMode;
use crate::mapper::albums::NO_GENRE;
use crate::models::tracks::TracksMsg;
use crate::models::tracks::PageKind;
use crate::models::home::PlaylistRow;
use crate::models::home::ViewModel;
use karbon_ui::ui::drag_drop::DropEvent;
use super::*;

impl Application for MusicApp {
    type Message = Msg;

    fn fonts(&self) -> &[FontAsset] {
        &crate::app::resources::assets::FONTS
    }

    fn load(&mut self, resources: &dyn Resources) -> karbon_ui::Result<()> {
        self.assets = Some(Assets::load(resources)?);
        self.start();
        if self.export {
            self.settle(std::time::Duration::from_secs(30));
            if let Some(route) = self.export_route.take() {
                self.history.push(route);
                self.load_page();
                self.settle(std::time::Duration::from_secs(20));
            }
            for msg in std::mem::take(&mut self.export_msgs) {
                self.update(msg);
                self.settle(std::time::Duration::from_secs(20));
            }
            if let Some(query) = self.export_search.take() {
                self.update(Msg::Search(query));
                self.settle(std::time::Duration::from_secs(10));
            }
            if let Some(id) = self.export_popup.take() {
                self.menu = Some(Menu::Popup(id));
            }
            if let Some(menu) = self.export_open.take() {
                self.menu = Some(menu);
                if menu == Menu::Output {
                    self.settle(std::time::Duration::from_secs(5));
                }
            }
        }
        Ok(())
    }

    fn color_preferences(&self) -> ColorPreferences {
        // `light`, `dark` ou `auto` (le système), `dark` par défaut ;
        // contraste `normal` par défaut.
        ColorPreferences {
            scheme: match self.data.setting("theme") {
                Some("light") => ColorScheme::Light,
                Some("auto") => ColorScheme::System,
                _ => ColorScheme::Dark,
            },
            high_contrast: self.data.setting("contrast") == Some("high"),
        }
    }

    fn view(&self, context: &ViewContext) -> Element {
        self.viewport.set((context.width, context.height));
        let assets = self.assets.as_ref().expect("Application::load precedes view");
        let (dark, contrast) = color_scheme();
        self.dark.set(dark);
        let vm = ViewModel::build(&self.data, self.route(), self.source, Utc::now());
        // Variantes du thème (voir assets/css/theme-modes.css) ; l'état de référence (sombre,
        // contraste élevé) garde les valeurs de `:root` compilées.
        let variables = (!(dark && contrast)).then(|| self.theme_variables(assets, &vm, dark, contrast));
        let cx = TemplateContext::new(assets, "rustmusic");
        let cx = match &variables {
            Some(variables) => cx.with_variables(variables),
            None => cx,
        };
        self.covers_due.set(true);
        let volume = (f64::from(self.data.volume) / 100.) as f32;
        let muted = self.data.volume == 0;
        let full_page = self.route().is_full_page();
        let route_title = self.route().title();
        let progress = if vm.details.duration > 0. {
            (self.data.player.position / vm.details.duration * 100.).clamp(0., 100.) as f32
        } else {
            0.
        };
        // En mode mini, la fenêtre n'affiche que le mini ou le micro-lecteur.
        if self.views.mini.active {
            return ui_file!("/ui/templates/player/mini/Mini.ui", context = cx, styles = css);
        }
        ui_file!("/ui/templates/layout/Layout.ui", context = cx, styles = css)
    }

    fn update(&mut self, message: Msg) -> bool {
        let before = self.history.current().clone();
        let changed = self.handle(message);
        self.tray_sync();
        if *self.history.current() != before {
            self.views.carousels.clear();
            self.views.carousel_view += 1;
            self.load_page();
            return true;
        }
        changed
    }

    fn take_task_commands(&mut self) -> Vec<TaskCommand> {
        self.task_commands()
    }

    fn task_completed(&mut self, result: TaskResult) -> bool {
        let changed = self.on_task(result);
        self.tray_sync();
        // Fenêtre ouverte sur une page (`--page tracks`, mesures de fluidité à la main) :
        // la page s'ouvre une fois la bibliothèque connue, comme après un clic.
        if !self.export && self.data.library_id.is_some() {
            if let Some(route) = self.export_route.take() {
                return self.update(Msg::Go(route)) || changed;
            }
        }
        changed
    }

    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.window_commands)
    }

    fn has_frame_work(&self) -> bool {
        self.tracks_has_frame_work() || self.lists_growing()
    }

    fn frame_work(&mut self) -> bool {
        let tracks = self.tracks_frame_work();
        let lists = self.lists_frame_work();
        tracks || lists
    }

    fn window_changed(&mut self, state: karbon_ui::window::WindowState) -> bool {
        self.mini_window(state)
    }

    fn tray_event(&mut self, event: karbon_ui::window::TrayEvent) -> bool {
        self.tray_handle(event)
    }

    /// Le chargeur des pochettes : karbon-ui ordonnance ses tâches et lui livre leurs
    /// résultats.
    fn image_loader(&self) -> Option<karbon_ui::resources::ImageLoader> {
        self.assets.as_ref().map(|a| a.covers.clone())
    }
    /// Hors aperçu (qui attend ses données au chargement) et hors sortie.
    fn inbox(&self) -> Option<Inbox<Self>> {
        (!self.export && !self.exiting).then(|| self.inbox.clone())
    }
}
