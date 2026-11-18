//! Tri des albums : clé de tri (articles écartés), lettre d'index, ordre naturel.

use crate::mapper::tracks::track_list::LETTERS;

/// Sans accents, sans article de tête, en minuscules.
pub fn sort_key(text: &str) -> String {
    let folded: String = text
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'À' | 'Â' | 'Ä' | 'Á' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'î' | 'ï' | 'í' | 'Î' | 'Ï' => 'i',
            'ô' | 'ö' | 'ó' | 'õ' | 'Ô' | 'Ö' => 'o',
            'ù' | 'û' | 'ü' | 'ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_lowercase();
    for article in ["the ", "le ", "la ", "les ", "l'"] {
        if let Some(rest) = folded.strip_prefix(article) {
            return rest.trim_start().to_string();
        }
    }
    folded
}

/// La première lettre de la clé, `#` sinon.
pub fn sort_letter(text: &str) -> &'static str {
    let c = sort_key(text).chars().next().map(|c| c.to_ascii_uppercase());
    match c {
        Some(c @ 'A'..='Z') => LETTERS[(c as u8 - b'A') as usize + 1],
        _ => "#",
    }
}

/// Comparaison naturelle, les chiffres lus comme des nombres : « 2 » avant « 10 ».
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, _) => return std::cmp::Ordering::Less,
            (_, None) => return std::cmp::Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let n: String = std::iter::from_fn(|| x.next_if(|c| c.is_ascii_digit())).collect();
                let m: String = std::iter::from_fn(|| y.next_if(|c| c.is_ascii_digit())).collect();
                let order = n.trim_start_matches('0').len().cmp(&m.trim_start_matches('0').len()).then(n.cmp(&m));
                if order != std::cmp::Ordering::Equal {
                    return order;
                }
            }
            (Some(c), Some(d)) => {
                let order = c.to_lowercase().cmp(d.to_lowercase());
                if order != std::cmp::Ordering::Equal {
                    return order;
                }
                x.next();
                y.next();
            }
        }
    }
}
