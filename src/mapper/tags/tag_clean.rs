use crate::models::tags::*;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;
use crate::mapper::tags::FIELDS;

pub const RULES: [&str; 5] = ["underscores", "leading_number", "featuring", "spaces", "title_case"];

/// Pour le gabarit : (indice, règle).
pub fn rules() -> [&'static str; 5] {
    RULES
}

impl CleanState {
    /// Les règles, cochées ou non.
    pub fn rule_chips(&self) -> Vec<RuleChip> {
        rules()
            .iter()
            .enumerate()
            .map(|(index, rule)| RuleChip { index, label: t(&format!("clean.rule.{rule}")), active: self.rules.contains(rule) })
            .collect()
    }

    /// Nombre de corrections proposées.
    pub fn proposal_count(&self) -> usize {
        self.proposals.len()
    }
}

impl Proposal {
    pub fn file_name(&self) -> String {
        self.path.rsplit(['/', '\\']).next().unwrap_or(&self.path).to_string()
    }

    pub fn field_label(&self) -> String {
        t(&format!("tags.{}", FIELDS[self.field]))
    }
}

impl Popin {
    pub fn clean(&self) -> Option<&CleanState> {
        match &self.body {
            PopinBody::Clean(c) => Some(c),
            _ => None,
        }
    }
}
