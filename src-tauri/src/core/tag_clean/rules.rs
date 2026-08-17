//! Règles de nettoyage des tags.
//!
//! # Chacune est indépendante et facultative
//! Aucune ne s'applique d'office. Ce qui est du bruit chez l'un est une
//! intention chez l'autre : `AC/DC` en capitales n'est pas une faute de casse,
//! `___` dans un titre de musique industrielle peut être voulu. Une règle
//! qu'on ne peut pas décocher finit par abîmer une bibliothèque soignée.
//!
//! # Elles ne rendent qu'une proposition
//! Rien n'est écrit ici. Le résultat va dans les modifications en attente de
//! l'atelier, où on le relit avant d'écrire — et où le journal le rattrape si
//! on s'est trompé.
//!
//! # Ce qu'on ne fait pas
//! Pas de mise en capitales automatique du français : les règles de casse d'un
//! titre diffèrent d'une langue à l'autre, et appliquer les conventions
//! anglaises à un catalogue francophone produit des « Le Grand Bleu » en
//! « Le Grand Bleu » ici, mais des « La Thune » là où il fallait « La thune ».
//! La règle existe, elle est explicitement anglaise, et elle est décochée.

/// Les règles disponibles, dans l'ordre où elles s'appliquent.
///
/// L'ordre compte : retirer le numéro de piste avant de recoller les espaces
/// évite de laisser un blanc en tête.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// Souligné et point remplacés par des espaces — la marque d'un nom de
    /// fichier promu en titre.
    Underscores,
    /// Numéro de piste en tête de titre : `01 - Titre` devient `Titre`.
    LeadingNumber,
    /// `Feat`, `FT.`, `featuring` ramenés à `feat.`
    Featuring,
    /// Espaces multiples réduits, extrémités coupées.
    Spaces,
    /// Capitales initiales à l'anglaise, articles courts exceptés.
    TitleCase,
}

impl Rule {
    pub fn as_str(self) -> &'static str {
        match self {
            Rule::Underscores => "underscores",
            Rule::LeadingNumber => "leading_number",
            Rule::Featuring => "featuring",
            Rule::Spaces => "spaces",
            Rule::TitleCase => "title_case",
        }
    }

    pub fn parse(name: &str) -> Option<Rule> {
        Some(match name {
            "underscores" => Rule::Underscores,
            "leading_number" => Rule::LeadingNumber,
            "featuring" => Rule::Featuring,
            "spaces" => Rule::Spaces,
            "title_case" => Rule::TitleCase,
            _ => return None,
        })
    }
}

/// Toutes les règles, dans leur ordre d'application.
pub const ALL: [Rule; 5] = [
    Rule::Underscores,
    Rule::LeadingNumber,
    Rule::Featuring,
    Rule::Spaces,
    Rule::TitleCase,
];

/// Mots qui restent en minuscules au milieu d'un titre anglais.
const SMALL_WORDS: [&str; 20] = [
    "a", "an", "and", "as", "at", "but", "by", "for", "in", "nor", "of", "on", "or", "so", "the",
    "to", "up", "vs", "via", "yet",
];

/// Souligné et point isolé remplacés par des espaces.
fn underscores(text: &str) -> String {
    // Le point n'est remplacé qu'entre deux lettres, et jamais s'il précède
    // une extension : `R.E.M.` doit survivre, `Ma.Chanson` non. On s'en tient
    // donc au souligné, seul cas sans ambiguïté.
    text.replace('_', " ")
}

/// Retire un numéro de piste en tête de titre.
fn leading_number(text: &str) -> String {
    let trimmed = text.trim_start();
    let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();

    // Deux chiffres au plus : un titre qui commence par une année — « 1979 »,
    // « 2000 Miles » — n'est pas une numérotation.
    if digits.is_empty() || digits.len() > 2 {
        return text.to_string();
    }

    let rest = trimmed[digits.len()..].trim_start();
    // Un séparateur est exigé : sans lui, « 7 Nation Army » perdrait son sept.
    let rest = rest
        .strip_prefix('-')
        .or_else(|| rest.strip_prefix('.'))
        .or_else(|| rest.strip_prefix(')'))
        .or_else(|| rest.strip_prefix('_'));

    match rest {
        Some(rest) if !rest.trim().is_empty() => rest.trim_start().to_string(),
        _ => text.to_string(),
    }
}

/// Ramène les variantes de « featuring » à une forme unique.
fn featuring(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(pos) = find_featuring(rest) {
        let (start, len) = pos;
        out.push_str(&rest[..start]);
        out.push_str("feat.");
        rest = &rest[start + len..];
    }
    out.push_str(rest);
    out
}

/// Position et longueur de la prochaine variante de « featuring ».
///
/// Le point final fait partie de la correspondance quand il est là. Sans ça,
/// `feat.` se faisait reconnaître comme `feat` — dont le point suivant n'est
/// pas alphanumérique, donc frontière de mot valide — et devenait `feat..`.
fn find_featuring(text: &str) -> Option<(usize, usize)> {
    let lower = text.to_lowercase();
    let bytes = lower.as_bytes();

    // Du plus long au plus court : « featuring » d'abord, sinon on ne
    // remplacerait que son préfixe « feat ».
    for needle in ["featuring", "feat", "ft"] {
        let mut from = 0usize;
        while let Some(found) = lower[from..].find(needle) {
            let start = from + found;
            let mut end = start + needle.len();

            // Frontière avant : sans elle, « Defeat » serait mutilé.
            let before_ok = start == 0
                || !lower[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric());

            // Le point appartient à la variante : « feat. » se remplace en
            // entier, pas seulement ses quatre premières lettres.
            if bytes.get(end) == Some(&b'.') {
                end += 1;
            }

            // Frontière après : « Feature » ne doit pas passer pour « feat ».
            let after_ok = end >= lower.len()
                || !lower[end..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_alphanumeric());

            if before_ok && after_ok {
                // Déjà sous la forme voulue : on passe, sinon on tournerait en
                // rond en la remplaçant par elle-même.
                if &text[start..end] != "feat." {
                    return Some((start, end - start));
                }
            }
            from = end.max(start + 1);
        }
    }
    None
}

