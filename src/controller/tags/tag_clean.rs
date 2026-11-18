use crate::models::tags::*;
use crate::mapper::tags::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::models::layout::PopinSize;
use crate::models::tags::TagsMsg;
use crate::commands::tags::rename_command;

/// `TEXT_FIELDS` : ni numéros, ni totaux, ni année (indices dans `FIELDS`).
pub(crate) const TEXT_FIELDS: [usize; 7] = [1, 2, 3, 4, 6, 10, 11];

impl MusicApp {
    pub(crate) fn clean_state(&mut self) -> Option<&mut CleanState> {
        match self.views.popin.as_mut().map(|p| &mut p.body) {
            Some(PopinBody::Clean(c)) => Some(c),
            _ => None,
        }
    }

    pub(crate) fn clean_update(&mut self, msg: CleanMsg) -> bool {
        match msg {
            CleanMsg::Open => {
                if let Some(p) = self.views.tags.as_mut() {
                    p.menu = None;
                }
                self.open_popin(Popin {
                    title: t("clean.title"),
                    icon: Some("lucide:sparkles"),
                    body: PopinBody::Clean(Box::default()),
                    size: PopinSize::Xl,
                    bare: false,
                    flush: true,
                });
            }
            CleanMsg::Toggle(i) => {
                let Some(c) = self.clean_state() else { return false };
                let r = RULES[i];
                if let Some(k) = c.rules.iter().position(|x| *x == r) {
                    c.rules.remove(k);
                } else {
                    c.rules.push(r);
                }
                return self.clean_run();
            }
            CleanMsg::Apply => {
                let proposals = match self.clean_state() {
                    Some(c) if !c.proposals.is_empty() => c.proposals.clone(),
                    _ => return false,
                };
                self.views.popin = None;
                let Some(p) = self.views.tags.as_mut() else { return false };
                p.snapshot();
                for pr in &proposals {
                    if let Some(f) = p.files.iter_mut().find(|f| f.path == pr.path) {
                        f.apply(pr.field, pr.to.clone());
                    }
                }
                p.settle();
                return self.tags_update(TagsMsg::Cleaned(proposals.len()));
            }
        }
        true
    }

    /// `run` : les valeurs des fichiers visés, nettoyées par le moteur ; seules celles qui changent.
    pub(crate) fn clean_run(&mut self) -> bool {
        let values: Vec<(String, String)> = match self.views.tags.as_ref() {
            Some(p) => {
                let targets: Vec<_> = if p.selected.is_empty() { p.readable().collect() } else { p.targets() };
                targets
                    .iter()
                    .flat_map(|f| TEXT_FIELDS.iter().filter(|i| !f.value(**i).is_empty()).map(move |i| (format!("{}\u{0}{i}", f.path), f.value(*i).to_string())))
                    .collect()
            }
            None => Vec::new(),
        };
        let Some(c) = self.clean_state() else { return false };
        if c.rules.is_empty() || values.is_empty() {
            c.proposals.clear();
            return true;
        }
        c.seq += 1;
        c.computing = true;
        c.error = None;
        let (seq, rules) = (c.seq, c.rules.iter().map(|r| r.to_string()).collect::<Vec<_>>());
        let before: std::collections::HashMap<String, String> = values.iter().cloned().collect();
        self.backend.call(async move { rename_command::clean_tags(values, rules) }, move |me, r| {
            let Some(c) = me.clean_state() else { return false };
            if c.seq != seq {
                return false;
            }
            c.computing = false;
            match r {
                Ok(changed) => {
                    c.proposals = changed
                        .into_iter()
                        .filter_map(|(key, to)| {
                            let (path, field) = key.split_once('\u{0}')?;
                            Some(Proposal { path: path.to_string(), field: field.parse().ok()?, from: before.get(&key).cloned().unwrap_or_default(), to })
                        })
                        .collect();
                }
                Err(e) => c.error = Some(e),
            }
            true
        });
        true
    }
}
