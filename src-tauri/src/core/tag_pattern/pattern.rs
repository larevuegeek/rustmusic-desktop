//! Un motif, des tags, un chemin.
//!
//! Brique commune au renommage des fichiers et à la restructuration des
//! dossiers. Elle ne touche à rien : elle transforme un jeu de tags en une
//! chaîne, et signale ce qui empêcherait de l'écrire.
//!
//! # Le langage
//! ```text
//! {albumartist|artist}/{album} ({year})/[{disc}-]{track:02} - {title}.{ext}
//! ```
//! - `{champ}` — la valeur du tag
//! - `{track:02}` — complété à gauche par des zéros
//! - `{albumartist|artist}` — le premier champ renseigné
//! - `[…]` — le groupe disparaît **entier** si aucun champ qu'il contient
//!   n'est renseigné, ce qui évite les `1-` orphelins sur un album simple
//!
//! # Ce que le moteur refuse de faire à votre place
//! Il ne remplace pas un tag vide par `Inconnu`. Un dossier `Inconnu/` qui se
//! remplit au fil des imports est une poubelle dont on ne sort plus : le nom
//! ne dit rien, et rien ne signale qu'il faut y revenir. Un champ manquant
//! rend un motif inapplicable, et c'est à l'appelant de le dire.

use std::collections::HashMap;

/// Les champs qu'un motif peut citer.
pub const FIELDS: [&str; 10] = [
    "artist",
    "albumartist",
    "album",
    "title",
    "year",
    "genre",
    "track",
    "disc",
    "composer",
    "ext",
];

/// Un fragment de motif, tel que l'analyse le produit.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Du texte recopié tel quel.
    Literal(String),
    Field {
        /// Les noms candidats, dans l'ordre : le premier renseigné gagne.
        names: Vec<String>,
        /// Largeur de complément par des zéros — `{track:02}` → 2.
        pad: usize,
    },
    /// Un groupe qui disparaît si aucun de ses champs n'est renseigné.
    Optional(Vec<Token>),
}

/// Ce qui empêche un motif d'être compris.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternError {
    UnknownField(String),
    UnclosedField,
    UnclosedGroup,
    UnexpectedGroupEnd,
    EmptyField,
    /// `{track:xx}` — le complément attend un nombre.
    InvalidPad(String),
    /// Un groupe optionnel qui ne contient aucun champ ne disparaîtrait
    /// jamais : c'est presque toujours un crochet oublié.
    GroupWithoutField,
}

impl std::fmt::Display for PatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternError::UnknownField(name) => {
                write!(f, "Champ inconnu : « {name} ». Champs disponibles : {}", FIELDS.join(", "))
            }
            PatternError::UnclosedField => write!(f, "Accolade ouvrante sans fermeture."),
            PatternError::UnclosedGroup => write!(f, "Crochet ouvrant sans fermeture."),
            PatternError::UnexpectedGroupEnd => write!(f, "Crochet fermant sans ouverture."),
            PatternError::EmptyField => write!(f, "Accolades vides : « {{}} »."),
            PatternError::InvalidPad(pad) => {
                write!(f, "Complément invalide : « {pad} ». Attendu un nombre, par exemple « 02 ».")
            }
            PatternError::GroupWithoutField => {
                write!(f, "Groupe optionnel sans champ : il ne disparaîtrait jamais.")
            }
        }
    }
}

/// Analyse un motif.
pub fn parse(pattern: &str) -> Result<Vec<Token>, PatternError> {
    let chars: Vec<char> = pattern.chars().collect();
    let (tokens, end) = parse_until(&chars, 0, None)?;
    if end != chars.len() {
        return Err(PatternError::UnexpectedGroupEnd);
    }
    Ok(tokens)
}

/// Analyse jusqu'au délimiteur attendu — `None` pour la fin de la chaîne.
fn parse_until(
    chars: &[char],
    mut i: usize,
    stop: Option<char>,
) -> Result<(Vec<Token>, usize), PatternError> {
    let mut tokens = Vec::new();
    let mut literal = String::new();

    while i < chars.len() {
        let c = chars[i];

        if Some(c) == stop {
            if !literal.is_empty() {
                tokens.push(Token::Literal(literal));
            }
            return Ok((tokens, i + 1));
        }

        match c {
            '{' => {
                if !literal.is_empty() {
                    tokens.push(Token::Literal(std::mem::take(&mut literal)));
                }
                let close = find(chars, i + 1, '}').ok_or(PatternError::UnclosedField)?;
                let body: String = chars[i + 1..close].iter().collect();
                tokens.push(parse_field(&body)?);
                i = close + 1;
            }
            '[' => {
                if !literal.is_empty() {
                    tokens.push(Token::Literal(std::mem::take(&mut literal)));
                }
                let (inner, next) = parse_until(chars, i + 1, Some(']'))?;
                if next > chars.len() {
                    return Err(PatternError::UnclosedGroup);
                }
                // Un groupe sans champ est du texte qui ne disparaîtrait
                // jamais : le signaler vaut mieux que de l'accepter en
                // silence, parce que c'est un crochet oublié neuf fois sur dix.
                if !inner.iter().any(contains_field) {
                    return Err(PatternError::GroupWithoutField);
                }
                tokens.push(Token::Optional(inner));
                i = next;
            }
            ']' => return Err(PatternError::UnexpectedGroupEnd),
            _ => {
                literal.push(c);
                i += 1;
            }
        }
    }

    if stop.is_some() {
        return Err(PatternError::UnclosedGroup);
    }
    if !literal.is_empty() {
        tokens.push(Token::Literal(literal));
    }
    Ok((tokens, i))
}

