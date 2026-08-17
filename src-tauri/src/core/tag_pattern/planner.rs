//! De N fichiers et un motif, un plan de déplacement — et ce qui l'empêche.
//!
//! # Renommer et restructurer sont la même opération
//! Renommer garde le dossier et change le nom ; restructurer recompose le
//! chemin entier. Dans les deux cas on calcule une cible et on déplace. En
//! faire deux moteurs voudrait dire écrire deux fois la détection de
//! collisions, deux fois la vérification de longueur, deux fois la logique de
//! transaction — et corriger les bugs deux fois.
//!
//! # Rien ne s'exécute ici
//! Ce module ne touche ni au disque ni à la base. Il rend un plan, avec pour
//! chaque fichier ce qui se passerait et ce qui l'empêche. C'est la condition
//! posée par la roadmap : corriger cinq mille fichiers avec un motif erroné
//! n'est pas rattrapable à la main, donc l'aperçu à blanc n'est pas du confort.
//!
//! # Les trois obstacles, qui ne se valent pas
//! - **Collision** : deux fichiers visent la même cible. Insoluble sans
//!   changer le motif — c'est le cas du titre en double sur un album sans
//!   numéros.
//! - **Occupé** : la cible existe déjà et n'appartient pas au lot. Insoluble
//!   aussi : on ne va pas écraser un fichier que personne n'a désigné.
//! - **Chaîne** : la cible est le fichier d'un autre membre du lot, qui va
//!   lui-même bouger. Parfaitement soluble — il suffit de déplacer dans le
//!   bon ordre — mais pas en aveugle, d'où la distinction.

use std::collections::{HashMap, HashSet};

use super::pattern::{render, Token};
use super::sanitize::{self, PathIssue};

/// Ce que le motif recompose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Le nom du fichier seul ; le dossier ne change pas.
    Rename,
    /// Le chemin entier, relatif à la racine de la bibliothèque.
    Restructure,
}

/// Un fichier à replacer, avec les valeurs qui alimenteront le motif.
#[derive(Debug, Clone)]
pub struct FileInput {
    /// Chemin absolu actuel.
    pub path: String,
    pub values: HashMap<String, String>,
}

/// Ce qui empêche un déplacement.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanIssue {
    /// Champs cités par le motif et vides sur ce fichier.
    MissingFields(Vec<String>),
    /// Un autre fichier du lot vise la même cible.
    Collision,
    /// La cible existe déjà, hors du lot.
    Occupied,
    /// La cible est la source d'un autre membre du lot.
    Chain,
    TooLong { length: usize },
    Empty,
}

impl PlanIssue {
    /// Un obstacle bloque-t-il, ou se règle-t-il à l'exécution ?
    pub fn blocking(&self) -> bool {
        !matches!(self, PlanIssue::Chain)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PlanIssue::MissingFields(_) => "missing_fields",
            PlanIssue::Collision => "collision",
            PlanIssue::Occupied => "occupied",
            PlanIssue::Chain => "chain",
            PlanIssue::TooLong { .. } => "too_long",
            PlanIssue::Empty => "empty",
        }
    }
}

/// Ce qui arriverait à un fichier.
#[derive(Debug, Clone)]
pub struct PlannedMove {
    pub from: String,
    pub to: String,
    /// La cible est identique à la source : rien à faire.
    pub unchanged: bool,
    pub issues: Vec<PlanIssue>,
}

impl PlannedMove {
    pub fn blocked(&self) -> bool {
        self.issues.iter().any(PlanIssue::blocking)
    }

    /// Prêt à être exécuté — donc ni bloqué, ni sans effet.
    pub fn ready(&self) -> bool {
        !self.blocked() && !self.unchanged
    }
}

/// Le plan complet.
#[derive(Debug, Clone)]
pub struct MovePlan {
    pub moves: Vec<PlannedMove>,
}

impl MovePlan {
    pub fn ready(&self) -> usize {
        self.moves.iter().filter(|m| m.ready()).count()
    }
    pub fn blocked(&self) -> usize {
        self.moves.iter().filter(|m| m.blocked()).count()
    }
    pub fn unchanged(&self) -> usize {
        self.moves.iter().filter(|m| m.unchanged && !m.blocked()).count()
    }
}

/// Dossier parent d'un chemin absolu, sans le séparateur final.
fn parent_of(path: &str) -> String {
    match path.rfind(['/', '\\']) {
        Some(i) => path[..i].to_string(),
        None => String::new(),
    }
}

