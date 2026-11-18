//! Traductions de l'interface : les fichiers JSON de `ui/translations/`, embarqués et
//! installés comme catalogue de `karbon_ui::intl`, qui les lit (`t`, `t_with`, `t_count`).

use std::sync::Once;

use karbon_ui::intl::{self, Catalog, Locale};

/// Les langues de l'interface ; la première sert de repli à une clé manquante.
const SOURCES: [(&str, &str); 5] = [
    ("fr", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/translations/fr.json"))),
    ("en", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/translations/en.json"))),
    ("es", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/translations/es.json"))),
    ("de", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/translations/de.json"))),
    ("it", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/translations/it.json"))),
];

/// Choisit la langue (`language` des réglages ; inconnue, le français) des traductions
/// et des formats de `karbon_ui::intl` ; installe les traductions au premier appel.
pub fn set_language(code: &str) {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let mut catalog = Catalog::new();
        for (tag, text) in SOURCES {
            if let Err(e) = catalog.add(Locale::parse(tag), text) {
                log::error!("Traductions {tag} illisibles : {e}");
            }
        }
        intl::set_catalog(catalog);
    });
    let code = SOURCES.iter().find(|(c, _)| *c == code).map_or("fr", |(c, _)| c);
    intl::set_locale(Locale::parse(code));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_resolve_in_french_and_fall_back_to_the_key() {
        set_language("fr");
        assert_eq!(intl::t("nav.home"), "Accueil");
        assert_eq!(intl::t("cle.inconnue"), "cle.inconnue");
    }
}
