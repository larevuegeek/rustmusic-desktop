//! Les playlists intelligentes : chargement des données et réponse aux messages.

use crate::models::playlists::*;
use serde_json::{json, Value};
use crate::mapper::playlists::playlist_smart::limit_of;
use crate::models::playlists::Group;
use crate::models::playlists::Limit;
use crate::models::playlists::Node;
use crate::models::playlists::Vocab;
use crate::mapper::playlists::playlist_smart::PRESETS;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::mapper::playlists::playlist_form::COLORS;
use crate::mapper::playlists::playlist_form::ICONS;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::models::layout::PopinSize;
use crate::commands::library::library_command;
use crate::commands::playlist::smart_playlist_command;

/// Une borne d'un intervalle (« entre ») ; l'autre est gardée, 0 à défaut.
pub(crate) fn set_bound(group: &mut Group, path: &[usize], which: usize, text: &str) {
    let Some(Node::Rule { value, .. }) = group.at_mut(path) else { return };
    let get = |i: usize| value.as_array().and_then(|a| a.get(i)).and_then(Value::as_f64).unwrap_or(0.);
    let mut pair = [get(0), get(1)];
    pair[which] = text.trim().replace(',', ".").parse().unwrap_or(0.);
    *value = json!(pair);
}

impl Group {
    /// Le nœud au chemin donné (indices successifs depuis la racine).
    pub fn at_mut(&mut self, path: &[usize]) -> Option<&mut Node> {
        let (first, rest) = path.split_first()?;
        let node = self.rules.get_mut(*first)?;
        if rest.is_empty() {
            return Some(node);
        }
        match node {
            Node::Group(g) => g.at_mut(rest),
            Node::Rule { .. } => None,
        }
    }

    /// Le groupe au chemin donné (la racine pour un chemin vide).
    pub fn group_mut(&mut self, path: &[usize]) -> Option<&mut Group> {
        if path.is_empty() {
            return Some(self);
        }
        match self.at_mut(path)? {
            Node::Group(g) => Some(g),
            Node::Rule { .. } => None,
        }
    }
}

