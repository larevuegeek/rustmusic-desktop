//! Rendre un chemin acceptable pour le système de fichiers.
//!
//! Un motif produit du texte venu des tags, et les tags contiennent tout ce
//! qu'un être humain peut taper : `AC/DC`, `Where Are Ü Now`, `Q: Are We Not
//! Men?`. Écrit tel quel, `AC/DC` créerait un dossier `AC` contenant un
//! dossier `DC`.
//!
//! # On assainit pour la plateforme la plus stricte
//! Windows interdit neuf caractères, une vingtaine de noms hérités de MS-DOS,
//! et le point ou l'espace en fin de nom. Unix n'interdit que la barre
//! oblique. On applique **les règles de Windows partout** : une bibliothèque
//! posée sur un partage réseau se retrouve tôt ou tard lue depuis un poste
//! Windows, et un fichier nommé `Q: Are We Not Men?` y devient illisible.
//!
//! # La longueur est le piège qu'on n'attend pas
//! Sans les chemins longs activés, Windows s'arrête à 260 caractères. Une
//! arborescence `Artiste/Album (Année)/07 - Titre.flac` l'atteint pour de vrai
//! dès qu'on touche au metal, au classique ou aux coffrets — les noms y sont
//! longs et les trois niveaux s'additionnent.

/// Caractères que Windows interdit, et par quoi les remplacer.
///
/// # Un souligné n'est pas une traduction
/// La première version remplaçait tout par `_`, et un titre de medley
/// « Main Title / Chase The Red BMW / Krugerrand » devenait
/// « Main Title _ Chase The Red BMW _ Krugerrand ». Le nom était valide et
/// illisible : le souligné ne veut rien dire là où la barre oblique séparait
/// des parties.
///
/// On distingue donc deux familles.
///
/// **Les séparateurs** — `/`, `\`, `:`, `|` — deviennent un tiret entouré
/// d'espaces. C'est ce que font les éditeurs de tags établis, et ça se relit :
/// « Main Title - Chase The Red BMW - Krugerrand », « Vol. 2 - The Return ».
///
/// **Les ornements** — `?`, `*`, `"`, `<`, `>` — **disparaissent**.
/// « Are We Not Men? » n'a rien à gagner à devenir « Are We Not Men_ » : le
/// point d'interrogation ne portait pas de sens structurel, il décorait.
const SEPARATORS: [char; 4] = ['/', '\\', ':', '|'];
const ORNAMENTS: [char; 5] = ['?', '*', '"', '<', '>'];

/// Noms hérités de MS-DOS, toujours réservés — avec ou sans extension.
const RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Limite de longueur d'un chemin sous Windows, chemins longs désactivés.
pub const MAX_PATH: usize = 260;

/// Longueur maximale d'un composant, sur la quasi-totalité des systèmes.
pub const MAX_COMPONENT: usize = 255;

/// Réduit les espaces multiples et les tirets qui se suivent.
///
/// Nécessaire après substitution : « A / B » donne « A  -  B », et
/// « Titre - / - Suite » donne « Titre - - - Suite ».
fn tidy(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_space = false;
    let mut last_dash = false;

    for ch in text.chars() {
        match ch {
            ' ' | '\t' => {
                if !last_space {
                    out.push(' ');
                }
                last_space = true;
            }
            '-' => {
                // Un tiret qui suit un tiret, éventuellement séparé d'espaces,
                // est un artefact de substitution et non une intention.
                //
                // Quand on l'écarte, on laisse `last_space` tel quel : le
                // remettre à faux rouvrirait la porte aux espaces suivants, et
                // « Titre - / - Suite » rendait « Titre -   Suite ».
                if !last_dash {
                    out.push('-');
                    last_space = false;
                }
                last_dash = true;
            }
            _ => {
                out.push(ch);
                last_space = false;
                last_dash = false;
            }
        }
    }

    // Un tiret en tête ou en fin ne vient jamais du titre : il est né d'une
    // barre oblique en première ou dernière position.
    out.trim().trim_matches('-').trim().to_string()
}

/// Assainit **un composant** de chemin — un nom de dossier ou de fichier.
///
/// La barre oblique y est traitée comme un caractère à remplacer et non comme
/// un séparateur : c'est ce qui empêche `AC/DC` de devenir deux dossiers.
pub fn component(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if SEPARATORS.contains(&ch) {
            out.push_str(" - ");
        } else if ORNAMENTS.contains(&ch) {
            // Rien : l'ornement disparaît sans laisser de trace.
        } else if (ch as u32) < 0x20 {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }

    let mut out = tidy(&out);

    // Le point et l'espace finaux sont silencieusement retirés par Windows :
    // le fichier existe alors sous un nom qui n'est pas celui qu'on croit, et
    // toute vérification ultérieure échoue.
    out = out.trim_end_matches([' ', '.']).trim_start().to_string();

    // Un nom réservé reste réservé même suivi d'une extension : `con.flac`
    // est refusé comme `con`.
    let stem = out.split('.').next().unwrap_or("").to_lowercase();
    if RESERVED.contains(&stem.as_str()) {
        out = format!("_{out}");
    }

    if out.chars().count() > MAX_COMPONENT {
        out = out.chars().take(MAX_COMPONENT).collect();
        out = out.trim_end_matches([' ', '.']).to_string();
    }

    out
}

