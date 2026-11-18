use crate::models::layout::*;
use std::time::Duration;
use crate::app::music_app::MusicApp;

impl MusicApp {
    /// `toasts.push` : affiché, puis retiré après `duration` (2,5 s par défaut).
    pub(crate) fn toast(&mut self, kind: ToastKind, title: impl Into<String>, message: Option<String>) {
        let t = &mut self.views.toasts;
        let id = t.next;
        t.next += 1;
        t.list.push(Toast { id, kind, title: title.into(), message });
        self.backend.call(async move { tokio::time::sleep(Duration::from_millis(2500)).await }, move |me, ()| me.toast_remove(id));
    }

    pub(crate) fn toast_remove(&mut self, id: u64) -> bool {
        let before = self.views.toasts.list.len();
        self.views.toasts.list.retain(|t| t.id != id);
        before != self.views.toasts.list.len()
    }
}
