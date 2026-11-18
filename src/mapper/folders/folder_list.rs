//! L'explorateur de dossiers : ce qu'affichent ses gabarits, bâti depuis l'état de la page.

use crate::models::folders::*;
use crate::models::app::Choice;
use crate::models::app::Msg;
use crate::models::tracks::Tier;
use crate::entity::library::dir_entry::DirEntry;
use chrono::Utc;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use crate::helper::date::elapsed_seconds;
use karbon_ui::intl::{t, t_with};
use crate::mapper::albums::natural_cmp;
use crate::mapper::albums::sort_key;
use crate::models::albums::AlbumsMsg;
use crate::models::albums::ViewMode;
use crate::mapper::tracks::short_format;
use crate::mapper::tracks::tier;
use crate::entity::library::library_dirs::LibraryDir;
use crate::app::resources::assets::{small_key, Assets};

pub const SORTS: [(&str, &str, &str, bool); 4] = [
    ("name", "folders_view.sort_name", "material-symbols:sort-by-alpha-rounded", false),
    ("files", "folders_view.sort_files", "material-symbols:description-outline-rounded", true),
    ("size", "folders_view.sort_size", "material-symbols:hard-drive-outline-rounded", false),
    ("scan", "folders_view.sort_scan", "material-symbols:sync-rounded", true),
];

/// Sans profondeur connue au niveau du fichier : le DSD en or, le reste selon le format.
pub(crate) fn file_tier(format: &str) -> Tier {
    if format == "dsf" || format == "dff" {
        Tier::Hires
    } else {
        tier(Some(format), None, None)
    }
}

impl crate::models::folders::Crumb {
    /// Infobulle d'un maillon replié : le chemin qu'il cache.
    pub fn cache_title(&self) -> &str {
        self.cache.as_deref().unwrap_or("")
    }
}

/// Les dossiers de la bibliothèque par nombre de fichiers, poids, dernier scan ou nom
/// (par défaut).
pub(crate) fn sort_library_dirs(list: &mut [&LibraryDir], sort: &str) {
    match sort {
        "files" => list.sort_by(|a, b| b.total_files.cmp(&a.total_files)),
        "size" => list.sort_by(|a, b| b.total_size.cmp(&a.total_size)),
        "scan" => list.sort_by(|a, b| b.last_scan_at.cmp(&a.last_scan_at)),
        _ => list.sort_by(|a, b| natural_cmp(&sort_key(&a.name), &sort_key(&b.name))),
    }
}

/// Clé de mémoire d'une tuile ou d'une ligne (`memo={…}`) : ce qu'elle affiche (le message
/// d'ouverture découle du chemin et du nom).
impl std::hash::Hash for EntryItem {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        (&self.name, &self.path, self.is_dir, &self.covers, self.albums, &self.album_badge, &self.tracks).hash(h);
        (&self.quality, self.music, &self.cover, &self.format, self.tier, &self.title, &self.infos, &self.size).hash(h);
    }
}

/// Clé de mémoire d'une entrée : l'entrée et l'identité de ses pochettes en cache, pour
/// qu'une tuile reprise plus tard parte des pochettes arrivées depuis. `grid` : la tuile
/// (pochette pleine pour un dossier d'un album ou un fichier), sinon la ligne (vignettes).
pub fn entry_key<'a>(entry: &'a EntryItem, assets: &Assets, grid: bool) -> (&'a EntryItem, Vec<usize>) {
    let ids = match (grid, entry.covers.as_slice()) {
        (true, [one]) => vec![assets.image_id(one)],
        (true, covers) if !covers.is_empty() => covers.iter().map(|c| assets.image_id(&small_key(c))).collect(),
        (false, [first, ..]) => vec![assets.image_id(&small_key(first))],
        (true, _) => entry.cover.iter().map(|c| assets.image_id(c)).collect(),
        (false, _) => entry.cover.iter().map(|c| assets.image_id(&small_key(c))).collect(),
    };
    (entry, ids)
}