/// Assainit un chemin relatif produit par un motif.
///
/// Les barres obliques du **motif** séparent les dossiers ; celles venues des
/// tags ont déjà été neutralisées composant par composant.
pub fn relative_path(path: &str) -> String {
    path.split(['/', '\\'])
        .map(str::trim)
        .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        .map(component)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

/// Ce qui empêcherait d'écrire à ce chemin.
#[derive(Debug, Clone, PartialEq)]
pub enum PathIssue {
    /// Le chemin complet dépasse la limite de Windows.
    TooLong { length: usize },
    /// Le motif n'a produit aucun nom exploitable.
    Empty,
}

impl std::fmt::Display for PathIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathIssue::TooLong { length } => write!(
                f,
                "Chemin trop long : {length} caractères pour {MAX_PATH} au maximum."
            ),
            PathIssue::Empty => write!(f, "Le motif ne produit aucun nom."),
        }
    }
}

/// Vérifie un chemin absolu final.
pub fn check(full_path: &str) -> Option<PathIssue> {
    let trimmed = full_path.trim();
    if trimmed.is_empty() {
        return Some(PathIssue::Empty);
    }
    // On compte en caractères et non en octets : c'est ainsi que Windows
    // mesure, et un titre accentué pèse deux octets par lettre en UTF-8.
    let length = trimmed.chars().count();
    if length > MAX_PATH {
        return Some(PathIssue::TooLong { length });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_separateurs_deviennent_des_tirets() {
        assert_eq!(component("AC/DC"), "AC - DC");
        // Le cas qui a motivé ce choix : un medley séparé par des barres.
        assert_eq!(
            component("Main Title / Chase The Red BMW / Krugerrand"),
            "Main Title - Chase The Red BMW - Krugerrand"
        );
        assert_eq!(component("Vol. 2: The Return"), "Vol. 2 - The Return");
    }

    #[test]
    fn les_ornements_disparaissent() {
        // « Are We Not Men_ » n'apporte rien : le point d'interrogation
        // décorait, il ne structurait pas.
        assert_eq!(component("Are We Not Men?"), "Are We Not Men");
        assert_eq!(component("Wish You Were Here*"), "Wish You Were Here");
        assert_eq!(component("Say \"Hello\""), "Say Hello");
    }

    #[test]
    fn pas_de_tirets_ni_d_espaces_en_trop() {
        assert_eq!(component("A  /  B"), "A - B");
        assert_eq!(component("Titre - / - Suite"), "Titre - Suite");
        // Une barre en tête ou en fin ne doit pas laisser de tiret orphelin.
        assert_eq!(component("/Titre/"), "Titre");
    }

    #[test]
    fn un_tiret_voulu_reste() {
        assert_eq!(component("01 - Titre"), "01 - Titre");
    }

    #[test]
    fn le_point_final_disparait() {
        // Windows le retire lui-même : le fichier existerait sous un autre nom.
        assert_eq!(component("Album."), "Album");
        assert_eq!(component("Album "), "Album");
    }

    #[test]
    fn les_points_internes_restent() {
        assert_eq!(component("R.E.M."), "R.E.M");
        assert_eq!(component("titre.flac"), "titre.flac");
    }

    #[test]
    fn noms_reserves() {
        assert_eq!(component("CON"), "_CON");
        assert_eq!(component("con.flac"), "_con.flac");
        assert_eq!(component("COM1"), "_COM1");
        // Un nom qui commence pareil sans en être un n'est pas touché.
        assert_eq!(component("Concert"), "Concert");
    }

    #[test]
    fn caracteres_de_controle() {
        // Un espace plutôt qu'un souligné, réduit ensuite comme les autres.
        assert_eq!(component("ti\u{7}tre"), "ti tre");
    }

    #[test]
    fn composant_trop_long() {
        let name = "a".repeat(300);
        assert_eq!(component(&name).chars().count(), MAX_COMPONENT);
    }

    #[test]
    fn chemin_relatif_conserve_ses_dossiers() {
        assert_eq!(
            relative_path("Angèle/Brol (2018)/03 - La thune.flac"),
            "Angèle/Brol (2018)/03 - La thune.flac"
        );
    }

    #[test]
    fn une_barre_venue_d_un_tag_ne_cree_pas_de_dossier() {
        // Le motif fait le découpage ; le tag ne doit pas pouvoir en ajouter.
        let rendered = format!("{}/album", component("AC/DC"));
        assert_eq!(relative_path(&rendered), "AC - DC/album");
    }

    #[test]
    fn remontees_de_dossier_ecartees() {
        assert_eq!(relative_path("../../etc/passwd"), "etc/passwd");
        assert_eq!(relative_path("a/./b"), "a/b");
    }

    #[test]
    fn segments_vides_ecartes() {
        assert_eq!(relative_path("a//b/"), "a/b");
    }

    #[test]
    fn longueur_de_chemin() {
        let long = format!("C:\\Musique\\{}", "a".repeat(300));
        assert!(matches!(check(&long), Some(PathIssue::TooLong { .. })));
        assert_eq!(check("C:\\Musique\\a.flac"), None);
    }

    #[test]
    fn longueur_comptee_en_caracteres() {
        // 200 lettres accentuées font 400 octets mais 200 caractères : c'est
        // ainsi que Windows mesure.
        let path = format!("C:\\{}", "é".repeat(200));
        assert_eq!(check(&path), None);
    }

    #[test]
    fn chemin_vide() {
        assert_eq!(check("   "), Some(PathIssue::Empty));
    }
}
