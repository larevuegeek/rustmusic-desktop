use crate::models::tags::*;
use serde_json::{json, Value};
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::models::layout::PopinSize;
use crate::mapper::tags::EDITOR_FIELDS;
use crate::commands::tags::tag_command;

impl MusicApp {
    pub(crate) fn batch_editor(&mut self) -> Option<&mut BatchEditor> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::BatchEdit(b)) => Some(b),
            _ => None,
        }
    }

    pub(crate) fn batch_editor_update(&mut self, msg: BatchEditorMsg) -> bool {
        if let BatchEditorMsg::Open(paths) = msg {
            let ed = BatchEditor { paths: paths.clone(), form: vec![String::new(); EDITOR_FIELDS.len()], loading: true, ..Default::default() };
            self.open_popin(Popin { title: t("tags.edit"), icon: Some("lucide:tags"), body: PopinBody::BatchEdit(Box::new(ed)), size: PopinSize::Xl, bare: false, flush: true });
            self.backend.call(
                async move {
                    crate::app::platform::runtime::spawn_blocking(move || {
                        paths
                            .into_iter()
                            .filter_map(|p| tag_command::read_track_tags(p).ok())
                            .map(|tags| {
                                let v = serde_json::to_value(tags).unwrap_or_default();
                                EDITOR_FIELDS.iter().map(|k| match &v[*k] {
                                    Value::String(s) => s.clone(),
                                    Value::Number(n) => n.to_string(),
                                    _ => String::new(),
                                }).collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>()
                    })
                    .await
                    .unwrap_or_default()
                },
                |me, per_file| {
                    let Some(b) = me.batch_editor() else { return false };
                    b.loading = false;
                    if per_file.is_empty() {
                        b.error = Some(t("tags.batch_unreadable"));
                        return true;
                    }
                    // La valeur commune, sinon « valeurs multiples ».
                    b.common = (0..EDITOR_FIELDS.len()).map(|i| {
                        let first = &per_file[0][i];
                        per_file.iter().all(|f| &f[i] == first).then(|| first.clone())
                    }).collect();
                    b.form = b.common.iter().map(|c| c.clone().unwrap_or_default()).collect();
                    true
                },
            );
            return true;
        }
        let Some(b) = self.batch_editor() else { return false };
        match msg {
            BatchEditorMsg::Field(i, v) => {
                b.form[i] = v;
                b.touched.insert(i);
            }
            BatchEditorMsg::Revert(i) => {
                b.form[i] = b.common.get(i).cloned().flatten().unwrap_or_default();
                b.touched.remove(&i);
            }
            BatchEditorMsg::RevertAll => {
                b.form = b.common.iter().map(|c| c.clone().unwrap_or_default()).collect();
                b.touched.clear();
                b.numbering = false;
            }
            BatchEditorMsg::Numbering => {
                b.numbering = !b.numbering;
                // Index de `total_tracks` dans EDITOR_FIELDS.
                let total = EDITOR_FIELDS.iter().position(|f| *f == "total_tracks").unwrap_or(9);
                if b.numbering && !b.touched.contains(&total) {
                    b.form[total] = b.paths.len().to_string();
                    b.touched.insert(total);
                }
            }
            BatchEditorMsg::Close => {
                if b.dirty() && !b.busy {
                    b.confirming = true;
                } else {
                    self.views.popin = None;
                }
            }
            BatchEditorMsg::Discard => self.views.popin = None,
            BatchEditorMsg::KeepEditing => b.confirming = false,
            BatchEditorMsg::Submit => return self.batch_editor_submit(),
            BatchEditorMsg::Open(_) => {}
        }
        true
    }

    pub(crate) fn batch_editor_submit(&mut self) -> bool {
        let running = self.views.batch.as_ref().is_some_and(|b| b.running);
        let Some(b) = self.batch_editor() else { return false };
        if !b.dirty() || b.busy {
            return false;
        }
        if running {
            b.error = Some(t("tags.batch_already_running"));
            return true;
        }
        b.error = None;
        b.busy = true;
        let (edit, paths, numbering) = (b.payload(), b.paths.clone(), b.numbering);
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                if numbering {
                    let items: Vec<Value> = paths
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            let mut e = edit.clone();
                            e.insert("track_number".into(), json!((i + 1).to_string()));
                            json!({ "path": p, "edit": Value::Object(e) })
                        })
                        .collect();
                    let items = serde_json::from_value(Value::Array(items)).map_err(|e| e.to_string())?;
                    tag_command::write_tags_each(app.clone(), app.state(), items).await
                } else {
                    let edit = serde_json::from_value(Value::Object(edit)).map_err(|e| e.to_string())?;
                    tag_command::write_tags_batch(app.clone(), app.state(), paths, edit).await
                }
            },
            |me, r| {
                match r {
                    Ok(job) => {
                        me.views.popin = None;
                        me.batch_begin(job, t("batch.writing_tags"));
                    }
                    Err(e) => {
                        if let Some(b) = me.batch_editor() {
                            b.busy = false;
                            b.error = Some(e);
                        }
                    }
                }
                true
            },
        );
        true
    }
}
