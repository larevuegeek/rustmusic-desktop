//! L'en-tête des pages de bibliothèque : ce qu'affichent ses gabarits, bâti depuis l'état de la page,
//! et les pochettes à charger.

use crate::models::library::*;
use chrono::Utc;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use crate::helper::date::elapsed_seconds;
use karbon_ui::intl::{t, t_with};
use crate::models::app::Menu;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::app::Section;
use crate::entity::library::library::Library;
use crate::entity::library::library_dirs::LibraryDir;

impl MusicApp {
    /// L'en-tête de la bibliothèque ouverte, pour la section de la route courante.
    pub fn library_header(&self) -> Option<LibraryHeader> {
        let library = self.data.library()?;
        let section = self.route().section();
        let key = section.map(|s| s.key()).unwrap_or("");
        let label = section.map(Section::label);
        let kicker = match label {
            Some(l) => format!("{} · {l}", t("library_head.kicker")),
            None => t("library_head.kicker"),
        };
        let icon = match key {
            "genres" => "material-symbols:sell-outline-rounded",
            "folders" => "material-symbols:folder-open-outline-rounded",
            _ => "material-symbols:library-music-outline-rounded",
        };
        let count = |n: i64, one: &str, many: &str| (intl::number(n), t(if n == 1 { one } else { many }));
        let albums = (library.total_albums, count(library.total_albums, "library_head.albums_one", "library_head.albums_n"));
        let artists = (library.total_artists, count(library.total_artists, "library_head.artists_one", "library_head.artists_n"));
        let tracks = (library.total_tracks, count(library.total_tracks, "library_head.tracks_one", "library_head.tracks_n"));
        let order = match key {
            "tracks" => vec![tracks, albums, artists],
            "artists" => vec![artists, albums, tracks],
            _ => vec![albums, artists, tracks],
        };
        // La page peut donner ses chiffres et son action principale.
        let page_head = self.page_head();
        let digits = match &page_head {
            Some((ch, _)) => ch.iter().filter(|(n, ..)| *n > 0).map(|(n, one, many)| count(*n, one, many)).collect(),
            None => order.into_iter().filter(|(n, _)| *n > 0).map(|(_, c)| c).collect(),
        };
        let dirs = &self.data.library_dirs;
        let size: i64 = dirs.iter().map(|d| d.total_size).sum();
        let extra = if page_head.is_some() {
            None
        } else if key == "tracks" {
            self.data
                .stats
                .as_ref()
                .filter(|s| s.total_duration_sec > 0.)
                .map(|s| (intl::duration(s.total_duration_sec), t("library_head.listening")))
        } else if size > 0 {
            let text = karbon_ui::intl::bytes(size as u64);
            text.rsplit_once(' ').map(|(a, b)| (a.to_string(), b.to_string()))
        } else {
            None
        };
        let last_scan = dirs.iter().filter_map(|d| d.last_scan_at).max();
        let scan_ago = last_scan.map(|d| {
            let ago = intl::relative_time_short(elapsed_seconds(&d.to_rfc3339(), Utc::now()).unwrap_or(0));
            t_with("library_head.scan_ago", &[("t", ago.as_str())])
        });
        let libraries: Vec<_> = self
            .data
            .libraries
            .iter()
            .map(|l| (l.id, l.name.clone(), intl::number(l.total_tracks), l.id == library.id))
            .collect();
        let (primary_icon, primary_label) = match (self.route(), page_head.as_ref().and_then(|(_, a)| *a)) {
            (_, Some((icon, key))) => (icon, t(key)),
            (Route::Library(Section::Tracks), _) => ("material-symbols:play-arrow-rounded", t("library_head.play_all")),
            _ => ("material-symbols:add-rounded", t("library_head.add_folder")),
        };
        let state = if self.data.scan.is_some() || dirs.iter().any(|d| d.scan_status == "scanning") {
            "scanning"
        } else if dirs.iter().any(|d| d.scan_status == "error") {
            "error"
        } else {
            "ok"
        };
        Some(LibraryHeader {
            id: library.id,
            name: library.name.clone(),
            kicker,
            icon,
            digits,
            extra,
            scan_ago,
            many_libraries: libraries.len() > 1,
            libraries,
            primary_icon,
            primary_label,
            state,
        })
    }