fn find(chars: &[char], from: usize, target: char) -> Option<usize> {
    (from..chars.len()).find(|&i| chars[i] == target)
}

fn contains_field(token: &Token) -> bool {
    match token {
        Token::Field { .. } => true,
        Token::Optional(inner) => inner.iter().any(contains_field),
        Token::Literal(_) => false,
    }
}

fn parse_field(body: &str) -> Result<Token, PatternError> {
    if body.trim().is_empty() {
        return Err(PatternError::EmptyField);
    }

    let (names_part, pad) = match body.split_once(':') {
        Some((names, pad)) => {
            let pad: usize = pad
                .trim()
                .parse()
                .map_err(|_| PatternError::InvalidPad(pad.to_string()))?;
            (names, pad)
        }
        None => (body, 0),
    };

    let mut names = Vec::new();
    for name in names_part.split('|') {
        let name = name.trim().to_lowercase();
        if name.is_empty() {
            return Err(PatternError::EmptyField);
        }
        if !FIELDS.contains(&name.as_str()) {
            return Err(PatternError::UnknownField(name));
        }
        names.push(name);
    }

    Ok(Token::Field { names, pad })
}

/// Les champs qu'un motif cite réellement.
///
/// Sert à dire « ce motif a besoin de l'année, que douze morceaux n'ont pas »
/// **avant** de lancer quoi que ce soit.
pub fn referenced_fields(tokens: &[Token]) -> Vec<String> {
    let mut out = Vec::new();
    collect_fields(tokens, &mut out);
    out.sort();
    out.dedup();
    out
}

fn collect_fields(tokens: &[Token], out: &mut Vec<String>) {
    for token in tokens {
        match token {
            Token::Field { names, .. } => out.extend(names.iter().cloned()),
            Token::Optional(inner) => collect_fields(inner, out),
            Token::Literal(_) => {}
        }
    }
}

/// Ce qui manque pour appliquer un motif à un morceau donné.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendered {
    pub text: String,
    /// Champs cités hors groupe optionnel et pourtant vides.
    ///
    /// Le rendu est produit quand même — avec un trou — pour que l'aperçu
    /// montre à quoi ressemblerait le résultat. C'est l'appelant qui refuse.
    pub missing: Vec<String>,
}

/// Applique un motif à un jeu de valeurs.
pub fn render(tokens: &[Token], values: &HashMap<String, String>) -> Rendered {
    let mut text = String::new();
    let mut missing = Vec::new();
    render_into(tokens, values, &mut text, &mut missing, false);
    missing.sort();
    missing.dedup();
    Rendered { text, missing }
}

fn render_into(
    tokens: &[Token],
    values: &HashMap<String, String>,
    out: &mut String,
    missing: &mut Vec<String>,
    optional: bool,
) {
    for token in tokens {
        match token {
            Token::Literal(text) => out.push_str(text),
            Token::Field { names, pad } => {
                match resolve(names, values) {
                    Some(value) => out.push_str(&pad_value(&value, *pad)),
                    None if !optional => missing.push(names.join("|")),
                    None => {}
                }
            }
            Token::Optional(inner) => {
                // Le groupe est rendu à part : s'il ne produit rien, il
                // disparaît sans laisser ses littéraux — c'est tout l'intérêt
                // d'écrire `[{disc}-]` plutôt que `{disc}-`.
                let mut buffer = String::new();
                // Dans un groupe optionnel, un champ vide n'est pas un manque :
                // c'est justement ce qui fait disparaître le groupe.
                let mut ignored = Vec::new();
                let filled = inner.iter().any(|token| match token {
                    Token::Field { names, .. } => resolve(names, values).is_some(),
                    Token::Optional(_) => true,
                    Token::Literal(_) => false,
                });
                if filled {
                    render_into(inner, values, &mut buffer, &mut ignored, true);
                    out.push_str(&buffer);
                }
            }
        }
    }
}

