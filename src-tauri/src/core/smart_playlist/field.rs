//! Les champs sur lesquels une règle peut porter, et leur traduction en SQL.
//!
//! # Une seule table, deux usages
//! Trier une colonne et filtrer dessus posent la même question : « quelle
//! expression SQL désigne ce champ ? ». Deux tables séparées auraient fini par
//! diverger — un champ triable mais pas filtrable, ou l'inverse, sans que rien
//! ne le signale. Celle-ci sert les deux.
//!
//! # Le nom d'un tag ne rejoint jamais le texte de la requête
//! Les champs `tag:…` désignent un tag lu dans un fichier. Un morceau peut
//! porter un tag nommé `x'; DROP TABLE library_tracks; --`. L'expression rendue
//! ne contient donc jamais le nom : elle contient un `?`, et le nom part comme
//! valeur liée, que le moteur ne peut plus interpréter comme du code.

/// Nature d'un champ. Elle décide des opérateurs qui ont un sens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    Text,
    Number,
    /// Date stockée en texte ISO. Comparée comme telle, ce qui fonctionne :
    /// l'ordre lexicographique de l'ISO 8601 est l'ordre chronologique.
    Date,
    Bool,
}

/// Une expression SQL, et la valeur à lier si elle en contient une.
pub struct FieldExpr {
    pub sql: String,
    pub bind: Option<String>,
    pub kind: FieldKind,
}

/// Un champ proposable dans l'interface.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FieldInfo {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
}

/// Table close des champs de la bibliothèque.
///
/// Les alias supposent les jointures de `TRACK_FROM` ci-dessous.
const CHAMPS: &[(&str, &str, &str, FieldKind)] = &[
    // clé, intitulé, expression SQL, nature
    ("title", "Titre", "lt.title", FieldKind::Text),
    ("artist", "Artiste", "a.name", FieldKind::Text),
    ("album", "Album", "la.title", FieldKind::Text),
    ("album_artist", "Artiste d'album", "lc.album_artist", FieldKind::Text),
    ("genre", "Genre", "lc.genre", FieldKind::Text),
    ("year", "Année", "lc.year", FieldKind::Text),
    ("audio_format", "Format", "lc.audio_format", FieldKind::Text),
    ("extension", "Extension", "lf.extension", FieldKind::Text),
    ("filename", "Nom de fichier", "lf.filename", FieldKind::Text),
    ("path", "Chemin", "lf.path", FieldKind::Text),
    ("duration", "Durée (s)", "COALESCE(lt.duration, lc.duration)", FieldKind::Number),
    ("rating", "Notation", "lt.rating", FieldKind::Number),
    ("play_count", "Écoutes", "lt.play_count", FieldKind::Number),
    ("track_number", "N° de piste", "lt.track_number", FieldKind::Number),
    ("disc_number", "N° de disque", "lt.disc_number", FieldKind::Number),
    ("bitrate", "Débit", "COALESCE(lt.bitrate, lc.bitrate)", FieldKind::Number),
    ("sample_rate", "Fréquence", "COALESCE(lt.sample_rate, lc.sample_rate)", FieldKind::Number),
    ("bits_per_sample", "Bits", "lc.bits_per_sample", FieldKind::Number),
    ("channels", "Canaux", "lc.channels", FieldKind::Number),
    ("file_size", "Taille", "COALESCE(lc.file_size, lf.size)", FieldKind::Number),
    ("created_at", "Date d'ajout", "lt.created_at", FieldKind::Date),
    ("last_played_at", "Dernière écoute", "lt.last_played_at", FieldKind::Date),
    ("favorite", "Favori", "lt.favorite", FieldKind::Bool),
    ("is_available", "Fichier présent", "lf.is_available", FieldKind::Bool),
];

/// Jointures communes à toutes les requêtes de règles.
///
/// Une seule forme, pour que les alias employés par `CHAMPS` soient toujours
/// définis. Y ajouter un champ n'oblige à toucher à rien d'autre.
pub const TRACK_FROM: &str = "\
    FROM library_tracks lt
    INNER JOIN library_files lf ON lf.id = lt.file_id
    LEFT JOIN library_cache lc ON lc.id = lt.cache_id
    LEFT JOIN library_albums la ON la.id = lt.library_album_id
    LEFT JOIN artists a ON a.id = lt.artist_id";

