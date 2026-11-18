use crate::models::servers::*;
use crate::app::music_app::MusicApp;
use crate::models::layout::PopinBody;
use crate::commands::network::server_command;

impl MusicApp {
    pub(crate) fn dlna_pick(&mut self) -> Option<&mut DlnaPick> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::AddServer(f)) => f.dlna.as_mut(),
            Some(PopinBody::ServerFolders(f)) => f.dlna.as_mut(),
            _ => None,
        }
    }

    pub(crate) fn dlna_update(&mut self, msg: DlnaMsg) -> bool {
        let Some(p) = self.dlna_pick() else { return false };
        match msg {
            DlnaMsg::Open(i) => {
                let Some(f) = p.folders.get(i) else { return false };
                p.trail.push((f.id.clone(), f.name.clone()));
            }
            DlnaMsg::Trail(i) => p.trail.truncate(i + 1),
            DlnaMsg::Toggle(i) => {
                let Some(f) = p.folders.get(i).cloned() else { return false };
                if p.is_selected(&f.id) {
                    p.selected.retain(|s| s.0 != f.id);
                } else {
                    p.selected.push((f.id, f.name));
                }
                return true;
            }
            DlnaMsg::Unselect(i) => {
                if i < p.selected.len() {
                    p.selected.remove(i);
                }
                return true;
            }
        }
        self.dlna_load();
        true
    }

    /// Charge les sous-dossiers de l'étape courante.
    pub(crate) fn dlna_load(&mut self) {
        let Some(p) = self.dlna_pick() else { return };
        let id = p.trail.last().map(|t| t.0.clone()).unwrap_or_else(|| "0".into());
        let url = p.url.clone();
        p.loading = true;
        p.error = None;
        let wanted = id.clone();
        self.backend.call(async move { server_command::server_browse(url, id).await }, move |me, r| {
            let Some(p) = me.dlna_pick() else { return false };
            if p.trail.last().map(|t| t.0.as_str()) != Some(wanted.as_str()) {
                return false;
            }
            p.loading = false;
            match r {
                Ok(f) => p.folders = f,
                Err(e) => {
                    p.folders.clear();
                    p.error = Some(e);
                }
            }
            true
        });
    }
}
