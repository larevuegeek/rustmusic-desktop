//! Appariement d'une piste locale avec une piste distante.
//!
//! # Pourquoi c'est le cœur du sujet
//! Récupérer un album depuis une source en ligne est facile. Décider **quelle
//! piste distante correspond à quel fichier** ne l'est pas, et c'est là que
//! tout se joue : une erreur d'appariement écrit le mauvais titre sur le
//! mauvais morceau, en masse, sans que rien ne le signale.
//!
//! On ne peut pas se fier au seul numéro de piste — beaucoup de fichiers n'en
//! ont pas, ou en ont un faux, et c'est souvent précisément ce qu'on est venu
//! corriger. Ni au seul titre : les fautes de frappe, les accents manquants et
//! les mentions « (Remastered) » le rendent instable.
//!
//! D'où un **score** qui combine trois indices indépendants, et trois niveaux
//! de confiance. Rien n'est appliqué automatiquement en dessous du niveau sûr :
//! l'appariement propose, l'utilisateur tranche.
//!
//! # Ce que le module ne fait pas
//! Il ne connaît ni Deezer, ni MusicBrainz, ni aucune source. Il travaille sur
//! deux listes de structures neutres — ce qui le rend testable, et réutilisable
//! le jour où une seconde source arrivera.

use serde::Serialize;

/// Une piste du fichier, telle qu'on la connaît avant correction.
#[derive(Debug, Clone)]
pub struct LocalTrack {
    /// Position dans le jeu de travail — c'est par elle que l'appelant
    /// retrouvera son fichier.
    pub index: usize,
    pub title: String,
    pub track_number: Option<u16>,
    /// En secondes. Absente sur un fichier illisible.
    pub duration: Option<u32>,
}

/// Une piste telle que la source en ligne la décrit.
#[derive(Debug, Clone)]
pub struct RemoteTrack {
    pub position: u16,
    pub title: String,
    /// En secondes.
    pub duration: u32,
}

/// À quel point on peut se fier à un appariement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    /// Applicable sans relecture ligne à ligne.
    Sure,
    /// Plausible, mais à confirmer à l'œil.
    Doubtful,
    /// Écarté : mieux vaut aucun appariement qu'un mauvais.
    Rejected,
}

/// Le résultat pour une piste locale.
#[derive(Debug, Clone, Serialize)]
pub struct TrackMatch {
    pub local_index: usize,
    /// Position de la piste distante retenue, `None` si aucune ne convient.
    pub remote_position: Option<u16>,
    /// Entre 0 et 1.
    pub score: f32,
    pub confidence: Confidence,
}

/// Au-dessus, on considère l'appariement acquis.
const SURE: f32 = 0.85;
/// En dessous, on n'en propose aucun.
const DOUBTFUL: f32 = 0.55;

/// Écart de durée au-delà duquel l'indice ne vaut plus rien, en secondes.
const DURATION_TOLERANCE: f32 = 15.0;

// Poids des trois indices. Le titre domine parce qu'il est le seul à porter
// vraiment l'identité du morceau ; les deux autres départagent.
const WEIGHT_TITLE: f32 = 0.60;
const WEIGHT_DURATION: f32 = 0.25;
const WEIGHT_NUMBER: f32 = 0.15;

/// Apparie chaque piste locale à au plus une piste distante.
///
/// Glouton par score décroissant : on prend d'abord les couples les plus sûrs,
/// et chaque piste distante n'est consommée qu'une fois. C'est ce qui évite
/// qu'un titre générique — « Intro », « Untitled » — capture la piste d'un
/// autre morceau mieux identifié.
pub fn match_tracks(local: &[LocalTrack], remote: &[RemoteTrack]) -> Vec<TrackMatch> {
    // Tous les couples possibles, avec leur score.
    let mut pairs: Vec<(usize, usize, f32)> = Vec::with_capacity(local.len() * remote.len());
    for (li, l) in local.iter().enumerate() {
        for (ri, r) in remote.iter().enumerate() {
            pairs.push((li, ri, score(l, r)));
        }
    }
    pairs.sort_by(|a, b| b.2.total_cmp(&a.2));

    let mut taken_local = vec![false; local.len()];
    let mut taken_remote = vec![false; remote.len()];
    let mut result: Vec<Option<(u16, f32)>> = vec![None; local.len()];

    for (li, ri, score) in pairs {
        if score < DOUBTFUL || taken_local[li] || taken_remote[ri] {
            continue;
        }
        taken_local[li] = true;
        taken_remote[ri] = true;
        result[li] = Some((remote[ri].position, score));
    }

    local
        .iter()
        .enumerate()
        .map(|(li, track)| match result[li] {
            Some((position, score)) => TrackMatch {
                local_index: track.index,
                remote_position: Some(position),
                score,
                confidence: if score >= SURE {
                    Confidence::Sure
                } else {
                    Confidence::Doubtful
                },
            },
            None => TrackMatch {
                local_index: track.index,
                remote_position: None,
                score: 0.0,
                confidence: Confidence::Rejected,
            },
        })
        .collect()
}

