use crate::models::tags::*;
use crate::mapper::tags::*;
use std::collections::HashSet;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use karbon_ui::storage;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::models::layout::PopinSize;
use crate::mapper::tags::FIELDS;
use crate::commands::library::library_command;
use crate::commands::tags::rename_command::{self, RenameInput};
use crate::service::library::move_service::MoveOptions;

pub(crate) const STORAGE_KEY: &str = "rustmusic:rename-pattern";

pub(crate) const TREE_KEY: &str = "rustmusic:tree-pattern";

impl RenameState {
    pub(crate) fn set_pattern(&mut self, v: String) {
        if self.restructure {
            self.tree_pattern = v;
        } else {
            self.name_pattern = v;
        }
    }
}

impl MusicApp {
    pub(crate) fn rename_state(&mut self) -> Option<&mut RenameState> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::Rename(r)) => Some(r),
            _ => None,
        }
    }

    pub(crate) fn rename_update(&mut self, msg: RenameMsg) -> bool {
        if msg == RenameMsg::Open {
            if let Some(p) = self.views.tags.as_mut() {
                p.menu = None;
            }
            let st = RenameState {
                name_pattern: storage::local().get(STORAGE_KEY).filter(|s| !s.is_empty()).unwrap_or_else(|| PRESETS[0].1.into()),
                tree_pattern: storage::local().get(TREE_KEY).filter(|s| !s.is_empty()).unwrap_or_else(|| TREE_PRESETS[0].1.into()),
                satellites: true,
                cleanup: true,
                ..Default::default()
            };
            self.open_popin(Popin { title: t("rename.title"), icon: Some("lucide:file-pen-line"), body: PopinBody::Rename(Box::new(st)), size: PopinSize::Xl, bare: false, flush: true });
            if let Some(library) = self.data.library_id {
                let app = self.backend.app.clone();
                self.backend.call(async move { library_command::get_library_dirs(app.state(), library).await.unwrap_or_default() }, |me, dirs| {
                    let Some(r) = me.rename_state() else { return false };
                    r.dirs = dirs.into_iter().map(|d| d.path).collect();
                    if r.root.is_empty() {
                        r.root = r.dirs.first().cloned().unwrap_or_default();
                    }
                    me.rename_schedule();
                    true
                });
            }
            self.rename_schedule();
            return true;
        }
        let Some(r) = self.rename_state() else { return false };
        match msg {
            RenameMsg::Mode(m) => r.restructure = m,
            RenameMsg::Root(i) => r.root = r.dirs.get(i as usize).cloned().unwrap_or_default(),
            RenameMsg::Satellites => {
                r.satellites = !r.satellites;
                return true;
            }
            RenameMsg::Cleanup => {
                r.cleanup = !r.cleanup;
                return true;
            }
            RenameMsg::Pattern(v) => r.set_pattern(v),
            RenameMsg::Insert(i) => {
                let v = format!("{}{{{}}}", r.pattern(), PATTERN_FIELDS[i]);
                r.set_pattern(v);
            }
            RenameMsg::Preset(i) => {
                let v = r.presets().get(i).map(|p| p.1.to_string()).unwrap_or_default();
                r.set_pattern(v);
            }
            RenameMsg::Toggle(path) => {
                if !r.excluded.remove(&path) {
                    r.excluded.insert(path);
                }
                return true;
            }
            RenameMsg::SelectAll => {
                r.excluded = if r.all_selected() { r.changeable().iter().map(|m| m.from.clone()).collect() } else { HashSet::new() };
                return true;
            }
            RenameMsg::Confirm => {
                r.confirming = true;
                return true;
            }
            RenameMsg::Cancel => {
                r.confirming = false;
                return true;
            }
            RenameMsg::Apply => return self.rename_apply(),
            RenameMsg::Undo => return self.rename_undo(),
            RenameMsg::Open => {}
        }
        self.rename_schedule();
        true
    }

    /// Recalcule l'aperçu 250 ms après le dernier changement.
    pub(crate) fn rename_schedule(&mut self) {
        let Some(r) = self.rename_state() else { return };
        r.seq += 1;
        let seq = r.seq;
        self.backend.call(async { tokio::time::sleep(std::time::Duration::from_millis(250)).await }, move |me, ()| me.rename_run(seq));
    }

    pub(crate) fn rename_run(&mut self, seq: u64) -> bool {
        let files: Vec<RenameInput> = match self.views.tags.as_ref() {
            Some(p) => {
                let targets: Vec<_> = if p.selected.is_empty() { p.readable().collect() } else { p.targets() };
                targets.iter().map(|f| RenameInput { path: f.path.clone(), tags: FIELDS.iter().enumerate().map(|(i, k)| (k.to_string(), f.value(i).to_string())).collect() }).collect()
            }
            None => Vec::new(),
        };
        let Some(r) = self.rename_state() else { return false };
        if r.seq != seq {
            return false;
        }
        let pattern = r.pattern().to_string();
        if pattern.trim().is_empty() || files.is_empty() {
            r.preview = None;
            return true;
        }
        if r.restructure && r.root.is_empty() {
            r.preview = None;
            r.pattern_error = Some(t("rename.no_destination"));
            return true;
        }
        if let Err(e) = rename_command::check_pattern(pattern.clone()) {
            r.pattern_error = Some(e);
            r.preview = None;
            return true;
        }
        r.pattern_error = None;
        r.computing = true;
        let (restructure, root) = (r.restructure, Some(r.root.clone()).filter(|_| r.restructure));
        let app = self.backend.app.clone();
        let p2 = pattern.clone();
        self.backend.call(async move { rename_command::preview_rename(app.state(), files, p2, restructure, root).await }, move |me, res| {
            let Some(r) = me.rename_state() else { return false };
            if r.seq != seq {
                return false;
            }
            r.computing = false;
            match res {
                Ok(p) => {
                    let moves = p
                        .moves
                        .into_iter()
                        .map(|m| {
                            let mut reason = m.issues.iter().map(|i| t(&format!("rename.issue.{i}"))).collect::<Vec<_>>().join(" · ");
                            if !m.missing.is_empty() {
                                reason += &format!(" : {}", m.missing.iter().map(|f| t(&format!("rename.field.{f}"))).collect::<Vec<_>>().join(", "));
                            }
                            Move { from: m.from, from_name: m.from_name, to: m.to, to_name: m.to_name, unchanged: m.unchanged, blocked: m.blocked, reason }
                        })
                        .collect();
                    r.preview = Some((moves, p.blocked, p.unchanged));
                    if let Err(e) = storage::local().set(if restructure { TREE_KEY } else { STORAGE_KEY }, &pattern) {
                log::warn!("Préférences locales : {e}");
            }
                }
                Err(e) => {
                    r.pattern_error = Some(e);
                    r.preview = None;
                }
            }
            true
        });
        true
    }

    pub(crate) fn rename_apply(&mut self) -> bool {
        let library = self.data.library_id;
        let Some(r) = self.rename_state() else { return false };
        if r.applying {
            return false;
        }
        let pairs: Vec<(String, String)> = r.changeable().iter().filter(|m| !r.excluded.contains(&m.from)).map(|m| (m.from.clone(), m.to.clone())).collect();
        if pairs.is_empty() {
            return false;
        }
        r.applying = true;
        r.apply_error = None;
        let (pattern, restructure) = (r.pattern().to_string(), r.restructure);
        let options = MoveOptions { satellites: r.satellites, cleanup_empty: r.cleanup, roots: Vec::new() };
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let ordered = rename_command::order_moves(pairs)?;
                rename_command::apply_rename(app.state(), library, pattern, restructure, ordered, Some(options)).await
            },
            |me, res| {
                match res {
                    Ok(o) => {
                        me.tags_resync(&o.moved);
                        let Some(r) = me.rename_state() else { return true };
                        r.applying = false;
                        r.confirming = false;
                        r.outcome = Some(Outcome { batch_id: o.batch_id, succeeded: o.succeeded, failed: o.failed.into_iter().map(|f| (f.path, f.message)).collect() });
                    }
                    Err(e) => {
                        let Some(r) = me.rename_state() else { return true };
                        r.applying = false;
                        r.apply_error = Some(e);
                    }
                }
                true
            },
        );
        true
    }

    pub(crate) fn rename_undo(&mut self) -> bool {
        let Some(r) = self.rename_state() else { return false };
        let Some(id) = r.outcome.as_ref().map(|o| o.batch_id.clone()) else { return false };
        if r.undoing {
            return false;
        }
        r.undoing = true;
        r.apply_error = None;
        let app = self.backend.app.clone();
        self.backend.call(async move { rename_command::undo_batch(app.clone(), app.state(), id).await }, |me, res| {
            match res {
                Ok(o) => {
                    me.tags_resync(&o.moved);
                    if let Some(r) = me.rename_state() {
                        r.undoing = false;
                        r.outcome = None;
                    }
                    me.rename_schedule();
                }
                Err(e) => {
                    if let Some(r) = me.rename_state() {
                        r.undoing = false;
                        r.apply_error = Some(e);
                    }
                }
            }
            true
        });
        true
    }

    /// `resync` : l'atelier suit les fichiers déplacés.
    pub(crate) fn tags_resync(&mut self, moved: &[(String, String)]) {
        let Some(p) = self.views.tags.as_mut() else { return };
        for (from, to) in moved {
            if let Some(f) = p.files.iter_mut().find(|f| &f.path == from) {
                f.path = to.clone();
            }
            if p.selected.remove(from) {
                p.selected.insert(to.clone());
            }
        }
    }
}
