use crate::models::library::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::commands::library::library_command;

impl MusicApp {
    pub(crate) fn library_folders_update(&mut self, msg: LibraryFoldersMsg) -> bool {
        match msg {
            LibraryFoldersMsg::Open => {
                let Some(id) = self.data.library_id else { return false };
                self.open_library_folders(id);
            }
            LibraryFoldersMsg::Close => self.views.library_folders = None,
            LibraryFoldersMsg::Add => {
                let Some(library) = self.views.library_folders.as_ref().map(|f| f.library_id) else { return false };
                self.ask(crate::models::layout::Choice::ImportFolder { library, redirect: false });
            }
            LibraryFoldersMsg::Rescan(dir) => {
                let Some(f) = self.views.library_folders.as_mut() else { return false };
                f.rescanning = Some(dir.clone());
                let library = f.library_id;
                let app = self.backend.app.clone();
                self.backend.call(async move { library_command::rescan_library_dir(app.clone(), app.state(), library, dir).await.map(|_| ()) }, |me, r| {
                    if let Some(f) = me.views.library_folders.as_mut() {
                        f.rescanning = None;
                    }
                    if let Err(e) = r {
                        let msg = if e == "deja_en_cours" { t("notify.import_busy") } else { e };
                        me.toast(crate::models::layout::ToastKind::Error, t("notify.error"), Some(msg));
                    }
                    me.library_folders_load();
                    me.load_library_data();
                    true
                });
            }
            LibraryFoldersMsg::Remove(dir) => {
                let Some(f) = self.views.library_folders.as_mut() else { return false };
                f.dirs.retain(|d| d.id != dir);
                let app = self.backend.app.clone();
                self.backend.fire(async move {
                    let _ = library_command::remove_library_dir(app.state(), dir).await;
                });
            }
        }
        true
    }

    /// La fenêtre des dossiers de la bibliothèque `id`.
    pub(crate) fn open_library_folders(&mut self, id: i64) {
        self.menu = None;
        self.views.library_folders = Some(LibraryFolders { library_id: id, loading: true, ..Default::default() });
        self.library_folders_load();
    }

    pub(crate) fn library_folders_load(&mut self) {
        let Some(f) = self.views.library_folders.as_ref() else { return };
        let library = f.library_id;
        let app = self.backend.app.clone();
        self.backend.call(async move { library_command::get_library_dirs(app.state(), library).await }, |me, r| {
            let Some(f) = me.views.library_folders.as_mut() else { return false };
            f.loading = false;
            if let Ok(d) = r {
                f.dirs = d;
            }
            true
        });
    }
}