fn score(local: &LocalTrack, remote: &RemoteTrack) -> f32 {
    let title = similarity(&normalize(&local.title), &normalize(&remote.title));

    // La durée départage deux titres proches. Une absence n'accuse ni
    // n'innocente : on retombe sur une valeur neutre plutôt que de pénaliser
    // un fichier dont on ne sait rien.
    let duration = match local.duration {
        Some(d) => {
            let gap = (d as f32 - remote.duration as f32).abs();
            (1.0 - gap / DURATION_TOLERANCE).max(0.0)
        }
        None => 0.5,
    };

    let number = match local.track_number {
        Some(n) if n == remote.position => 1.0,
        Some(_) => 0.0,
        None => 0.5,
    };

    WEIGHT_TITLE * title + WEIGHT_DURATION * duration + WEIGHT_NUMBER * number
}

/// Ramène un titre à sa forme comparable.
///
/// Minuscules, accents repliés, ponctuation retirée, espaces réduits. Sans ça
/// « Où sont les femmes ? » et « Ou sont les femmes » seraient deux titres
/// différents — alors que c'est la même chanson, écrite par deux sources.
///
/// La valeur normalisée ne sert **qu'à comparer** : ce qu'on écrit dans le
/// fichier reste la chaîne d'origine de la source.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_was_space = true; // évite un espace en tête

    for ch in text.chars() {
        // Minuscule **Unicode** d'abord : `to_ascii_lowercase` laisserait « É »
        // intact, et le repli d'accent, qui ne connaît que les minuscules, ne
        // le reconnaîtrait pas. Un caractère peut en donner plusieurs (« İ »),
        // d'où la boucle.
        for lower in ch.to_lowercase() {
            let folded = fold_accent(lower);
            if folded.is_alphanumeric() {
                out.push(folded);
                last_was_space = false;
            } else if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        }
    }

    out.trim_end().to_string()
}

/// Replie les lettres accentuées du latin-1 sur leur base.
///
/// Écrit à la main plutôt qu'avec une bibliothèque de normalisation Unicode :
/// une dizaine de lignes couvrent le français, l'espagnol et l'allemand, qui
/// sont les cas réels. Une lettre non couverte passe telle quelle et se
/// compare à elle-même — le pire cas est un score légèrement plus bas.
fn fold_accent(ch: char) -> char {
    match ch {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        other => other,
    }
}

/// Similarité de deux chaînes, entre 0 et 1.
///
/// Distance de Levenshtein rapportée à la longueur de la plus longue : une
/// faute de frappe sur un titre court pèse plus lourd que sur un titre long,
/// ce qui est le comportement voulu.
pub fn similarity(a: &str, b: &str) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let longest = a.chars().count().max(b.chars().count());
    if longest == 0 {
        return 1.0;
    }
    1.0 - levenshtein(a, b) as f32 / longest as f32
}