    /// Chiffres (nombre, singulier, pluriel) et action (icône, libellé) propres à la page.
    #[allow(clippy::type_complexity)]
    pub(crate) fn page_head(&self) -> Option<(Vec<(i64, &'static str, &'static str)>, Option<(&'static str, &'static str)>)> {
        let library = self.data.library()?;
        match self.route() {
            Route::Library(Section::Genres) => {
                let without = self.data.albums.iter().filter(|a| a.genre.is_none()).count() as i64;
                let tracks_count = if library.total_tracks > 0 {
                    library.total_tracks
                } else {
                    self.data.genres.iter().map(|g| g.total_tracks).sum()
                };
                Some((
                    vec![
                        (self.data.genres.len() as i64, "library_head.genres_one", "library_head.genres_n"),
                        (library.total_albums, "library_head.albums_one", "library_head.albums_n"),
                        (tracks_count, "library_head.tracks_one", "library_head.tracks_n"),
                        (without, "library_head.untagged_one", "library_head.untagged_n"),
                    ],
                    Some(("material-symbols:shuffle-rounded", "genres_view.random_mix")),
                ))
            }
            Route::Library(Section::Mixes) => {
                let v = self.mixes_view();
                let total = v.for_you.len() + v.custom.len() + v.genres.len() + v.decades.len();
                Some((
                    vec![
                        (total as i64, "library_head.mixes_one", "library_head.mixes_n"),
                        (v.custom.len() as i64, "library_head.own_mixes_one", "library_head.own_mixes_n"),
                    ],
                    Some(("material-symbols:add-rounded", "mix_view.create")),
                ))
            }
            Route::Library(Section::Folders) => {
                let dirs = &self.data.library_dirs;
                Some((
                    vec![
                        (dirs.len() as i64, "library_head.folders_one", "library_head.folders_n"),
                        (dirs.iter().map(|d| d.total_files).sum(), "library_head.files_one", "library_head.files_n"),
                        (library.total_tracks, "library_head.tracks_one", "library_head.tracks_n"),
                    ],
                    None,
                ))
            }
            Route::Library(Section::Years) => {
                let dates = self.data.albums.iter().filter(|a| a.year.is_some_and(|y| y >= 1900)).count() as i64;
                let decades: std::collections::HashSet<i32> = self.data.albums.iter().filter_map(|a| a.year.filter(|y| *y >= 1900)).map(|y| y / 10).collect();
                Some((
                    vec![
                        (decades.len() as i64, "library_head.decades_one", "library_head.decades_n"),
                        (dates, "library_head.albums_one", "library_head.albums_n"),
                        (self.data.albums.len() as i64 - dates, "library_head.undated_one", "library_head.undated_n"),
                    ],
                    Some(("material-symbols:shuffle-rounded", "genres_view.random_mix")),
                ))
            }
            _ => None,
        }
    }

    /// Le menu « Gérer » de la bibliothèque `id` : celle qui est ouverte, ou celle de la
    /// liste de la barre latérale dont les dossiers ont été lus.
    pub fn manage_menu(&self, id: i64) -> Option<ManageMenu> {
        let library = self.data.libraries.iter().find(|l| l.id == id)?;
        let open = self.data.library_id == Some(id);
        let dirs: &[LibraryDir] = match &self.views.library_manage {
            _ if open => &self.data.library_dirs,
            Some((target, dirs)) if *target == id => dirs,
            _ => &[],
        };
        let size: i64 = dirs.iter().map(|d| d.total_size).sum();
        let state = if (open && self.data.scan.is_some()) || dirs.iter().any(|d| d.scan_status == "scanning") {
            "scanning"
        } else if dirs.iter().any(|d| d.scan_status == "error") {
            "error"
        } else {
            "ok"
        };
        let dirs_line = match dirs.len() {
            0 => String::new(),
            1 => dirs[0].path.clone(),
            n => t_with("library_head.dirs_n", &[("n", &n.to_string())]),
        };
        Some(ManageMenu {
            id,
            name: library.name.clone(),
            remote: library.server_id.is_some(),
            server_name: library.server_name.clone().unwrap_or_default(),
            dirs_line,
            state,
            groups: self.manage_actions(library, open, dirs, size),
        })
    }

