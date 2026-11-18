//! L'éditeur de tags : chargement des données et réponse aux messages.

use crate::models::tags::*;
use crate::mapper::tags::*;
use serde_json::{json, Map, Value};
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::models::layout::PopinSize;
use crate::models::layout::ToastKind;
use crate::commands::tags::tag_command;

pub const OTHER: u8 = 0;

pub(crate) fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

impl MusicApp {
    pub(crate) fn tag_editor(&mut self) -> Option<&mut TagEditor> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::TagEdit(f)) => Some(f),
            _ => None,
        }
    }

    pub(crate) fn tag_editor_update(&mut self, msg: TagEditorMsg) -> bool {
        if let TagEditorMsg::Open(path) = msg {
            return self.tag_editor_open(path);
        }
        let Some(e) = self.tag_editor() else { return false };
        match msg {
            TagEditorMsg::Field(i, v) => {
                if let Some(f) = e.form.get_mut(i) {
                    *f = v;
                }
            }
            TagEditorMsg::Revert(i) => {
                if let (Some(f), Some(o)) = (e.form.get_mut(i), e.original.get(i)) {
                    *f = o.clone();
                }
            }
            TagEditorMsg::RevertAll => {
                e.form = e.original.clone();
                e.slots = e.original_slots.clone();
            }
            TagEditorMsg::SetCover(i) => {
                for (j, s) in e.slots.iter_mut().enumerate() {
                    if j == i {
                        s.picture_type = COVER;
                    } else if s.picture_type == COVER {
                        s.picture_type = OTHER;
                    }
                }
            }
            TagEditorMsg::Move(i, down) => {
                let j = if down { i + 1 } else { i.wrapping_sub(1) };
                if j < e.slots.len() && i < e.slots.len() {
                    e.slots.swap(i, j);
                }
            }
            TagEditorMsg::Remove(i) => {
                if i < e.slots.len() {
                    e.slots.remove(i);
                }
            }
            TagEditorMsg::Zoom(z) => e.zoomed = z,
            TagEditorMsg::Fetch => {
                let query = [e.value("artist"), e.value("title")].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ");
                let query = if query.trim().is_empty() {
                    let f = e.filename();
                    f.rsplit_once('.').map(|(n, _)| n.to_string()).unwrap_or(f).replace(['_', '-'], " ")
                } else {
                    query
                };
                e.lookup = Some(crate::models::tags::Lookup::new(query));
                return self.lookup_search();
            }
            TagEditorMsg::Close => {
                if e.dirty() && !e.busy {
                    e.confirming = true;
                } else {
                    self.views.popin = None;
                }
            }
            TagEditorMsg::Discard => self.views.popin = None,
            TagEditorMsg::KeepEditing => e.confirming = false,
            TagEditorMsg::Submit => return self.tag_editor_submit(),
            TagEditorMsg::Open(_) => {}
        }
        true
    }

    pub(crate) fn tag_editor_open(&mut self, path: String) -> bool {
        let editor = TagEditor {
            path: path.clone(),
            original: vec![String::new(); EDITOR_FIELDS.len()],
            form: vec![String::new(); EDITOR_FIELDS.len()],
            slots: Vec::new(),
            original_slots: Vec::new(),
            zoomed: None,
            loading: true,
            busy: false,
            error: None,
            confirming: false,
            filled: Vec::new(),
            lookup: None,
        };
        self.menu = None;
        self.open_popin(Popin {
            title: t("tags.edit"),
            icon: Some("material-symbols:sell-outline-rounded"),
            body: PopinBody::TagEdit(Box::new(editor)),
            size: PopinSize::Xl,
            bare: false,
            flush: true,
        });
        self.backend.call(
            async move {
                let p = path.clone();
                crate::app::platform::runtime::spawn_blocking(move || (tag_command::read_track_tags(p.clone()), tag_command::read_track_images(p)))
                    .await
                    .map_err(|e| e.to_string())
            },
            |me, r| {
                let Some(e) = me.tag_editor() else { return false };
                e.loading = false;
                match r {
                    Ok((tags, images)) => {
                        match tags.map(|v| serde_json::to_value(v).unwrap_or_default()) {
                            Ok(v) => {
                                e.original = EDITOR_FIELDS.iter().map(|k| text(&v[*k])).collect();
                                e.form = e.original.clone();
                            }
                            Err(err) => e.error = Some(err),
                        }
                        e.original_slots = images
                            .unwrap_or_default()
                            .into_iter()
                            .map(|i| Slot {
                                id: Some(i.id),
                                path: None,
                                src: i.src,
                                picture_type: i.picture_type,
                                description: i.description.unwrap_or_default(),
                                mime: i.mime_type,
                                bytes: i.bytes,
                            })
                            .collect();
                        e.slots = e.original_slots.clone();
                    }
                    Err(err) => e.error = Some(err),
                }
                true
            },
        );
        true
    }

    pub(crate) fn tag_editor_submit(&mut self) -> bool {
        let Some(e) = self.tag_editor() else { return false };
        if !e.dirty() || e.busy {
            return false;
        }
        e.error = None;
        e.busy = true;
        let mut edit = Map::new();
        for (i, k) in EDITOR_FIELDS.iter().enumerate() {
            if e.form[i] != e.original[i] {
                edit.insert((*k).into(), json!(e.form[i]));
            }
        }
        if e.media_dirty() {
            let images: Vec<Value> = e
                .slots
                .iter()
                .map(|s| json!({ "id": s.id, "path": s.path, "picture_type": s.picture_type, "description": Some(&s.description).filter(|d| !d.is_empty()) }))
                .collect();
            edit.insert("images".into(), Value::Array(images));
        }
        let path = e.path.clone();
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let edit = serde_json::from_value(Value::Object(edit)).map_err(|e| e.to_string())?;
                tag_command::write_track_tags(app.clone(), app.state(), path, edit).await.map(|_| ())
            },
            |me, r| {
                match r {
                    Ok(()) => {
                        me.views.popin = None;
                        me.toast(ToastKind::Success, t("tags.saved"), None);
                        me.load_page();
                    }
                    Err(err) => {
                        let Some(e) = me.tag_editor() else { return true };
                        e.busy = false;
                        e.error = Some(err);
                    }
                }
                true
            },
        );
        true
    }
}
