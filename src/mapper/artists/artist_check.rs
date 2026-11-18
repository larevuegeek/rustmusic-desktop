use crate::models::artists::*;
use karbon_ui::intl;
use karbon_ui::intl::t;
use crate::models::layout::Popin;
use crate::models::layout::PopinBody;

/// Libellé de la source d'une règle.
pub fn source_label(s: &str) -> String {
    match s {
        "tag" => t("artist_split.source_tag"),
        "user" => t("artist_split.source_user"),
        _ => "MusicBrainz".into(),
    }
}

/// Icône de la source d'une règle.
pub fn source_icon(s: &str) -> &'static str {
    match s {
        "tag" => "material-symbols:sell-outline-rounded",
        "user" => "material-symbols:person-check-outline-rounded",
        _ => "material-symbols:public",
    }
}

/// Avatar d'un nom : photo, sinon initiales sur la teinte tirée du nom.
pub fn avatar(a: &ArtistCheck, name: &str) -> (Option<String>, String, f32) {
    let (ini, hue) = crate::mapper::artists::avatar(name);
    (a.photo(name), ini, hue)
}

/// Le nom tient-il dans 30 caractères pile (coupé par le tag) ?
pub fn truncated(name: &str) -> bool {
    name.chars().count() == 30
}

/// Les pastilles d'une pile : trois noms au plus, deux et « +N » au-delà.
pub fn pile_shown(names: &[String]) -> Vec<(usize, &String)> {
    let shown = if names.len() > 3 { 2 } else { 3 };
    names.iter().take(shown).enumerate().collect()
}

/// « +4 », quand la pile ne montre pas tous les noms.
pub fn pile_more(names: &[String]) -> Option<String> {
    (names.len() > 3).then(|| format!("+{}", names.len() - 2))
}

impl RuleView {
    /// Le nom se sépare en plusieurs artistes.
    pub fn can_split(&self) -> bool {
        self.parts.len() >= 2
    }

    /// Les noms de la pile : les artistes séparés, ou le nom entier.
    pub fn pile_parts(&self) -> Vec<String> {
        if self.can_split() { self.parts.clone() } else { vec![self.name.clone()] }
    }

    /// La pile après décision (`whole` : le nom entier).
    pub fn decided_parts(&self, done: Option<&str>) -> Vec<String> {
        if done == Some("whole") { vec![self.name.clone()] } else { self.pile_parts() }
    }

    /// « Séparé en 3 artistes », ou « Gardé entier ».
    pub fn decided_label(&self, done: Option<&str>) -> String {
        if done == Some("split") {
            karbon_ui::intl::t_with("artist_split.split_into", &[("n", &self.parts.len().to_string())])
        } else {
            t("artist_split.done_whole")
        }
    }
}

impl ArtistCheck {
    /// Les tuiles du bilan.
    pub fn count_tiles(&self) -> Vec<CountTile> {
        let (split, whole, unknown) = self.counts();
        [
            (split, "artist_split.section_split", "material-symbols:call-split-rounded"),
            (whole, "artist_split.stat_whole", "material-symbols:person-outline-rounded"),
            (unknown, "artist_split.section_unknown", "material-symbols:help-outline-rounded"),
        ]
        .into_iter()
        .enumerate()
        .map(|(tone, (n, label, icon))| CountTile { value: intl::number(n), label: t(label), tone: tone as u8, icon })
        .collect()
    }

    /// Le nom en cours d'analyse, ou « Préparation… ».
    pub fn live_name(&self) -> String {
        if self.current.is_empty() {
            self.one.clone().unwrap_or_else(|| t("artist_split.live_preparing"))
        } else {
            self.current.clone()
        }
    }

    /// (séparés, gardés, à trancher) : en direct, ou du bilan.
    pub fn counts(&self) -> (i64, i64, i64) {
        if self.running {
            let c = |d: &str| self.live.iter().filter(|x| x.decision == d).count() as i64;
            (c("split"), c("whole"), c("unknown"))
        } else {
            self.report.as_ref().map_or((0, 0, 0), |r| (r.split.len() as i64, r.whole as i64, r.run_unknown))
        }
    }

    pub fn percent(&self) -> f32 {
        if self.total > 0 {
            (self.done as f32 / self.total as f32 * 100.).min(100.)
        } else {
            0.
        }
    }

    /// Titre de l'en-tête.
    pub fn title(&self) -> String {
        if self.running {
            t(if self.one.is_some() { "artist_split.live_one" } else { "artist_split.live_title" })
        } else if let Some(n) = self.report.as_ref().and_then(|r| r.checked) {
            t("artist_split.summary").replace("{n}", &intl::number(n))
        } else {
            t("artist_split.title")
        }
    }

    /// Sous-titre : avancement et temps restant, interruption, ou note.
    pub fn subtitle(&self) -> (String, bool) {
        if self.running {
            if self.total > 1 {
                let fr = intl::number;
                let mut s = t("artist_split.names_done").replace("{done}", &fr(self.done)).replace("{total}", &fr(self.total));
                if let (Some(st), true) = (self.started, self.done >= 3 && self.total > self.done) {
                    let rest = st.elapsed().as_secs_f64() / self.done as f64 * (self.total - self.done) as f64;
                    let eta = if rest < 60. { t("artist_split.eta_less") } else { t("artist_split.eta_min").replace("{n}", &((rest / 60.).ceil() as i64).to_string()) };
                    s = format!("{s}  ·  {eta}");
                }
                return (s, false);
            }
            return (self.one.clone().unwrap_or_else(|| t("artist_split.live_preparing")), false);
        }
        if self.report.as_ref().is_some_and(|r| r.error) || self.error.is_some() {
            return (t("artist_split.interrupted"), true);
        }
        (t("artist_split.footnote_short"), false)
    }

    /// Le fil en direct : les noms séparés ou à trancher, du plus récent, 40 au plus.
    pub fn feed(&self) -> Vec<LiveItem> {
        self.live.iter().rev().filter(|x| x.decision != "whole").take(40).cloned().collect()
    }

    pub fn remaining(&self) -> usize {
        self.report.as_ref().map_or(0, |r| r.split.iter().filter(|v| !self.undone.contains(&v.name)).count())
    }

    pub fn to_decide(&self) -> usize {
        self.report.as_ref().map_or(0, |r| r.unknown.iter().filter(|v| !self.decided.contains_key(&v.name)).count())
    }

    /// Photo connue d'un artiste.
    pub fn photo(&self, name: &str) -> Option<String> {
        self.images.get(&name.trim().to_lowercase()).cloned()
    }
}

impl Popin {
    pub fn is_artist_check(&self) -> bool {
        matches!(self.body, PopinBody::ArtistCheck)
    }
}