    fn manage_actions(&self, library: &Library, open: bool, dirs: &[LibraryDir], size: i64) -> Vec<Vec<ManageAction>> {
        let id = library.id;
        let remote = library.server_id.is_some();
        let action = |icon, title: String, sub: String, msg: LibraryMsg| ManageAction {
            icon,
            title,
            sub,
            msg: Msg::Library(msg),
            arrow: false,
            count: None,
            alert: false,
            busy: false,
        };
        let scanning = open && self.data.scan.is_some();
        let mut first = Vec::new();
        if remote {
            let mut sync = action("material-symbols:sync-rounded", t("servers.sync"), t("servers.sync_desc"), LibraryMsg::Rescan(id));
            sync.busy = scanning;
            first.push(sync);
            let mut manage = action(
                "material-symbols:cloud-outline-rounded",
                t("servers.manage"),
                library.server_name.clone().unwrap_or_default(),
                LibraryMsg::ImportExport,
            );
            manage.arrow = true;
            first.push(manage);
        } else {
            let mut rescan = action("material-symbols:sync-rounded", t("library_head.rescan"), t("library_head.rescan_desc"), LibraryMsg::Rescan(id));
            rescan.busy = scanning;
            first.push(rescan);
            let dirs_text = if dirs.len() == 1 {
                t("library_head.dirs_one")
            } else {
                t_with("library_head.dirs_n", &[("n", &dirs.len().to_string())])
            };
            let sub = [dirs_text, if size > 0 { karbon_ui::intl::bytes(size as u64) } else { String::new() }]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            let mut folders = action("material-symbols:folder-open-outline-rounded", t("library_head.folders"), sub, LibraryMsg::Folders(id));
            folders.arrow = true;
            first.push(folders);
            first.push(action(
                "material-symbols:upload-file-outline-rounded",
                t("library.add_files"),
                t("library_head.add_files_desc"),
                LibraryMsg::AddFiles(id),
            ));
            // Les albums chargés sont ceux de la bibliothèque ouverte : ailleurs, pas de compte.
            let missing = if open { self.data.albums.iter().filter(|a| a.cover_url.is_none()).count() } else { 0 };
            let covers_sub = match missing {
                0 => t("library_head.covers_ok"),
                1 => t("library_head.covers_missing_one"),
                n => t_with("library_head.covers_missing_n", &[("n", &intl::number(n as i64))]),
            };
            let mut covers = action("material-symbols:image-search-rounded", t("library_head.covers"), covers_sub, LibraryMsg::Covers(id));
            if missing > 0 {
                covers.count = Some(intl::number(missing as i64));
                covers.alert = true;
            }
            first.push(covers);
            first.push(action(
                "material-symbols:person-outline-rounded",
                t("library_head.artist_images"),
                t("library_head.artist_images_desc"),
                LibraryMsg::Portraits,
            ));
            first.push(action(
                "material-symbols:group-search-rounded",
                t("artist_split.check_now"),
                t("artist_split.menu_desc"),
                LibraryMsg::CheckArtists,
            ));
            let mut io = action(
                "material-symbols:swap-vert-rounded",
                t("library_head.import_export"),
                t("library_head.import_export_desc"),
                LibraryMsg::ImportExport,
            );
            io.arrow = true;
            first.push(io);
        }
        let mut second = vec![action(
            "material-symbols:edit-outline-rounded",
            t("selector.rename"),
            t("selector.rename_desc"),
            LibraryMsg::Rename(id),
        )];
        if !library.is_default {
            second.push(action(
                "material-symbols:keep-outline-rounded",
                t("selector.set_default"),
                t("selector.default_hint"),
                LibraryMsg::MakeDefault(id),
            ));
        }
        vec![first, second]
    }

    /// Le menu « Gérer » ouvert depuis la liste de la barre latérale.
    pub fn sidebar_manage_menu(&self) -> Option<ManageMenu> {
        let (id, _) = self.views.library_manage.as_ref()?;
        self.popup(crate::controller::library::library_update::LIBRARY_MANAGE).then(|| self.manage_menu(*id)).flatten()
    }

    /// La bibliothèque ouverte (0 sans bibliothèque : ses actions ne font rien).
    pub fn library_id(&self) -> i64 {
        self.data.library_id.unwrap_or_default()
    }

    /// Le menu `Menu::Popup(id)` est ouvert.
    pub fn popup(&self, id: &str) -> bool {
        matches!(self.menu, Some(Menu::Popup(m)) if m == id)
    }
}