/// Espaces multiples réduits, extrémités coupées.
fn spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Capitales initiales à l'anglaise.
fn title_case(text: &str) -> String {
    let words: Vec<&str> = text.split(' ').collect();
    let last = words.len().saturating_sub(1);

    words
        .iter()
        .enumerate()
        .map(|(i, word)| {
            if word.is_empty() {
                return String::new();
            }
            // Un mot déjà tout en capitales est un sigle — `DNA`, `USA`, `NYC` —
            // et le rabaisser en `Dna` serait une régression, pas un nettoyage.
            if word.chars().filter(|c| c.is_alphabetic()).count() > 1
                && word.chars().all(|c| !c.is_lowercase())
            {
                return word.to_string();
            }

            let lower = word.to_lowercase();
            if i != 0 && i != last && SMALL_WORDS.contains(&lower.as_str()) {
                return lower;
            }

            let mut chars = lower.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Applique les règles retenues, dans l'ordre.
pub fn apply(text: &str, rules: &[Rule]) -> String {
    let mut out = text.to_string();
    for rule in ALL {
        if !rules.contains(&rule) {
            continue;
        }
        out = match rule {
            Rule::Underscores => underscores(&out),
            Rule::LeadingNumber => leading_number(&out),
            Rule::Featuring => featuring(&out),
            Rule::Spaces => spaces(&out),
            Rule::TitleCase => title_case(&out),
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn souligne_remplace() {
        assert_eq!(apply("Ma_Chanson", &[Rule::Underscores]), "Ma Chanson");
    }

    #[test]
    fn numero_de_piste_retire() {
        let r = [Rule::LeadingNumber];
        assert_eq!(apply("01 - Titre", &r), "Titre");
        assert_eq!(apply("01. Titre", &r), "Titre");
        assert_eq!(apply("7) Titre", &r), "Titre");
    }

    #[test]
    fn un_titre_qui_commence_par_un_chiffre_est_epargne() {
        let r = [Rule::LeadingNumber];
        // Sans séparateur, ce n'est pas une numérotation.
        assert_eq!(apply("7 Nation Army", &r), "7 Nation Army");
        // Une année n'est pas un numéro de piste.
        assert_eq!(apply("1979", &r), "1979");
        assert_eq!(apply("2000 - Miles", &r), "2000 - Miles");
    }

    #[test]
    fn un_numero_seul_reste() {
        // Retirer le « 01 » laisserait un titre vide.
        assert_eq!(apply("01 - ", &[Rule::LeadingNumber]), "01 - ");
    }

    #[test]
    fn featuring_normalise() {
        let r = [Rule::Featuring];
        assert_eq!(apply("Titre ft Angèle", &r), "Titre feat. Angèle");
        assert_eq!(apply("Titre FT. Angèle", &r), "Titre feat. Angèle");
        assert_eq!(apply("Titre Featuring Angèle", &r), "Titre feat. Angèle");
        assert_eq!(apply("Titre Feat Angèle", &r), "Titre feat. Angèle");
    }

    #[test]
    fn featuring_deja_correct_inchange() {
        assert_eq!(
            apply("Titre feat. Angèle", &[Rule::Featuring]),
            "Titre feat. Angèle"
        );
    }

    #[test]
    fn un_mot_qui_contient_feat_est_epargne() {
        let r = [Rule::Featuring];
        assert_eq!(apply("Defeat", &r), "Defeat");
        assert_eq!(apply("Aftermath", &r), "Aftermath");
        assert_eq!(apply("Feature Presentation", &r), "Feature Presentation");
    }

    #[test]
    fn espaces_reduits() {
        assert_eq!(apply("  Trop   d'espaces  ", &[Rule::Spaces]), "Trop d'espaces");
    }

    #[test]
    fn capitales_a_l_anglaise() {
        let r = [Rule::TitleCase];
        assert_eq!(apply("the dark side of the moon", &r), "The Dark Side of the Moon");
        // Le dernier mot prend la capitale, même petit.
        assert_eq!(apply("what are you waiting for", &r), "What Are You Waiting For");
    }

    #[test]
    fn les_sigles_survivent_a_la_casse() {
        let r = [Rule::TitleCase];
        assert_eq!(apply("DNA", &r), "DNA");
        assert_eq!(apply("live at BBC", &r), "Live at BBC");
    }

    #[test]
    fn les_regles_s_enchainent_dans_l_ordre() {
        // Souligné, puis numéro, puis espaces : chacune prépare la suivante.
        assert_eq!(
            apply("01_-_Ma__Chanson  ", &[Rule::Underscores, Rule::LeadingNumber, Rule::Spaces]),
            "Ma Chanson"
        );
    }

    #[test]
    fn aucune_regle_ne_change_rien() {
        assert_eq!(apply("  01_Titre  ", &[]), "  01_Titre  ");
    }

    #[test]
    fn les_noms_de_regles_font_l_aller_retour() {
        for rule in ALL {
            assert_eq!(Rule::parse(rule.as_str()), Some(rule));
        }
        assert_eq!(Rule::parse("inconnue"), None);
    }
}
