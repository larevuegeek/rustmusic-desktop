//! La liste de colonnes de `TrackListRow` suit ses champs.

use super::track_list_row::*;

/// Les champs de la structure, lus dans son propre code source.
///
/// Se relire soi-même est inhabituel, mais c'est ce qui rend le test
/// automatique : ajouter un champ suffit à le faire échouer si la liste de
/// colonnes ne suit pas. Une liste de noms recopiée dans le test aurait
/// simplement dérivé avec le reste.
fn struct_fields() -> Vec<String> {
    let source = include_str!("track_list_row.rs");
    // Après l'accolade ouvrante : la ligne de déclaration commence elle
    // aussi par `pub`, et serait prise pour un champ nommé « struct ».
    let header = "pub struct TrackListRow {";
    let start = source.find(header).expect("déclaration de la structure") + header.len();
    let corps = &source[start..];
    let end = corps.find("\n}").expect("fin de la structure");

    corps[..end]
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix("pub ")?;
            let name = rest.split(':').next()?.trim();
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

#[test]
fn the_column_list_covers_every_field() {
    let fields = struct_fields();
    assert!(fields.len() > 30, "lecture de la structure ratée : {fields:?}");

    for field in &fields {
        assert!(
            TRACK_VIEW_SELECT.contains(&format!("AS {field}")),
            "le champ « {field} » n'a pas de colonne dans TRACK_VIEW_SELECT"
        );
    }
}

#[test]
fn every_used_alias_is_joined() {
    // Un alias non joint produit une requête invalide, et l'erreur
    // n'apparaît qu'au moment où on l'exécute.
    for alias in ["lt.", "lf.", "lc.", "la.", "a.", "lat."] {
        if TRACK_VIEW_SELECT.contains(alias) {
            let declare = TRACK_VIEW_FROM.contains(&format!(" {}", alias.trim_end_matches('.')))
                || TRACK_VIEW_FROM.contains(&format!("{} ", alias.trim_end_matches('.')));
            assert!(declare, "l'alias « {alias} » n'est joint nulle part");
        }
    }
}
