//! Clés de mémoire (`memo={…}`) des grandes zones stables de la fenêtre : barre latérale,
//! lecteur, recherche et actions de la barre de titre. Une reconstruction de la vue (un
//! lot de morceaux qui arrive, un message quelconque) reprend ces zones telles quelles.
//!
//! Chaque clé doit couvrir **tout** ce que la zone lit, gestionnaires de clic compris :
//! une donnée oubliée ici laisserait la zone périmée à l'écran. Leur forme `Debug` suffit,
//! ces zones lisent peu de données. Réglages en entier : les zones en lisent une vingtaine
//! et ils changent rarement. Pochettes par leur identité dans le cache : la zone est
//! refaite quand l'image arrive.

use std::fmt::Write;

use crate::app::music_app::MusicApp;
use crate::app::resources::assets::Assets;
use crate::models::sidebar::PinKind;
use crate::models::home::ViewModel;

impl MusicApp {
    /// Ce qui compte pour toutes les zones : langue, thème, réglages, menu ouvert.
    fn region_key(&self) -> String {
        format!(
            "{:?}{:?}{:?}{:?}",
            karbon_ui::intl::locale(),
            self.dark,
            self.menu,
            self.data.settings
        )
    }

    /// Identités des pochettes dans le cache, grandes et vignettes.
    fn covers_key<'a>(assets: &Assets, out: &mut String, paths: impl Iterator<Item = &'a Option<String>>) {
        for path in paths {
            let _ = write!(out, ",{},{}", assets.cover_id(path), assets.cover_small_id(path));
        }
    }

    /// `sidebar/Sidebar.ui` et ses menus.
    pub fn sidebar_key(&self, vm: &ViewModel, assets: &Assets) -> String {
        let mut key = self.region_key();
        let sidebar = &self.views.sidebar;
        let tags = self.data.player.audio.as_ref().map(|a| (&a.tags.album, &a.tags.album_artist, &a.tags.artist));
        let _ = write!(
            key,
            "|{:?}{:?}{:?}{:?}{:?}{:?}",
            self.route(),
            self.sidebar_collapsed,
            self.footer_collapsed,
            self.source,
            self.data.library_id,
            self.data.tabs_placement(),
        );
        let _ = write!(
            key,
            "|{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
            sidebar.pins_library, sidebar.pins, sidebar.place, sidebar.organize, sidebar.item_menu, self.sidebar_podcast_favs(), tags, self.data.player.path, self.data.player.status,
        );
        let _ = write!(
            key,
            "|{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
            vm.sidebar_playlists,
            vm.tabs,
            vm.shortcuts,
            vm.scan,
            vm.first_name,
            vm.initial,
            vm.tiles,
            vm.sources,
            vm.current_source,
            vm.libraries,
            vm.library,
            vm.current_library,
            vm.open_buttons,
            vm.podcast_covers,
            vm.podcasts_detail,
            (&vm.radios_detail, &vm.radio_logo),
        );
        // Avatar du profil actif (pied de la barre) : son nom, et si son image est chargée.
        let _ = write!(key, "|{:?}", vm.profile_avatar);
        // Menu « Gérer » ouvert par-dessus la liste des bibliothèques : sans changer
        // `self.menu` (la liste reste ouverte), sa bibliothèque et ses dossiers lus.
        let _ = write!(key, "|{:?}{:?}", self.views.library_manage, self.views.library_manage_nested);
        Self::covers_key(assets, &mut key, std::iter::once(&vm.profile_avatar));
        let pins: Vec<Option<String>> = [PinKind::Album, PinKind::Artist]
            .into_iter()
            .flat_map(|kind| self.sidebar_pins(kind))
            .map(|pin| pin.cover.clone())
            .collect();
        let podcasts: Vec<Option<String>> = self.sidebar_podcast_favs().into_iter().map(|p| p.cover).collect();
        let covers: Vec<Option<String>> = vm
            .podcast_covers
            .iter()
            .chain(vm.sidebar_playlists.iter().flat_map(|p| &p.covers))
            .cloned()
            .map(Some)
            .collect();
        Self::covers_key(
            assets,
            &mut key,
            pins.iter()
                .chain(&podcasts)
                .chain(&covers)
                .chain(std::iter::once(&vm.radio_logo)),
        );
        key
    }

    /// `player/Player.ui` : le temps de lecture y est, la zone suit donc la lecture.
    pub fn player_key(&self, vm: &ViewModel, assets: &Assets) -> String {
        let mut key = self.region_key();
        let queue = &self.data.queue;
        let _ = write!(
            key,
            "|{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
            self.data.player.pipeline,
            vm.details,
            vm.statut,
            vm.output,
            vm.outputs,
            vm.exclusive,
            vm.dop,
            vm.liked,
            vm.network,
            self.data.volume,
            (&queue.repeat_mode, &queue.is_shuffled),
            self.data.player.position.to_bits(),
            self.data.player.status,
        );
        Self::covers_key(assets, &mut key, std::iter::once(&vm.details.cover_path));
        key
    }

    /// `search/SearchAutocomplete.ui`.
    pub fn search_key(&self, vm: &ViewModel, assets: &Assets) -> String {
        let mut key = self.region_key();
        let _ = write!(key, "|{:?}{:?}{:?}", self.search, self.suggestion, vm.results);
        Self::covers_key(assets, &mut key, vm.results.iter().map(|r| &r.cover_path));
        key
    }

    /// Actions à droite de la barre de titre : minuteur, nouveautés, thème, profil (son
    /// image, et si elle est chargée).
    pub fn title_actions_key(&self, vm: &ViewModel, assets: &Assets) -> String {
        let mut key = self.region_key();
        let _ = write!(
            key,
            "|{:?}{:?}{:?}{:?}{:?}{:?}",
            self.sleep_timer,
            self.sleep_input,
            self.sleep_remaining(),
            self.views.whats_new.viewed,
            vm.initial,
            vm.first_name,
        );
        Self::covers_key(assets, &mut key, std::iter::once(&vm.profile_avatar));
        key
    }
}
