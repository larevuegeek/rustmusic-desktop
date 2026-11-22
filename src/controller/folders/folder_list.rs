//! L'explorateur de dossiers : chargement des données et réponse aux messages.

use crate::models::folders::*;
use crate::app::music_app::MusicApp;
use crate::commands::library::library_command;

impl MusicApp {
    pub(crate) fn folders_open(&mut self) {
        let Some(library) = self.data.library_id else { return };
        if !self.views.folders.as_ref().is_some_and(|p| p.library_id == library) {
            self.views.folders = Some(FoldersPage { library_id: library, sort: "name", ..Default::default() });
        }
        self.load_library_dirs();
    }

    pub(crate) fn folders_navigate(&mut self, path: String, name: String) {
        let Some(library) = self.data.library_id else { return };
        let Some(page) = self.views.folders.as_mut() else { return };
        page.loading = true;
        page.path = Some(path.clone());
        page.search.clear();
        page.window.reset();
        // Les rangs cochés désignaient les fichiers du dossier quitté.
        if let Some(s) = self.views.selection.as_mut() {
            s.picked.clear();
            s.all = false;
        }
        match page.breadcrumb.iter().position(|(_, p)| *p == path) {
            Some(i) => page.breadcrumb.truncate(i + 1),
            None => page.breadcrumb.push((name, path.clone())),
        }
        let app = self.backend.app.clone();
        let p = path.clone();
        self.backend.call(
            async move { library_command::list_directory(app.state(), library, p).await.unwrap_or_default() },
            move |me, entries| {
                let Some(page) = me.views.folders.as_mut().filter(|x| x.path.as_deref() == Some(path.as_str())) else { return false };
                page.entries = entries;
                page.loading = false;
                true
            },
        );
    }

    pub(crate) fn folders_search(&mut self, value: String) -> bool {
        let Some(page) = self.views.folders.as_mut() else { return false };
        page.search = value;
        page.window.reset();
        true
    }


    pub(crate) fn folders_update(&mut self, msg: FoldersMsg) -> bool {
        match msg {
            FoldersMsg::Open(path, name) => {
                self.folders_navigate(path, name);
                return true;
            }
            FoldersMsg::Parent => {
                let parent = self.views.folders.as_ref().and_then(|p| (p.breadcrumb.len() > 1).then(|| p.breadcrumb[p.breadcrumb.len() - 2].clone()));
                match parent {
                    Some((name, path)) => self.folders_navigate(path, name),
                    None => return self.folders_update(FoldersMsg::Root),
                }
                return true;
            }
            FoldersMsg::PlayFile(_) | FoldersMsg::PlayFolder if self.views.folders.is_some() => {
                let files: Vec<_> = self.views.folders.as_ref().map(|p| p.entries.iter().filter(|e| !e.is_dir).cloned().collect()).unwrap_or_default();
                let profile = self.data.profile_id().unwrap_or(0);
                let queue: Vec<_> = files
                    .iter()
                    .map(|e| crate::entity::queue::queue_track::QueueTrack {
                        queue_id: String::new(),
                        profile_id: profile,
                        path: e.path.clone(),
                        title: e.title.clone().unwrap_or_else(|| e.name.clone()),
                        artist: e.artist.clone(),
                        duration: e.duration,
                        cover: e.thumbnail_path.clone(),
                        position: 0,
                    })
                    .collect();
                let start = match &msg {
                    FoldersMsg::PlayFile(p) => p.clone(),
                    _ => queue.first().map(|q| q.path.clone()).unwrap_or_default(),
                };
                if !queue.is_empty() {
                    self.play_list(queue, &start);
                }
                return true;
            }
            FoldersMsg::Pick(i) => {
                let track = self.folder_tracks().get(i).cloned();
                return self.selection_toggle_row(i, track);
            }
            FoldersMsg::EditTags => {
                let paths: Vec<String> = self.folder_tracks().into_iter().map(|t| t.path).collect();
                if paths.is_empty() {
                    return false;
                }
                let folder = self.views.folders.as_ref().and_then(|p| p.breadcrumb.last().map(|(name, _)| name.clone()));
                self.views.workshop_paths = Some(paths);
                self.views.workshop_source = folder;
                return self.handle(crate::models::app::Msg::Go(crate::models::app::Route::Tags(None)));
            }
            FoldersMsg::Scrolled(offset, max) => {
                let (len, export) = (self.folders_visible().len(), self.export);
                let Some(page) = self.views.folders.as_mut() else { return false };
                return page.window.scrolled(offset, max, len, export);
            }
            _ => {}
        }
        let Some(page) = self.views.folders.as_mut() else { return false };
        match msg {
            FoldersMsg::Root => {
                page.path = None;
                page.breadcrumb.clear();
                page.entries.clear();
                page.search.clear();
                page.window.reset();
            }
            FoldersMsg::Sort(k) => {
                page.sort = k;
                page.window.reset();
                self.menu = None;
            }
            FoldersMsg::Covers => page.without_covers = !page.without_covers,
            _ => {}
        }
        true
    }
}
