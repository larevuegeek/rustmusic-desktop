//! Classement des résultats de la recherche rapide : liés en `?5`, `?6` et `?7` aux
//! requêtes qui cherchent `?1` (`%texte%`), ils font passer le nom exact, puis celui qui
//! commence par le texte, puis celui dont un mot commence par lui, avant le reste.

/// Texte exact, `texte%` et `% texte%`, en minuscules comme `search_term`.
pub fn terms(search_term: &str) -> (String, String, String) {
    let text = search_term.trim_matches('%').to_string();
    let prefix = format!("{text}%");
    let word = format!("% {text}%");
    (text, prefix, word)
}

#[cfg(test)]
mod tests {
    #[test]
    fn terms_are_built_from_the_contains_pattern() {
        assert_eq!(
            super::terms("%nirv%"),
            ("nirv".to_string(), "nirv%".to_string(), "% nirv%".to_string())
        );
    }
}