/// Assemble un chemin absolu à partir d'une base et d'un chemin relatif.
fn join(base: &str, relative: &str) -> String {
    let base = base.trim_end_matches(['/', '\\']);
    if base.is_empty() {
        return relative.to_string();
    }
    // On conserve le séparateur de la base : sur un chemin Windows, mélanger
    // les deux styles fonctionne mais rend illisibles les messages d'erreur et
    // les comparaisons de chaînes.
    let sep = if base.contains('\\') && !base.contains('/') { '\\' } else { '/' };
    format!("{base}{sep}{}", relative.replace('/', &sep.to_string()))
}

/// Clé de comparaison de deux chemins.
///
/// Sous Windows, `A.flac` et `a.flac` désignent le même fichier : comparer
/// exactement laisserait passer un renommage qui en écrase un autre. Sous
/// Unix ce sont deux fichiers distincts, et confondre les deux interdirait
/// des renommages parfaitement légitimes.
fn key(path: &str, case_insensitive: bool) -> String {
    let unified = path.replace('\\', "/");
    if case_insensitive {
        unified.to_lowercase()
    } else {
        unified
    }
}

/// Construit le plan.
///
/// `exists` interroge le disque : le module reste sans entrées-sorties, et les
/// tests fournissent une simple fermeture sur un ensemble.
pub fn plan(
    files: &[FileInput],
    tokens: &[Token],
    mode: Mode,
    library_root: &str,
    case_insensitive: bool,
    exists: impl Fn(&str) -> bool,
) -> MovePlan {
    let mut moves: Vec<PlannedMove> = Vec::with_capacity(files.len());

    for file in files {
        let rendered = render(tokens, &file.values);
        let mut issues = Vec::new();

        if !rendered.missing.is_empty() {
            issues.push(PlanIssue::MissingFields(rendered.missing.clone()));
        }

        // L'assainissement diffère selon le mode, et c'est essentiel : en
        // renommage tout le rendu est **un seul nom**, donc une barre oblique
        // venue d'un tag — « AC/DC » — doit être neutralisée. En
        // restructuration, ce sont les barres du motif qui découpent
        // l'arborescence, et seules celles venues des tags sont neutralisées.
        let relative = match mode {
            Mode::Rename => sanitize::component(&rendered.text),
            Mode::Restructure => sanitize::relative_path(&rendered.text),
        };
        let target = match mode {
            Mode::Rename => join(&parent_of(&file.path), &relative),
            Mode::Restructure => join(library_root, &relative),
        };

        if relative.is_empty() {
            issues.push(PlanIssue::Empty);
        } else if let Some(PathIssue::TooLong { length }) = sanitize::check(&target) {
            issues.push(PlanIssue::TooLong { length });
        }

        let unchanged = key(&target, case_insensitive) == key(&file.path, case_insensitive);

        moves.push(PlannedMove {
            from: file.path.clone(),
            to: target,
            unchanged,
            issues,
        });
    }

    // ─── Collisions entre membres du lot ───
    let mut targets: HashMap<String, usize> = HashMap::new();
    let mut colliding: HashSet<String> = HashSet::new();
    for m in &moves {
        if m.unchanged {
            continue;
        }
        let k = key(&m.to, case_insensitive);
        let count = targets.entry(k.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            colliding.insert(k);
        }
    }

    // ─── Cibles occupées ───
    let sources: HashSet<String> = moves
        .iter()
        .map(|m| key(&m.from, case_insensitive))
        .collect();

    for m in moves.iter_mut() {
        if m.unchanged {
            continue;
        }
        let k = key(&m.to, case_insensitive);
        if colliding.contains(&k) {
            m.issues.push(PlanIssue::Collision);
            continue;
        }
        if sources.contains(&k) {
            // La cible appartient au lot : elle va se libérer, à condition de
            // déplacer dans le bon ordre.
            m.issues.push(PlanIssue::Chain);
        } else if exists(&m.to) {
            m.issues.push(PlanIssue::Occupied);
        }
    }

    MovePlan { moves }
}