/// Rend la liste des champs bâtis, pour l'interface.
pub fn known_fields() -> Vec<FieldInfo> {
    CHAMPS
        .iter()
        .map(|(key, label, _, kind)| FieldInfo {
            key,
            label,
            kind: *kind,
        })
        .collect()
}

/// Traduit une clé de champ en expression SQL.
///
/// Rend `None` pour une clé inconnue : c'est ce qui garantit qu'aucune chaîne
/// venue de l'extérieur n'atteint le SQL par ce chemin.
pub fn resolve(field: &str) -> Option<FieldExpr> {
    // Tag libre : la valeur se cherche dans la liste de paires `custom_tags`.
    if let Some(nom) = field.strip_prefix("tag:custom:") {
        return Some(FieldExpr {
            sql: "(SELECT json_extract(je.value, '$[1]') \
                   FROM json_each(json_extract(lt.tags, '$.custom_tags')) je \
                   WHERE json_extract(je.value, '$[0]') = ? LIMIT 1)"
                .to_string(),
            bind: Some(nom.to_string()),
            kind: FieldKind::Text,
        });
    }

    // Champ nommé de la structure de tags.
    if let Some(champ) = field.strip_prefix("tag:") {
        return Some(FieldExpr {
            sql: "json_extract(lt.tags, ?)".to_string(),
            bind: Some(format!("$.{champ}")),
            kind: FieldKind::Text,
        });
    }

    CHAMPS
        .iter()
        .find(|(key, _, _, _)| *key == field)
        .map(|(_, _, sql, kind)| FieldExpr {
            sql: (*sql).to_string(),
            bind: None,
            kind: *kind,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_champ_connu_rend_son_expression() {
        let e = resolve("rating").expect("champ connu");
        assert_eq!(e.sql, "lt.rating");
        assert!(e.bind.is_none());
        assert_eq!(e.kind, FieldKind::Number);
    }

    #[test]
    fn un_champ_inconnu_ne_rend_rien() {
        // C'est la porte fermée : une clé imprévue ne peut pas atteindre le SQL.
        assert!(resolve("").is_none());
        assert!(resolve("lt.title; DROP TABLE library_tracks").is_none());
        assert!(resolve("RANDOM()").is_none());
    }

    #[test]
    fn un_tag_part_en_valeur_liee() {
        let e = resolve("tag:composer").expect("tag nommé");
        assert_eq!(e.sql, "json_extract(lt.tags, ?)");
        assert_eq!(e.bind.as_deref(), Some("$.composer"));

        let e = resolve("tag:custom:Label").expect("tag libre");
        assert!(e.sql.contains("json_each"));
        assert_eq!(e.bind.as_deref(), Some("Label"));
    }

    #[test]
    fn le_nom_d_un_tag_hostile_ne_touche_pas_le_sql() {
        for nom in [
            "x'; DROP TABLE library_tracks; --",
            "a\" OR 1=1 --",
            "'||(SELECT value FROM settings)||'",
        ] {
            for cle in [format!("tag:custom:{nom}"), format!("tag:{nom}")] {
                let e = resolve(&cle).expect("tag");
                assert!(!e.sql.contains(nom), "le nom a fuité : {}", e.sql);
                assert!(e.bind.unwrap().contains(nom));
            }
        }
    }

    #[test]
    fn chaque_champ_annonce_utilise_un_alias_defini() {
        // Un champ dont l'alias n'est pas joint produirait une requête invalide,
        // et l'erreur n'apparaîtrait qu'au moment où une règle l'emploie.
        let alias = ["lt.", "lf.", "lc.", "la.", "a."];
        for (cle, _, sql, _) in CHAMPS {
            assert!(
                alias.iter().any(|p| sql.contains(p)),
                "le champ {cle} n'utilise aucun alias connu : {sql}"
            );
        }
    }

    #[test]
    fn les_cles_sont_uniques() {
        let mut vues = std::collections::HashSet::new();
        for (cle, _, _, _) in CHAMPS {
            assert!(vues.insert(*cle), "clé en double : {cle}");
        }
    }
}
