//! Source d'un morceau (cet ordinateur ou un serveur) et filtres SQL qui en dépendent.


/// Préfixe des chemins de morceaux distants : `remote://<server>/<album>/<morceau>`.
pub const SCHEME: &str = "remote://";

/// Source d'un morceau : 0 pour « Cet ordinateur », sinon l'id de son serveur.
pub fn source_of(path: &str) -> i64 {
    path.strip_prefix(SCHEME).and_then(|r| r.split('/').next()?.parse().ok()).unwrap_or(0)
}

/// Bibliothèques d'une source (un paramètre), pour `… library_id IN (…)`.
pub const LIBRARIES_OF_SOURCE: &str = "SELECT id FROM library WHERE COALESCE(server_id, 0) = ?";

/// Condition « source active » sur une colonne `source_id` : un serveur désactivé est caché d'un bloc.
pub fn enabled_source(col: &str) -> String {
    format!("({col} = 0 OR {col} IN (SELECT id FROM media_servers WHERE enabled = 1))")
}
