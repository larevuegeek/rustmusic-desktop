use crate::models::playlists::*;
use serde_json::{json, Value};
use karbon_ui::intl::t;
use karbon_ui::ui::prelude::{Background, Color, GradientStop, LinearGradient};
use crate::models::playlists::Group;
use crate::models::playlists::Node;
use crate::models::playlists::Vocab;
use crate::models::playlists::SmartForm;
use karbon_ui::intl;
use crate::helper::color::hex;
use crate::models::playlists::Limit;
use crate::mapper::playlists::playlist_form::COLORS;
use crate::mapper::playlists::playlist_form::ICONS;

/// `MAX_DEPTH` : le moteur refuse au-delà de trois niveaux.
pub const MAX_DEPTH: usize = 3;

/// Les tailles proposées pour un mix, toujours tiré au hasard.
pub const SIZES: [i64; 3] = [30, 50, 100];

/// La limite d'un mix.
pub fn limit_of(n: Option<i64>) -> Limit {
    Limit { count: Some(n.unwrap_or(50)), sort: Some("random".into()), desc: false }
}

pub(crate) fn translated(key: &str) -> Option<String> {
    let v = t(key);
    (v != key).then_some(v)
}

/// Intitulé traduit, repli sur celui du moteur.
pub fn field_label(f: &Field) -> String {
    translated(&format!("smart.field.{}", f.key)).or_else(|| translated(&format!("columns.{}", f.key))).unwrap_or_else(|| f.label.clone())
}

pub fn op_label(o: &Op) -> String {
    translated(&format!("smart.op.{}", o.key)).unwrap_or_else(|| o.label.clone())
}

pub(crate) const fn preset(
    key: &'static str,
    icon: &'static str,
    color: &'static str,
    rules: &'static [(&'static str, &'static str, i64)],
    count: Option<i64>,
    sort: &'static str,
    desc: bool,
) -> Preset {
    Preset { key, icon, color, rules, count, sort, desc }
}

/// `PRESETS` (noms et aides : `smart.preset.<clé>` et `<clé>_hint`).
pub const PRESETS: [Preset; 8] = [
    preset("most_played", "mynaui:fire", "#f97316", &[("play_count", "gt", 0)], Some(100), "play_count", true),
    preset("top_genres", "mynaui:star", "#8b5cf6", &[("genre", "in_top_played", 5)], Some(100), "random", false),
    preset("top_artists", "mynaui:heart", "#ec4899", &[("artist", "in_top_played", 10)], Some(100), "random", false),
    preset("best_rated", "mynaui:star", "#22c55e", &[("rating", "gte", 4)], None, "rating", true),
    preset("never_played", "mynaui:moon", "#0ea5e9", &[("play_count", "eq", 0)], Some(200), "random", false),
    preset("forgotten", "mynaui:cloud", "#f59e0b", &[("play_count", "gt", 0), ("last_played_at", "not_in_last", 180)], Some(100), "random", false),
    preset("recent", "mynaui:sun", "#14b8a6", &[("created_at", "in_last", 30)], None, "created_at", true),
    preset("hires", "mynaui:lightning", "#6366f1", &[("bits_per_sample", "gte", 24)], None, "sample_rate", true),
];

/// Nom traduit d'une recette.
pub fn preset_name(p: &Preset) -> String {
    t(&format!("smart.preset.{}", p.key))
}

/// Aide traduite d'une recette.
pub fn preset_hint(p: &Preset) -> String {
    t(&format!("smart.preset.{}_hint", p.key))
}

pub(crate) fn number(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.as_f64().map_or_else(|| n.to_string(), |f| if f.fract() == 0. { format!("{}", f as i64) } else { f.to_string() }),
        _ => String::new(),
    }
}

pub(crate) fn group_view(vocab: &Vocab, labels: &[String], g: &Group, path: Vec<usize>, depth: usize) -> GroupView {
    let nodes = g
        .rules
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let mut p = path.clone();
            p.push(i);
            match n {
                Node::Group(sub) => NodeView { rule: None, group: Some(group_view(vocab, labels, sub, p, depth + 1)) },
                Node::Rule { field, op, value } => {
                    let ops = vocab.ops_for(field);
                    let pair = value.as_array();
                    NodeView {
                        rule: Some(RuleView {
                            path: p,
                            fields: labels.to_vec(),
                            field: vocab.fields.iter().position(|f| f.key == *field).unwrap_or(0) as u64,
                            ops: ops.iter().map(op_label).collect(),
                            op: ops.iter().position(|o| o.key == *op).unwrap_or(0) as u64,
                            arity: vocab.arity(field, op),
                            value: if pair.is_some() { String::new() } else { number(value) },
                            low: pair.and_then(|a| a.first()).map(number).unwrap_or_default(),
                            high: pair.and_then(|a| a.get(1)).map(number).unwrap_or_default(),
                        }),
                        group: None,
                    }
                }
            }
        })
        .collect();
    GroupView { head: GroupHead { path, all: g.all, removable: depth > 0, can_group: depth < MAX_DEPTH - 1 }, nodes }
}