impl MusicApp {
    pub(crate) fn smart_form(&mut self) -> Option<&mut SmartForm> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::Smart(f)) => Some(f),
            _ => None,
        }
    }

    pub(crate) fn smart_update(&mut self, msg: SmartMsg) -> bool {
        if let SmartMsg::Open(id, mix) = msg {
            return self.smart_open(id, mix);
        }
        if msg == SmartMsg::Submit {
            return self.smart_save();
        }
        let Some(f) = self.smart_form() else { return false };
        let Some(vocab) = f.vocab.clone() else { return false };
        match msg {
            SmartMsg::Name(v) => {
                f.name = v;
                return true;
            }
            SmartMsg::Color(i) => {
                f.color = i;
                return true;
            }
            SmartMsg::Icon(i) => {
                f.icon = i;
                return true;
            }
            SmartMsg::Preset(i) => {
                let Some(p) = PRESETS.get(i) else { return false };
                f.preset = Some(p.key);
                f.group = p.group();
                f.limit = if f.mix { limit_of(f.limit.count) } else { p.limit() };
                f.count_text = f.limit.count.map(|n| n.to_string()).unwrap_or_default();
                if f.name.trim().is_empty() || Some(&f.name) == f.preset_name.as_ref() {
                    f.name = crate::mapper::playlists::playlist_smart::preset_name(p);
                    f.preset_name = Some(f.name.clone());
                }
                if let Some(c) = COLORS.iter().position(|c| *c == p.color) {
                    f.color = c;
                }
                if let Some(c) = ICONS.iter().position(|c| *c == p.icon) {
                    f.icon = c;
                }
            }
            SmartMsg::Match(path, all) => {
                let Some(g) = f.group.group_mut(&path) else { return false };
                g.all = all;
            }
            SmartMsg::Field(path, i) => {
                let Some(key) = vocab.fields.get(i as usize).map(|f| f.key.clone()) else { return false };
                let Some(Node::Rule { field, op, value }) = f.group.at_mut(&path) else { return false };
                // L'opérateur courant peut ne pas exister pour la nouvelle nature.
                let ops = vocab.ops_for(&key);
                let keep = ops.iter().any(|o| o.key == *op);
                if !keep {
                    *op = ops.first().map_or("is".into(), |o| o.key.clone());
                    *value = Value::Null;
                }
                *field = key;
            }
            SmartMsg::Op(path, i) => {
                let Some(Node::Rule { field, op, .. }) = f.group.at_mut(&path) else { return false };
                let Some(o) = vocab.ops_for(field).get(i as usize) else { return false };
                *op = o.key.clone();
            }
            SmartMsg::Value(path, v) => {
                let Some(Node::Rule { value, .. }) = f.group.at_mut(&path) else { return false };
                *value = json!(v);
            }
            SmartMsg::Low(path, v) => set_bound(&mut f.group, &path, 0, &v),
            SmartMsg::High(path, v) => set_bound(&mut f.group, &path, 1, &v),
            SmartMsg::Remove(path) => {
                let Some((last, parent)) = path.split_last() else { return false };
                let Some(g) = f.group.group_mut(parent) else { return false };
                if *last < g.rules.len() {
                    g.rules.remove(*last);
                }
            }
            SmartMsg::AddRule(path) => {
                let Some(g) = f.group.group_mut(&path) else { return false };
                g.rules.push(vocab.default_rule());
            }
            SmartMsg::AddGroup(path) => {
                let Some(g) = f.group.group_mut(&path) else { return false };
                let sub = Group { all: !g.all, rules: vec![vocab.default_rule()] };
                g.rules.push(Node::Group(sub));
            }
            SmartMsg::Size(n) => f.limit = limit_of(Some(n)),
            SmartMsg::LimitCount(v) => {
                f.limit.count = v.trim().parse().ok().filter(|n| *n > 0);
                f.count_text = v;
            }
            SmartMsg::LimitSort(i) => {
                f.limit.sort = match i {
                    0 => None,
                    1 => Some("random".into()),
                    n => vocab.fields.get(n as usize - 2).map(|f| f.key.clone()),
                };
            }
            SmartMsg::LimitDesc => f.limit.desc = !f.limit.desc,
            SmartMsg::Open(..) | SmartMsg::Submit => {}
        }
        self.smart_changed();
        true
    }

    pub(crate) fn smart_open(&mut self, id: Option<i64>, mix: bool) -> bool {
        let existing = id.and_then(|id| self.data.playlists.iter().find(|p| p.id == id)).cloned();
        let mix = mix || existing.as_ref().is_some_and(|p| p.is_mix);
        let source = existing.as_ref().map(|p| p.source_id).or_else(|| self.source_id()).unwrap_or(0);
        let mut form = SmartForm {
            playlist_id: id,
            mix,
            source,
            name: String::new(),
            color: 0,
            icon: 0,
            group: Group { all: true, rules: Vec::new() },
            limit: Limit { count: None, sort: None, desc: true },
            vocab: None,
            saving: false,
            error: None,
            count: None,
            counting: false,
            generation: 0,
            preset_name: None,
            preset: None,
            count_text: String::new(),
        };
        if let Some(p) = &existing {
            form.name = p.name.clone();
            form.color = COLORS.iter().position(|c| *c == p.color).unwrap_or(0);
            form.icon = ICONS.iter().position(|c| *c == p.icon).unwrap_or(0);
        }
        let (title, icon) = if mix {
            (t(if id.is_some() { "mix_view.edit" } else { "mix_view.new" }), "material-symbols:shuffle-rounded")
        } else {
            (t("playlist.new_smart"), "lucide:sparkles")
        };
        self.open_popin(Popin { title, icon: Some(icon), body: PopinBody::Smart(Box::new(form)), size: PopinSize::Xl, bare: false, flush: true });

        let library = self.data.libraries.first().map(|l| l.id);
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let voc = serde_json::to_value(smart_playlist_command::get_rule_vocabulary()).unwrap_or_default();
                let tags = match library {
                    Some(l) => library_command::get_library_tag_keys(app.state(), l).await.map(|v| v.into_iter().map(|c| (c.key, c.filled)).collect()).unwrap_or_else(|e| {
                        log::error!("Recensement des tags impossible : {e}");
                        Vec::new()
                    }),
                    None => Vec::new(),
                };
                let rules = match id {
                    Some(id) => smart_playlist_command::get_smart_playlist_rules(app.state(), id).await.map(|r| r.and_then(|r| serde_json::to_value(r).ok())),
                    None => Ok(None),
                };
                (voc, tags, rules)
            },
            move |me, (voc, tags, rules)| {
                let Some(f) = me.smart_form() else { return false };
                let vocab = Vocab::from_json(&voc, &tags);
                match rules {
                    Ok(Some(r)) => {
                        f.group = Group::from_json(&r);
                        if let Some(l) = r.get("limit").filter(|l| !l.is_null()) {
                            f.limit = Limit::from_json(l);
                        }
                    }
                    Ok(None) => {}
                    Err(e) => f.error = Some(e),
                }
                if f.mix {
                    f.limit = limit_of(f.limit.count);
                }
                f.count_text = f.limit.count.map(|n| n.to_string()).unwrap_or_default();
                if f.group.rules.is_empty() {
                    f.group = Group { all: true, rules: vec![Node::Rule { field: "rating".into(), op: "gte".into(), value: json!("4") }] };
                }
                f.vocab = Some(vocab);
                me.smart_changed();
                true
            },
        );
        true
    }

    /// Recompte 250 ms après le dernier changement.
    pub(crate) fn smart_changed(&mut self) {
        let Some(f) = self.smart_form() else { return };
        f.generation += 1;
        let generation = f.generation;
        self.backend.call(async { tokio::time::sleep(std::time::Duration::from_millis(250)).await }, move |me, ()| me.smart_recount(generation));
    }

    pub(crate) fn smart_recount(&mut self, generation: u64) -> bool {
        let Some(f) = self.smart_form() else { return false };
        if f.generation != generation {
            return false;
        }
        let Some(vocab) = &f.vocab else { return false };
        let mut rules = vocab.prune(&f.group).to_json();
        rules["limit"] = Value::Null;
        let source = f.source;
        f.counting = true;
        f.error = None;
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                match serde_json::from_value(rules) {
                    Ok(r) => smart_playlist_command::count_smart_playlist(app.state(), r, Some(source)).await,
                    Err(e) => Err(e.to_string()),
                }
            },
            move |me, r| {
                let Some(f) = me.smart_form() else { return false };
                if f.generation != generation {
                    return false;
                }
                f.counting = false;
                match r {
                    Ok(n) => f.count = Some(n),
                    Err(e) => {
                        f.count = None;
                        f.error = Some(e);
                    }
                }
                true
            },
        );
        true
    }

    pub(crate) fn smart_save(&mut self) -> bool {
        let profile = self.data.profile_id();
        let Some(f) = self.smart_form() else { return false };
        let Some(vocab) = &f.vocab else { return false };
        if f.name.trim().is_empty() {
            f.error = Some(t("smart.name_required"));
            return true;
        }
        let mut rules = vocab.prune(&f.group).to_json();
        rules["limit"] = if f.limit.count.is_some() || f.limit.sort.is_some() { f.limit.to_json() } else { Value::Null };
        f.saving = true;
        f.error = None;
        let (id, mix, source, name, color, icon) = (f.playlist_id, f.mix, f.source, f.name.clone(), f.color_hex().to_string(), f.icon_id().to_string());
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let rules = serde_json::from_value(rules).map_err(|e| e.to_string())?;
                match id {
                    Some(id) => smart_playlist_command::update_smart_playlist(app.state(), id, Some(name), Some(color), Some(icon), rules).await,
                    None => {
                        let profile = profile.ok_or_else(|| t("smart.no_profile"))?;
                        smart_playlist_command::create_smart_playlist(app.state(), profile, name, Some(color), Some(icon), rules, Some(mix), Some(source)).await.map(|_| ())
                    }
                }
            },
            |me, r| {
                match r {
                    Ok(()) => {
                        me.views.popin = None;
                        me.load_playlists();
                    }
                    Err(e) => {
                        let Some(f) = me.smart_form() else { return true };
                        f.saving = false;
                        f.error = Some(e);
                    }
                }
                true
            },
        );
        true
    }
}
