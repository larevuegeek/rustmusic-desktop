use crate::models::tracks::*;
use crate::mapper::tracks::fixed;
use crate::mapper::tracks::tag_title;

/// « 5 colonnes affichées ».
pub fn shown_label(n: usize) -> String {
    karbon_ui::intl::t_with("columns.shown", &[("n", &n.to_string())])
}

impl ColumnsPopin {
    /// « 6 colonnes affichées ».
    pub fn shown(&self) -> String {
        shown_label(self.choice.len())
    }

    /// Les colonnes affichées, dans l'ordre.
    pub fn rows(&self) -> Vec<ColumnRow> {
        let count = self.choice.len();
        self.choice
            .iter()
            .enumerate()
            .map(|(index, key)| ColumnRow {
                index,
                rank: index + 1,
                key: key.clone(),
                title: crate::controller::tracks::track_columns::title(key),
                first: index == 0,
                last: index + 1 == count,
            })
            .collect()
    }

    pub(crate) fn matches(&self, label: &str) -> bool {
        let f = self.filter.trim().to_lowercase();
        f.is_empty() || label.to_lowercase().contains(&f)
    }

    /// Champs de la bibliothèque filtrés : (clé, intitulé, coché).
    pub fn fields(&self) -> Vec<(String, String, bool)> {
        fixed().into_iter().map(|c| (c.key.clone(), c.label(), self.choice.contains(&c.key))).filter(|f| self.matches(&f.1)).collect()
    }

    /// Tags filtrés : (clé de colonne, intitulé, effectif, coché).
    pub fn tag_rows(&self) -> Vec<(String, String, i64, bool)> {
        self.tags
            .iter()
            .flatten()
            .map(|(k, n)| {
                let key = format!("tag:{k}");
                let on = self.choice.contains(&key);
                (key, tag_title(k), *n, on)
            })
            .filter(|r| self.matches(&r.1))
            .collect()
    }
}