/// Vignette : `linear-gradient(140deg, color, color-mix(color 40%, black))`.
pub fn tile_background(color: &str) -> Vec<Background> {
    let c = hex(color);
    let dark = Color::rgba((c.red as f32 * 0.4) as u8, (c.green as f32 * 0.4) as u8, (c.blue as f32 * 0.4) as u8, 255);
    vec![LinearGradient::new(50., [GradientStop { position: 0., color: c }, GradientStop { position: 1., color: dark }]).into()]
}

/// Les recettes pour le gabarit : (indice, clé, nom, aide, icône, couleur).
pub fn presets() -> Vec<(usize, &'static str, String, String, &'static str, &'static str)> {
    PRESETS.iter().enumerate().map(|(i, p)| (i, p.key, crate::mapper::playlists::playlist_smart::preset_name(p), crate::mapper::playlists::playlist_smart::preset_hint(p), p.icon, p.color)).collect()
}

/// Les tailles d'un mix.
pub fn sizes() -> [i64; 3] {
    crate::mapper::playlists::playlist_smart::SIZES
}

/// Bouton principal : icône et libellé (enregistrer, créer le mix, créer la playlist).
pub fn cta(f: &SmartForm) -> (&'static str, String) {
    if f.playlist_id.is_some() {
        ("material-symbols:check-rounded", t("common.save"))
    } else if f.mix {
        ("material-symbols:shuffle-rounded", t("mix_view.create_cta"))
    } else {
        ("lucide:sparkles", t("smart.create_cta"))
    }
}

impl SmartForm {
    pub fn color_hex(&self) -> &'static str {
        COLORS[self.color.min(COLORS.len() - 1)]
    }

    pub fn icon_id(&self) -> &'static str {
        ICONS[self.icon.min(ICONS.len() - 1)]
    }

    /// L'arbre des règles prêt à afficher (vide pendant le chargement).
    pub fn tree(&self) -> GroupView {
        let Some(vocab) = &self.vocab else { return GroupView::default() };
        let labels: Vec<String> = vocab.fields.iter().map(field_label).collect();
        group_view(vocab, &labels, &self.group, Vec::new(), 0)
    }

    /// Les champs triables, précédés de « ordre naturel » ; et le choix courant.
    pub fn sort_options(&self) -> (Vec<String>, u64) {
        let mut out = vec![t("smart.natural_order"), t("smart.random")];
        let fields = self.vocab.as_ref().map(|v| v.fields.as_slice()).unwrap_or(&[]);
        out.extend(fields.iter().map(field_label));
        let current = match self.limit.sort.as_deref() {
            None => 0,
            Some("random") => 1,
            Some(k) => fields.iter().position(|f| f.key == k).map_or(0, |i| i as u64 + 2),
        };
        (out, current)
    }

    /// Le pied : « 1 234 » et « morceaux correspondent », puis la ligne d'appoint.
    pub fn count_lines(&self) -> Option<(String, String, Option<(String, bool)>)> {
        let n = self.count?;
        let one = intl::plural(n) == intl::Plural::One;
        let label = t(if one { "smart.matching_one" } else { "smart.matching_n" });
        let fr = intl::number;
        let extra = if self.mix && n == 0 {
            Some((t("mix_view.empty_pool"), true))
        } else if let (true, Some(c)) = (self.mix, self.limit.count) {
            Some((t("mix_view.drawn").replace("{n}", &fr(c.min(n))), false))
        } else if let Some(c) = self.limit.count.filter(|c| n > *c) {
            Some((t("smart.shown").replace("{n}", &fr(c)), false))
        } else {
            None
        };
        Some((fr(n), label, extra))
    }
}