fn resolve(names: &[String], values: &HashMap<String, String>) -> Option<String> {
    for name in names {
        if let Some(value) = values.get(name) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Complète par des zéros à gauche, sans jamais tronquer.
///
/// Un complément n'est qu'une présentation : `{track:02}` sur la piste 143
/// doit rendre `143`, pas `43`. Perdre un chiffre changerait le fichier
/// désigné.
fn pad_value(value: &str, pad: usize) -> String {
    if pad == 0 {
        return value.to_string();
    }
    // Seuls les nombres se complètent : compléter « Live » donnerait « 0Live ».
    match value.parse::<u64>() {
        Ok(number) => format!("{number:0pad$}"),
        Err(_) => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn motif_simple() {
        let tokens = parse("{artist} - {title}").unwrap();
        let out = render(&tokens, &values(&[("artist", "Calogero"), ("title", "Avant toi")]));
        assert_eq!(out.text, "Calogero - Avant toi");
        assert!(out.missing.is_empty());
    }

    #[test]
    fn complement_par_des_zeros() {
        let tokens = parse("{track:02} - {title}").unwrap();
        let out = render(&tokens, &values(&[("track", "7"), ("title", "Flou")]));
        assert_eq!(out.text, "07 - Flou");
    }

    #[test]
    fn le_complement_ne_tronque_jamais() {
        // 143 sur deux chiffres reste 143 : perdre le 1 désignerait un autre
        // fichier.
        let tokens = parse("{track:02}").unwrap();
        assert_eq!(render(&tokens, &values(&[("track", "143")])).text, "143");
    }

    #[test]
    fn on_ne_complete_pas_ce_qui_n_est_pas_un_nombre() {
        let tokens = parse("{title:03}").unwrap();
        assert_eq!(render(&tokens, &values(&[("title", "Live")])).text, "Live");
    }

    #[test]
    fn repli_sur_le_premier_champ_renseigne() {
        let tokens = parse("{albumartist|artist}").unwrap();
        assert_eq!(
            render(&tokens, &values(&[("artist", "Angèle")])).text,
            "Angèle"
        );
        assert_eq!(
            render(&tokens, &values(&[("albumartist", "Divers"), ("artist", "Angèle")])).text,
            "Divers"
        );
    }

    #[test]
    fn un_champ_vide_compte_comme_absent() {
        let tokens = parse("{albumartist|artist}").unwrap();
        let out = render(&tokens, &values(&[("albumartist", "   "), ("artist", "Angèle")]));
        assert_eq!(out.text, "Angèle");
    }

    #[test]
    fn groupe_optionnel_present() {
        let tokens = parse("[{disc}-]{track:02}").unwrap();
        let out = render(&tokens, &values(&[("disc", "2"), ("track", "7")]));
        assert_eq!(out.text, "2-07");
    }

    #[test]
    fn groupe_optionnel_absent_emporte_son_texte() {
        // C'est tout l'intérêt des crochets : pas de « -07 » orphelin.
        let tokens = parse("[{disc}-]{track:02}").unwrap();
        let out = render(&tokens, &values(&[("track", "7")]));
        assert_eq!(out.text, "07");
        assert!(out.missing.is_empty());
    }

    #[test]
    fn un_champ_manquant_hors_groupe_est_signale() {
        let tokens = parse("{album} ({year})").unwrap();
        let out = render(&tokens, &values(&[("album", "Brol")]));
        assert_eq!(out.missing, vec!["year"]);
        // Le rendu existe quand même : l'aperçu doit montrer le trou.
        assert_eq!(out.text, "Brol ()");
    }

    #[test]
    fn motif_complet_d_arborescence() {
        let tokens =
            parse("{albumartist|artist}/{album} ({year})/[{disc}-]{track:02} - {title}.{ext}")
                .unwrap();
        let out = render(
            &tokens,
            &values(&[
                ("artist", "Angèle"),
                ("album", "Brol"),
                ("year", "2018"),
                ("track", "3"),
                ("title", "La thune"),
                ("ext", "flac"),
            ]),
        );
        assert_eq!(out.text, "Angèle/Brol (2018)/03 - La thune.flac");
    }

    #[test]
    fn champ_inconnu() {
        assert_eq!(
            parse("{artiste}"),
            Err(PatternError::UnknownField("artiste".into()))
        );
    }

    #[test]
    fn accolade_non_fermee() {
        assert_eq!(parse("{artist"), Err(PatternError::UnclosedField));
    }

    #[test]
    fn crochet_non_ferme() {
        assert_eq!(parse("[{disc}-"), Err(PatternError::UnclosedGroup));
    }

    #[test]
    fn crochet_fermant_orphelin() {
        assert_eq!(parse("{disc}]"), Err(PatternError::UnexpectedGroupEnd));
    }

    #[test]
    fn groupe_sans_champ() {
        assert_eq!(parse("[texte]"), Err(PatternError::GroupWithoutField));
    }

    #[test]
    fn complement_invalide() {
        assert_eq!(
            parse("{track:xx}"),
            Err(PatternError::InvalidPad("xx".into()))
        );
    }

    #[test]
    fn champs_cites() {
        let tokens = parse("{albumartist|artist}/{album}/[{disc}-]{track:02}").unwrap();
        assert_eq!(
            referenced_fields(&tokens),
            vec!["album", "albumartist", "artist", "disc", "track"]
        );
    }
}
