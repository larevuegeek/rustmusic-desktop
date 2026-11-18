use unidecode::unidecode;

pub fn normalize_name(name: &str) -> String {
    let mut normalized: String = name.trim().to_lowercase();
    
    // Articles à enlever
    let articles = [
        "the ", "a ", "an ",
        "le ", "la ", "les ", "l'",
        "el ", "los ", "las ",
        "der ", "die ", "das ",
        "il ", "lo ", "i ",
    ];
    
    for article in articles {
        if normalized.starts_with(article) {
            normalized = normalized.strip_prefix(article).unwrap().to_string();
            break;
        }
    }
    
    // Retirer les accents
    let normalized: String = unidecode(&normalized);
    
    // Enlever caractères spéciaux et normaliser espaces
    normalized
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn normalize_sort_name(name: &str) -> String {
    let name = name.trim();
    
    // Articles à déplacer à la fin (multilingues)
    let articles = [
        "The ", "the ",
        "A ", "a ",
        "An ", "an ",
        "Le ", "le ",
        "La ", "la ",
        "Les ", "les ",
        "L'", "l'",
        "El ", "el ",
        "Los ", "los ",
        "Las ", "las ",
        "Der ", "der ",
        "Die ", "die ",
        "Das ", "das ",
        "Il ", "il ",
        "Lo ", "lo ",
        "I ", "i ",
    ];
    
    for article in articles {
        if let Some(stripped) = name.strip_prefix(article) {
            let article_clean = article.trim();
            let article_capitalized = format!(
                "{}{}",
                article_clean.chars().next().unwrap().to_uppercase(),
                &article_clean[1..]
            );
            return format!("{}, {}", stripped, article_capitalized);
        }
    }
    
    // Pas d'article, retourne tel quel
    name.to_string()
}

/// Split un champ artiste brut en artistes individuels.
///
/// Gère les séparateurs courants dans les tags audio :
/// - `\0` (ID3v2.4 multi-value)
/// - `;`  (toujours safe)
/// - ` / ` (avec espaces — préserve "AC/DC")
/// - `feat.` / `ft.` / `featuring` (artiste principal + featured)
/// - ` vs. ` / ` vs `
/// - `,` seulement si pas de `&` dans le string (évite "Crosby, Stills & Nash")
///   et si chaque segment a 2+ mots (évite les faux positifs)
pub fn split_artists(raw: &str) -> Vec<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return vec![];
    }

    // 1. Null byte (ID3v2.4 multi-value standard)
    if raw.contains('\0') {
        return raw.split('\0')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    // 2. Point-virgule
    if raw.contains(';') {
        return raw.split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    // 3. " / " avec espaces (pas AC/DC, pas Guns N'/Roses)
    if raw.contains(" / ") {
        return raw.split(" / ")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    // 4. feat./ft./featuring → principal + featured
    // (?i) = case insensitive, \s+ = whitespace obligatoire autour
    let feat_patterns = [" feat. ", " feat ", " ft. ", " ft ", " featuring "];
    for pat in feat_patterns {
        if let Some(pos) = raw.to_lowercase().find(pat) {
            let before = raw[..pos].trim().to_string();
            let after = raw[pos + pat.len()..].trim().to_string();
            let mut result = vec![];
            if !before.is_empty() { result.push(before); }
            if !after.is_empty() { result.push(after); }
            if !result.is_empty() { return result; }
        }
    }

    // 5. " vs " / " vs. "
    let vs_patterns = [" vs. ", " vs "];
    for pat in vs_patterns {
        if let Some(pos) = raw.to_lowercase().find(pat) {
            let before = raw[..pos].trim().to_string();
            let after = raw[pos + pat.len()..].trim().to_string();
            let mut result = vec![];
            if !before.is_empty() { result.push(before); }
            if !after.is_empty() { result.push(after); }
            if !result.is_empty() { return result; }
        }
    }

    // 6. Virgule — prudent
    //    Ne PAS splitter si le string contient aussi '&' (pattern groupe : "Crosby, Stills & Nash")
    //    Ne splitter que si chaque segment a 2+ mots (évite faux positifs)
    if raw.contains(',') && !raw.contains('&') {
        let parts: Vec<String> = raw.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 2 && parts.iter().all(|p| p.split_whitespace().count() >= 2) {
            return parts;
        }
    }

    // 7. Aucun split
    vec![raw.to_string()]
}

pub fn normalize_year(date: Option<String>) -> Option<String> {
    date.and_then(|d| {
        let year: String = d.chars().take(4).collect();
        if year.len() == 4 && year.chars().all(|c| c.is_ascii_digit()) {
            Some(year)
        } else {
            None
        }
    })
}
/// Le titre, sinon le nom du fichier sans extension, sinon le repli.
pub fn display_title(title: Option<&str>, path: &str, fallback: &str) -> String {
    if let Some(t) = title.filter(|t| !t.trim().is_empty()) {
        return t.to_string();
    }
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
    if stem.trim().is_empty() {
        fallback.to_string()
    } else {
        stem.to_string()
    }
}

/// `hue()` de `helper/tools/hue.ts` : une teinte stable (0–359) pour un texte, qui
/// colore un genre, un mix ou une vignette (`style:--h`).
pub fn hue(text: &str) -> f32 {
    let mut h: u32 = 0;
    for unit in text.encode_utf16() {
        h = h.wrapping_mul(31).wrapping_add(u32::from(unit));
    }
    (h % 360) as f32
}

/// L'initiale d'un nom, en majuscule (vignette sans image) ; vide pour un nom vide.
pub fn initial(text: &str) -> String {
    text.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()
}

#[cfg(test)]
mod display_tests {
    use super::*;

    #[test]
    fn titles_and_hues_follow_the_frontend_helpers() {
        assert_eq!(display_title(None, r"D:\Music\4 Stairway.flac", "?"), "4 Stairway");
        assert_eq!(display_title(Some("  "), "", "?"), "?");
        assert_eq!(hue("pop"), 305.);
        assert_eq!(hue("rock"), 341.);
    }
}