impl MusicApp {
    pub(crate) fn folder_entry(&self, e: &DirEntry, covers: bool) -> EntryItem {
        let format = e.extension.clone().unwrap_or_default().to_lowercase();
        let p = e.preview.as_ref();
        let infos: Vec<String> = [e.artist.clone(), e.duration.filter(|d| *d > 0.).map(intl::clock)].into_iter().flatten().collect();
        let title = e.title.clone().unwrap_or_else(|| match e.name.rsplit_once('.') {
            Some((base, _)) if !e.is_dir => base.to_string(),
            _ => e.name.clone(),
        });
        EntryItem {
            name: e.name.clone(),
            path: e.path.clone(),
            is_dir: e.is_dir,
            covers: if covers { p.map(|p| p.covers.iter().take(4).cloned().collect()).unwrap_or_default() } else { Vec::new() },
            albums: p.map(|p| p.album_count).unwrap_or(0),
            album_badge: p.filter(|p| p.album_count > 1).map(|p| intl::number(i64::from(p.album_count))).unwrap_or_default(),
            tracks: p
                .map(|p| format!("{} {}", intl::number(i64::from(p.track_count)), t(if p.track_count == 1 { "library_head.tracks_one" } else { "library_head.tracks_n" })))
                .unwrap_or_default(),
            quality: p.and_then(|p| {
                let f = p.best_format.as_deref()?;
                let (b, r) = (p.best_bits.map(|b| b as i32), p.best_rate.map(|r| r as i32));
                Some((tier(Some(f), b, r), short_format(Some(f), b, r)))
            }),
            music: p.is_some(),
            cover: if covers { e.thumbnail_path.clone() } else { None },
            tier: file_tier(&format),
            format,
            title,
            infos: if !infos.is_empty() {
                infos.join(" · ")
            } else if e.size > 0 {
                intl::bytes(e.size as u64)
            } else {
                String::new()
            },
            size: if e.size > 0 { intl::bytes(e.size as u64) } else { String::new() },
            open: if e.is_dir {
                Msg::Folders(FoldersMsg::Open(e.path.clone(), e.name.clone()))
            } else {
                Msg::Folders(FoldersMsg::PlayFile(e.path.clone()))
            },
        }
    }