impl Vocab {
    /// Le vocabulaire du moteur (`get_rule_vocabulary`), puis les tags recensés.
    pub fn from_json(voc: &Value, tags: &[(String, i64)]) -> Self {
        let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
        let mut fields: Vec<Field> = voc
            .get("fields")
            .and_then(Value::as_array)
            .map(|a| a.iter().map(|f| Field { key: text(f, "key"), label: text(f, "label"), kind: text(f, "kind") }).collect())
            .unwrap_or_default();
        fields.extend(tags.iter().map(|(key, _)| Field {
            key: format!("tag:{key}"),
            label: key.strip_prefix("custom:").map(str::to_string).unwrap_or_else(|| key.replace('_', " ")),
            kind: "text".into(),
        }));
        let ops = voc
            .get("operators")
            .and_then(Value::as_object)
            .map(|o| {
                o.iter()
                    .map(|(kind, list)| {
                        let list = list
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .map(|op| Op { key: text(op, "key"), label: text(op, "label"), arity: op.get("arity").and_then(Value::as_u64).unwrap_or(1) as u8 })
                                    .collect()
                            })
                            .unwrap_or_default();
                        (kind.clone(), list)
                    })
                    .collect()
            })
            .unwrap_or_default();
        Vocab { fields, ops }
    }

    /// Un champ inconnu (ou un tag) est du texte.
    pub fn ops_for(&self, field: &str) -> &[Op] {
        let kind = self.fields.iter().find(|f| f.key == field).map_or("text", |f| f.kind.as_str());
        self.ops.get(kind).or_else(|| self.ops.get("text")).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn arity(&self, field: &str, op: &str) -> u8 {
        self.ops_for(field).iter().find(|o| o.key == op).map_or(1, |o| o.arity)
    }

    /// Posée sur le premier champ proposé.
    pub fn default_rule(&self) -> Node {
        Node::Rule { field: self.fields.first().map_or("title".into(), |f| f.key.clone()), op: "contains".into(), value: json!("") }
    }

    /// Retire les conditions sans valeur et les groupes vidés.
    pub fn prune(&self, group: &Group) -> Group {
        let mut kept = Vec::new();
        for n in &group.rules {
            match n {
                Node::Group(g) => {
                    let sub = self.prune(g);
                    if !sub.rules.is_empty() {
                        kept.push(Node::Group(sub));
                    }
                }
                Node::Rule { field, op, value } => match self.arity(field, op) {
                    0 => kept.push(Node::Rule { field: field.clone(), op: op.clone(), value: Value::Null }),
                    2 => {
                        if value.as_array().is_some_and(|a| a.len() == 2) {
                            kept.push(n.clone());
                        }
                    }
                    _ => {
                        let filled = match value {
                            Value::String(s) => !s.trim().is_empty(),
                            Value::Null => false,
                            _ => true,
                        };
                        if filled {
                            kept.push(n.clone());
                        }
                    }
                },
            }
        }
        Group { all: group.all, rules: kept }
    }
}

impl Group {
    pub fn to_json(&self) -> Value {
        let rules: Vec<Value> = self
            .rules
            .iter()
            .map(|n| match n {
                Node::Group(g) => g.to_json(),
                Node::Rule { field, op, value } => json!({ "field": field, "op": op, "value": value }),
            })
            .collect();
        json!({ "match": if self.all { "all" } else { "any" }, "rules": rules })
    }

    pub fn from_json(v: &Value) -> Group {
        let rules = v
            .get("rules")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|n| {
                        if n.get("rules").is_some() {
                            Node::Group(Group::from_json(n))
                        } else {
                            let s = |k: &str| n.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
                            Node::Rule { field: s("field"), op: s("op"), value: n.get("value").cloned().unwrap_or(Value::Null) }
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        Group { all: v.get("match").and_then(Value::as_str) != Some("any"), rules }
    }
}

impl Limit {
    pub fn to_json(&self) -> Value {
        json!({ "count": self.count, "sort": self.sort, "desc": self.desc })
    }

    pub fn from_json(v: &Value) -> Limit {
        Limit {
            count: v.get("count").and_then(Value::as_i64),
            sort: v.get("sort").and_then(Value::as_str).map(str::to_string),
            desc: v.get("desc").and_then(Value::as_bool).unwrap_or(false),
        }
    }
}

impl Preset {
    pub fn group(&self) -> Group {
        Group { all: true, rules: self.rules.iter().map(|(f, o, v)| Node::Rule { field: (*f).into(), op: (*o).into(), value: json!(v) }).collect() }
    }

    pub fn limit(&self) -> Limit {
        Limit { count: self.count, sort: Some(self.sort.into()), desc: self.desc }
    }
}
