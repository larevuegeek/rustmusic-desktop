//! Les pays proposés par l'annuaire des radios et leur drapeau.


/// Boutiques proposées.
pub const COUNTRY_CODES: [&str; 26] = [
    "FR", "BE", "CH", "CA", "LU", "US", "GB", "IE", "AU", "NZ",
    "DE", "AT", "ES", "MX", "AR", "IT", "PT", "BR", "NL", "SE",
    "NO", "DK", "FI", "PL", "JP", "IN",
];

/// Le nom du pays dans la langue de l'utilisateur, sinon son code.
pub fn country_name(code: &str) -> String {
    if COUNTRY_CODES.contains(&code) { karbon_ui::intl::t(&format!("countries.{code}")) } else { code.to_string() }
}

/// Noms écrits en entier, l'outil des icônes ne repère que
/// les littéraux.
pub fn flag(code: &str) -> &'static str {
    match code.to_uppercase().as_str() {
        "FR" => "circle-flags:fr",
        "BE" => "circle-flags:be",
        "CH" => "circle-flags:ch",
        "CA" => "circle-flags:ca",
        "LU" => "circle-flags:lu",
        "US" => "circle-flags:us",
        "GB" => "circle-flags:gb",
        "IE" => "circle-flags:ie",
        "AU" => "circle-flags:au",
        "NZ" => "circle-flags:nz",
        "DE" => "circle-flags:de",
        "AT" => "circle-flags:at",
        "ES" => "circle-flags:es",
        "MX" => "circle-flags:mx",
        "AR" => "circle-flags:ar",
        "IT" => "circle-flags:it",
        "PT" => "circle-flags:pt",
        "BR" => "circle-flags:br",
        "NL" => "circle-flags:nl",
        "SE" => "circle-flags:se",
        "NO" => "circle-flags:no",
        "DK" => "circle-flags:dk",
        "FI" => "circle-flags:fi",
        "PL" => "circle-flags:pl",
        "JP" => "circle-flags:jp",
        "IN" => "circle-flags:in",
        _ => "",
    }
}