/// Distance d'édition, en gardant une seule ligne du tableau en mémoire.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();

    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0usize; b.len() + 1];

    for (i, ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            current[j + 1] = (current[j] + 1)
                .min(previous[j + 1] + 1)
                .min(previous[j] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(index: usize, title: &str, number: Option<u16>, duration: Option<u32>) -> LocalTrack {
        LocalTrack {
            index,
            title: title.into(),
            track_number: number,
            duration,
        }
    }

    fn remote(position: u16, title: &str, duration: u32) -> RemoteTrack {
        RemoteTrack {
            position,
            title: title.into(),
            duration,
        }
    }

    #[test]
    fn normalising_ignores_case_accents_and_punctuation() {
        assert_eq!(normalize("Où sont les femmes ?"), "ou sont les femmes");
        assert_eq!(normalize("L'Écho — Part. II"), "l echo part ii");
        assert_eq!(normalize("  Déjà   Vu  "), "deja vu");
    }

    #[test]
    fn similarity_rates_a_typo_higher_than_a_different_title() {
        let typo = similarity(&normalize("Freeway Chase"), &normalize("Freway Chase"));
        let other = similarity(&normalize("Freeway Chase"), &normalize("Riggs Shoulder"));
        assert!(typo > 0.9, "une faute de frappe doit rester très proche : {typo}");
        assert!(other < 0.5, "deux titres différents ne doivent pas se ressembler : {other}");
    }

    #[test]
    fn an_exact_album_matches_every_track_surely() {
        let local = vec![
            local(0, "Main Title", Some(1), Some(180)),
            local(1, "Freeway Chase", Some(2), Some(240)),
        ];
        let remote = vec![remote(1, "Main Title", 180), remote(2, "Freeway Chase", 240)];

        let matches = match_tracks(&local, &remote);
        assert_eq!(matches.len(), 2);
        assert!(matches.iter().all(|m| m.confidence == Confidence::Sure));
        assert_eq!(matches[0].remote_position, Some(1));
        assert_eq!(matches[1].remote_position, Some(2));
    }

    #[test]
    fn accents_and_punctuation_do_not_prevent_a_match() {
        let local = vec![local(0, "Ou sont les femmes", Some(1), Some(200))];
        let remote = vec![remote(1, "Où sont les femmes ?", 200)];

        let matches = match_tracks(&local, &remote);
        assert_eq!(matches[0].confidence, Confidence::Sure);
    }

    #[test]
    fn the_title_wins_over_a_wrong_track_number() {
        // Le cas courant : des numéros faux, précisément ce qu'on vient
        // corriger. Se fier au numéro écrirait le mauvais titre partout.
        let local = vec![
            local(0, "Freeway Chase", Some(1), Some(240)),
            local(1, "Main Title", Some(2), Some(180)),
        ];
        let remote = vec![remote(1, "Main Title", 180), remote(2, "Freeway Chase", 240)];

        let matches = match_tracks(&local, &remote);
        assert_eq!(matches[0].remote_position, Some(2), "le titre doit primer");
        assert_eq!(matches[1].remote_position, Some(1));
    }

    #[test]
    fn a_track_absent_from_the_album_is_rejected() {
        // Mieux vaut aucun appariement qu'un mauvais : la piste reste sans
        // proposition, et l'utilisateur la traite à la main.
        let local = vec![
            local(0, "Main Title", Some(1), Some(180)),
            local(1, "Un morceau caché", Some(2), Some(300)),
        ];
        let remote = vec![remote(1, "Main Title", 180)];

        let matches = match_tracks(&local, &remote);
        assert_eq!(matches[0].confidence, Confidence::Sure);
        assert_eq!(matches[1].remote_position, None);
        assert_eq!(matches[1].confidence, Confidence::Rejected);
    }

    #[test]
    fn a_remote_track_is_never_used_twice() {
        // Deux fichiers au même titre générique ne doivent pas réclamer la
        // même piste distante.
        let local = vec![
            local(0, "Intro", Some(1), Some(60)),
            local(1, "Intro", Some(2), Some(61)),
        ];
        let remote = vec![remote(1, "Intro", 60)];

        let matches = match_tracks(&local, &remote);
        let used: Vec<_> = matches.iter().filter_map(|m| m.remote_position).collect();
        assert_eq!(used.len(), 1, "la même piste distante a servi deux fois");
    }

    #[test]
    fn duration_separates_two_similar_titles() {
        let local = vec![local(0, "Theme", None, Some(300))];
        let remote = vec![remote(1, "Theme", 60), remote(2, "Theme", 300)];

        let matches = match_tracks(&local, &remote);
        assert_eq!(
            matches[0].remote_position,
            Some(2),
            "la durée doit départager deux titres identiques"
        );
    }

    #[test]
    fn a_file_without_number_or_duration_still_matches_on_title() {
        // Un fichier fraîchement rippé sans tags : c'est le cas d'usage.
        let local = vec![local(0, "Freeway Chase", None, None)];
        let remote = vec![remote(7, "Freeway Chase", 240)];

        let matches = match_tracks(&local, &remote);
        assert_eq!(matches[0].remote_position, Some(7));
        assert_ne!(matches[0].confidence, Confidence::Rejected);
    }
}
