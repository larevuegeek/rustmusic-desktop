use crate::models::tags::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::models::layout::PopinSize;
use crate::commands::tags::rename_command;

impl MusicApp {
    pub(crate) fn history_state(&mut self) -> Option<&mut HistoryState> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::History(h)) => Some(h),
            _ => None,
        }
    }

    pub(crate) fn history_update(&mut self, msg: HistoryMsg) -> bool {
        match msg {
            HistoryMsg::Open => {
                if let Some(p) = self.views.tags.as_mut() {
                    p.menu = None;
                }
                self.open_popin(Popin {
                    title: t("history.title"),
                    icon: Some("lucide:history"),
                    body: PopinBody::History(Box::new(HistoryState { loading: true, ..Default::default() })),
                    size: PopinSize::Lg,
                    bare: false,
                    flush: true,
                });
                self.history_load();
            }
            HistoryMsg::Ask(id) => {
                let Some(h) = self.history_state() else { return false };
                h.confirming = Some(id);
            }
            HistoryMsg::Cancel => {
                let Some(h) = self.history_state() else { return false };
                h.confirming = None;
            }
            HistoryMsg::Undo(id) => {
                let Some(h) = self.history_state() else { return false };
                h.working = Some(id.clone());
                h.error = None;
                let app = self.backend.app.clone();
                self.backend.call(async move { rename_command::undo_batch(app.clone(), app.state(), id).await }, |me, r| {
                    match r {
                        Ok(o) => {
                            // `resync` : l'atelier suit les fichiers revenus à leur place.
                            if let Some(p) = me.views.tags.as_mut() {
                                for (from, to) in &o.moved {
                                    if let Some(f) = p.files.iter_mut().find(|f| &f.path == from) {
                                        f.path = to.clone();
                                    }
                                    if p.selected.remove(from) {
                                        p.selected.insert(to.clone());
                                    }
                                }
                            }
                            let Some(h) = me.history_state() else { return true };
                            h.working = None;
                            h.confirming = None;
                            h.result = Some((o.succeeded, o.failed.len(), o.failed.first().map(|f| f.message.clone())));
                            me.history_load();
                        }
                        Err(e) => {
                            let Some(h) = me.history_state() else { return true };
                            h.working = None;
                            h.error = Some(e);
                        }
                    }
                    true
                });
            }
        }
        true
    }

    pub(crate) fn history_load(&mut self) {
        let app = self.backend.app.clone();
        self.backend.call(async move { rename_command::list_batch_journal(app.state(), Some(50)).await }, |me, r| {
            let Some(h) = me.history_state() else { return false };
            h.loading = false;
            match r {
                Ok(list) => {
                    h.error = None;
                    h.entries = list
                        .into_iter()
                        .map(|e| Entry { id: e.id, kind: e.kind, pattern: e.pattern, created_at: e.created_at, succeeded: e.succeeded, failed: e.failed, undone: e.undone_at.is_some() })
                        .collect();
                }
                Err(e) => h.error = Some(e),
            }
            true
        });
    }
}