/// Ordonne les déplacements pour que les chaînes se résolvent.
///
/// `A → B` et `B → C` : déplacer A en premier écraserait B. On sort donc les
/// déplacements dont la cible n'est la source de personne, puis on recommence.
/// Ce qui reste est un **cycle** — `A → B` et `B → A` — que seul un nom
/// temporaire dénoue, et qu'on rend à part pour que l'exécutant le sache.
pub fn order(moves: &[PlannedMove], case_insensitive: bool) -> (Vec<usize>, Vec<usize>) {
    let ready: Vec<usize> = (0..moves.len()).filter(|&i| moves[i].ready()).collect();

    let mut remaining: HashSet<usize> = ready.iter().copied().collect();
    let mut ordered = Vec::with_capacity(ready.len());

    loop {
        // Sources encore occupées par un déplacement non effectué.
        let blocked_by: HashSet<String> = remaining
            .iter()
            .map(|&i| key(&moves[i].from, case_insensitive))
            .collect();

        let free: Vec<usize> = remaining
            .iter()
            .copied()
            .filter(|&i| !blocked_by.contains(&key(&moves[i].to, case_insensitive)))
            .collect();

        if free.is_empty() {
            break;
        }

        // Ordre stable : sans tri, l'ordre vient d'un ensemble de hachage et
        // change d'une exécution à l'autre — un journal d'annulation illisible.
        let mut free = free;
        free.sort_unstable();
        for i in &free {
            remaining.remove(i);
        }
        ordered.extend(free);
    }

    let mut cycles: Vec<usize> = remaining.into_iter().collect();
    cycles.sort_unstable();
    (ordered, cycles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tag_pattern::pattern::parse;

    fn file(path: &str, pairs: &[(&str, &str)]) -> FileInput {
        FileInput {
            path: path.into(),
            values: pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn none(_: &str) -> bool {
        false
    }

    #[test]
    fn renommage_garde_le_dossier() {
        let tokens = parse("{track:02} - {title}.{ext}").unwrap();
        let files = vec![file(
            "/musique/Album/piste1.flac",
            &[("track", "1"), ("title", "Ouverture"), ("ext", "flac")],
        )];
        let plan = plan(&files, &tokens, Mode::Rename, "/musique", false, none);
        assert_eq!(plan.moves[0].to, "/musique/Album/01 - Ouverture.flac");
        assert_eq!(plan.ready(), 1);
    }

    #[test]
    fn restructuration_recompose_le_chemin() {
        let tokens = parse("{artist}/{album}/{track:02} - {title}.{ext}").unwrap();
        let files = vec![file(
            "/musique/vrac/x.flac",
            &[
                ("artist", "Angèle"),
                ("album", "Brol"),
                ("track", "3"),
                ("title", "La thune"),
                ("ext", "flac"),
            ],
        )];
        let plan = plan(&files, &tokens, Mode::Restructure, "/musique", false, none);
        assert_eq!(plan.moves[0].to, "/musique/Angèle/Brol/03 - La thune.flac");
    }

    #[test]
    fn une_cible_identique_ne_bouge_pas() {
        let tokens = parse("{title}.{ext}").unwrap();
        let files = vec![file("/m/Titre.flac", &[("title", "Titre"), ("ext", "flac")])];
        let plan = plan(&files, &tokens, Mode::Rename, "/m", false, none);
        assert!(plan.moves[0].unchanged);
        assert_eq!(plan.ready(), 0);
        assert_eq!(plan.unchanged(), 1);
    }

    #[test]
    fn collision_entre_deux_membres_du_lot() {
        // Même titre, pas de numéro : le piège que la roadmap signale.
        let tokens = parse("{title}.{ext}").unwrap();
        let files = vec![
            file("/m/a.flac", &[("title", "Intro"), ("ext", "flac")]),
            file("/m/b.flac", &[("title", "Intro"), ("ext", "flac")]),
        ];
        let plan = plan(&files, &tokens, Mode::Rename, "/m", false, none);
        assert_eq!(plan.blocked(), 2);
        assert!(plan.moves[0].issues.contains(&PlanIssue::Collision));
    }

    #[test]
    fn cible_occupee_hors_du_lot() {
        let tokens = parse("{title}.{ext}").unwrap();
        let files = vec![file("/m/a.flac", &[("title", "Intro"), ("ext", "flac")])];
        let plan = plan(&files, &tokens, Mode::Rename, "/m", false, |p| {
            p == "/m/Intro.flac"
        });
        assert!(plan.moves[0].issues.contains(&PlanIssue::Occupied));
        assert!(plan.moves[0].blocked());
    }

    #[test]
    fn une_chaine_n_est_pas_un_blocage() {
        // a → b et b → c : la cible de a se libère, il suffit d'ordonner.
        let tokens = parse("{title}.{ext}").unwrap();
        let files = vec![
            file("/m/a.flac", &[("title", "b"), ("ext", "flac")]),
            file("/m/b.flac", &[("title", "c"), ("ext", "flac")]),
        ];
        let plan = plan(&files, &tokens, Mode::Rename, "/m", false, |p| {
            p == "/m/b.flac"
        });
        assert!(plan.moves[0].issues.contains(&PlanIssue::Chain));
        assert!(!plan.moves[0].blocked());
        assert_eq!(plan.ready(), 2);

        let (ordered, cycles) = order(&plan.moves, false);
        // b → c d'abord, pour libérer la place de a → b.
        assert_eq!(ordered, vec![1, 0]);
        assert!(cycles.is_empty());
    }

    #[test]
    fn un_cycle_est_rendu_a_part() {
        // a → b et b → a : aucun ordre ne suffit, il faut un nom temporaire.
        let tokens = parse("{title}.{ext}").unwrap();
        let files = vec![
            file("/m/a.flac", &[("title", "b"), ("ext", "flac")]),
            file("/m/b.flac", &[("title", "a"), ("ext", "flac")]),
        ];
        let plan = plan(&files, &tokens, Mode::Rename, "/m", false, none);
        let (ordered, cycles) = order(&plan.moves, false);
        assert!(ordered.is_empty());
        assert_eq!(cycles, vec![0, 1]);
    }

    #[test]
    fn champ_manquant_bloque() {
        let tokens = parse("{album}/{title}.{ext}").unwrap();
        let files = vec![file("/m/a.flac", &[("title", "Intro"), ("ext", "flac")])];
        let plan = plan(&files, &tokens, Mode::Restructure, "/m", false, none);
        assert!(matches!(
            plan.moves[0].issues[0],
            PlanIssue::MissingFields(_)
        ));
        assert!(plan.moves[0].blocked());
    }

    #[test]
    fn chemin_trop_long() {
        // Un nom seul ne peut pas dépasser : l'assainissement le tronque à 255.
        // La limite se franchit par la **profondeur**, comme le décrit la
        // roadmap — artiste, album et titre longs qui s'additionnent.
        let tokens = parse("{artist}/{album}/{title}.{ext}").unwrap();
        let long = "a".repeat(100);
        let files = vec![file(
            "/m/x.flac",
            &[
                ("artist", &long),
                ("album", &long),
                ("title", &long),
                ("ext", "flac"),
            ],
        )];
        let plan = plan(&files, &tokens, Mode::Restructure, "/musique", false, none);
        assert!(plan.moves[0]
            .issues
            .iter()
            .any(|i| matches!(i, PlanIssue::TooLong { .. })));
        assert!(plan.moves[0].blocked());
    }

    #[test]
    fn sous_windows_la_casse_ne_distingue_pas_deux_fichiers() {
        let tokens = parse("{title}.{ext}").unwrap();
        let files = vec![
            file("/m/a.flac", &[("title", "Intro"), ("ext", "flac")]),
            file("/m/b.flac", &[("title", "INTRO"), ("ext", "flac")]),
        ];
        // Sensible à la casse : deux fichiers distincts, aucune collision.
        assert_eq!(plan(&files, &tokens, Mode::Rename, "/m", false, none).blocked(), 0);
        // Insensible : c'est le même fichier, donc une collision.
        assert_eq!(plan(&files, &tokens, Mode::Rename, "/m", true, none).blocked(), 2);
    }

    #[test]
    fn un_tag_ne_peut_pas_creer_de_dossier_en_mode_renommage() {
        let tokens = parse("{artist} - {title}.{ext}").unwrap();
        let files = vec![file(
            "/m/x.flac",
            &[("artist", "AC/DC"), ("title", "T.N.T"), ("ext", "flac")],
        )];
        let plan = plan(&files, &tokens, Mode::Rename, "/m", false, none);
        assert_eq!(plan.moves[0].to, "/m/AC - DC - T.N.T.flac");
    }

    #[test]
    fn le_separateur_windows_est_conserve() {
        let tokens = parse("{artist}/{title}.{ext}").unwrap();
        let files = vec![file(
            "C:\\Musique\\vrac\\x.flac",
            &[("artist", "Angèle"), ("title", "Flou"), ("ext", "flac")],
        )];
        let plan = plan(&files, &tokens, Mode::Restructure, "C:\\Musique", false, none);
        assert_eq!(plan.moves[0].to, "C:\\Musique\\Angèle\\Flou.flac");
    }
}