    /// Le contenu du dossier ouvert, filtré et trié : dossiers d'abord.
    pub(crate) fn folders_visible(&self) -> Vec<&DirEntry> {
        let Some(page) = self.views.folders.as_ref() else { return Vec::new() };
        let q = page.search.trim().to_lowercase();
        let by_size = page.sort == "size";
        let mut visible: Vec<&DirEntry> = page.entries.iter().filter(|e| q.is_empty() || e.name.to_lowercase().contains(&q)).collect();
        visible.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then(if by_size { b.size.cmp(&a.size) } else { std::cmp::Ordering::Equal })
                .then(natural_cmp(&a.name, &b.name))
        });
        visible
    }


    pub fn folders_view(&self) -> Option<FoldersView> {
        let page = self.views.folders.as_ref()?;
        let is_root = page.path.is_none();
        let available: Vec<_> = SORTS.iter().filter(|x| is_root || !x.3).collect();
        let sort = if available.iter().any(|x| x.0 == page.sort) { page.sort } else { "name" };
        let q = page.search.trim().to_lowercase();
        let mut root_dirs: Vec<&LibraryDir> = self.data.library_dirs.iter().filter(|d| q.is_empty() || format!("{} {}", d.name, d.path).to_lowercase().contains(&q)).collect();
        sort_library_dirs(&mut root_dirs, sort);
        let now = Utc::now();
        let roots = root_dirs
            .iter()
            .map(|d| {
                let (state, class) = match d.scan_status.as_str() {
                    "scanning" => (t("library_head.status_scanning"), "scanning"),
                    "error" if d.scan_error.as_deref() == Some("unavailable") => (t("library_head.status_unavailable"), "error"),
                    "error" => (t("library_head.status_error"), "error"),
                    _ => (t("library_head.status_ok"), "ok"),
                };
                RootItem {
                    id: d.id.clone(),
                    name: d.name.clone(),
                    path: d.path.clone(),
                    files_n: intl::number(d.total_files),
                    files_label: t(if d.total_files == 1 { "library_head.files_one" } else { "library_head.files_n" }),
                    size: if d.total_size > 0 { intl::bytes(d.total_size as u64) } else { String::new() },
                    when: d.last_scan_at.map(|w| intl::relative_time_short(elapsed_seconds(&w.to_rfc3339(), now).unwrap_or(0))).unwrap_or_default(),
                    state,
                    class,
                }
            })
            .collect();
        let visible = self.folders_visible();
        let total = visible.len();
        let window = &visible[page.window.range(total)];
        let covers = !page.without_covers;
        let dirs = window.iter().filter(|e| e.is_dir).map(|e| self.folder_entry(e, covers)).collect();
        let files = window.iter().filter(|e| !e.is_dir).map(|e| self.folder_entry(e, covers)).collect();
        let n = page.breadcrumb.len();
        let crumb = |i: usize| Crumb { name: page.breadcrumb[i].0.clone(), path: page.breadcrumb[i].1.clone(), last: i + 1 == n, cache: None };
        let crumbs = if n <= 4 {
            (0..n).map(crumb).collect()
        } else {
            let caches: Vec<&str> = page.breadcrumb[1..n - 2].iter().map(|(name, _)| name.as_str()).collect();
            vec![
                crumb(0),
                Crumb { name: "…".into(), path: page.breadcrumb[n - 3].1.clone(), last: false, cache: Some(caches.join(" › ")) },
                crumb(n - 2),
                crumb(n - 1),
            ]
        };
        let nb_dirs = page.entries.iter().filter(|e| e.is_dir).count();
        let nb_files = page.entries.len() - nb_dirs;
        let count = |n: usize, one: &str, many: &str| (n > 0).then(|| format!("{} {}", intl::number(n as i64), t(if n == 1 { one } else { many })));
        let (sort_label, sort_icon) = SORTS.iter().find(|x| x.0 == sort).map(|x| (t(x.1), x.2)).unwrap_or((t(SORTS[0].1), SORTS[0].2));
        let sorts = available
            .iter()
            .map(|x| Choice { label: t(x.1), icon: x.2.into(), active: x.0 == sort, msg: Msg::Folders(FoldersMsg::Sort(x.0)), icon_only: false })
            .collect();
        let views = [(ViewMode::Grid, "albums_view.grid", "material-symbols:grid-view-outline-rounded"), (ViewMode::List, "albums_view.list", "material-symbols:view-list-outline-rounded")]
            .into_iter()
            .map(|(mode, l, i)| Choice { label: t(l), icon: i.into(), active: self.views.view_mode == mode, msg: Msg::Albums(AlbumsMsg::View(mode)), icon_only: true })
            .collect();
        Some(FoldersView {
            is_root,
            roots,
            dirs,
            files,
            crumbs,
            dir_count: count(nb_dirs, "library_head.folders_one", "library_head.folders_n"),
            file_count: count(nb_files, "library_head.files_one", "library_head.files_n"),
            has_files: nb_files > 0,
            empty_folder: !is_root && page.entries.is_empty(),
            no_match: if is_root { root_dirs.is_empty() && !self.data.library_dirs.is_empty() } else { total == 0 && !page.entries.is_empty() },
            placeholder: if is_root {
                t_with("folders_view.filter_dirs", &[("n", &intl::number(self.data.library_dirs.len() as i64))])
            } else {
                t("folders_view.filter_entries")
            },
            sort_label,
            sort_icon,
            sorts,
            views,
            music_here: nb_files > 0,
        })
    }

    pub(crate) fn folders_covers(&self) -> Vec<String> {
        let Some(v) = self.folders_view() else { return Vec::new() };
        let small = crate::app::resources::assets::small_key;
        let mut paths = Vec::new();
        for d in &v.dirs {
            match d.covers.len() {
                1 => paths.push(d.covers[0].clone()),
                _ => paths.extend(d.covers.iter().map(|c| small(c))),
            }
        }
        if self.views.view_mode == ViewMode::Grid {
            paths.extend(v.files.iter().filter_map(|f| f.cover.clone()));
        } else {
            paths.extend(v.files.iter().filter_map(|f| f.cover.as_deref().map(small)));
        }
        paths
    }
}
