use serde::Serialize;
use tauri::State;

use crate::core::smart_playlist::{engine, field, rules};
use crate::mapper::playlist::playlist::playlist_track_view::PlaylistTrackView;
use crate::repository::playlist::playlist_repository::PlaylistRepository;
use crate::state::AppState;

/// Un opérateur, tel que l'interface le propose.
#[derive(Debug, Serialize)]
pub struct OpInfo {
    pub key: &'static str,
    pub label: &'static str,
    /// Nombre de valeurs à saisir : 0 pour « est vide », 2 pour « entre ».
    pub arity: u8,
}

/// Le vocabulaire complet des règles, servi à l'interface.
///
/// Le décrire ici plutôt que de le recopier côté interface évite qu'un champ
/// ajouté en Rust reste invisible, ou qu'un opérateur proposé n'existe pas.
#[derive(Debug, Serialize)]
pub struct RuleVocabulary {
    pub fields: Vec<field::FieldInfo>,
    /// Opérateurs par nature de champ : `text`, `number`, `date`, `bool`.
    pub operators: std::collections::BTreeMap<String, Vec<OpInfo>>,
}

fn arity(op: rules::Op) -> u8 {
    use rules::Op::*;
    match op {
        IsEmpty | IsNotEmpty | IsTrue | IsFalse => 0,
        Between => 2,
        _ => 1,
    }
}

fn op_key(op: rules::Op) -> &'static str {
    use rules::Op::*;
    match op {
        Contains => "contains",
        NotContains => "not_contains",
        Is => "is",
        IsNot => "is_not",
        StartsWith => "starts_with",
        EndsWith => "ends_with",
        IsEmpty => "is_empty",
        IsNotEmpty => "is_not_empty",
        Eq => "eq",
        Neq => "neq",
        Gt => "gt",
        Gte => "gte",
        Lt => "lt",
        Lte => "lte",
        Between => "between",
        Before => "before",
        After => "after",
        InLast => "in_last",
        NotInLast => "not_in_last",
        IsTrue => "is_true",
        IsFalse => "is_false",
        InTopPlayed => "in_top_played",
    }
}

#[tauri::command]
pub fn get_rule_vocabulary() -> RuleVocabulary {
    use field::FieldKind::*;

    let mut operators = std::collections::BTreeMap::new();
    for (nom, kind) in [("text", Text), ("number", Number), ("date", Date), ("bool", Bool)] {
        operators.insert(
            nom.to_string(),
            rules::Op::for_kind(kind)
                .iter()
                .map(|op| OpInfo {
                    key: op_key(*op),
                    label: op.label(),
                    arity: arity(*op),
                })
                .collect(),
        );
    }

    RuleVocabulary {
        fields: field::known_fields(),
        operators,
    }
}

/// Compte les morceaux que des règles retiendraient, sans rien enregistrer.
///
/// Appelée pendant qu'on compose : voir le nombre bouger est ce qui permet de
/// comprendre une règle, bien plus qu'un intitulé.
#[tauri::command]
pub async fn count_smart_playlist(
    state: State<'_, AppState>,
    rules: rules::SmartRules,
    source_id: Option<i64>,
) -> Result<i64, String> {
    engine::count(&state.pool, &rules, source_id).await
}

/// Rend un échantillon de ce que les règles donneraient.
#[tauri::command]
pub async fn preview_smart_playlist(
    state: State<'_, AppState>,
    rules: rules::SmartRules,
    source_id: Option<i64>,
) -> Result<Vec<PlaylistTrackView>, String> {
    engine::evaluate(&state.pool, 0, &rules, source_id).await
}

/// Crée une playlist intelligente.
#[tauri::command]
pub async fn create_smart_playlist(
    state: State<'_, AppState>,
    profil_id: i64,
    name: String,
    color: Option<String>,
    icon: Option<String>,
    rules: rules::SmartRules,
    is_mix: Option<bool>,
    source_id: Option<i64>,
) -> Result<i64, String> {
    // Les règles sont validées avant d'être enregistrées : une playlist qu'on
    // ne saurait pas évaluer ne doit pas exister.
    rules::build_where(&rules)?;
    rules::build_order(&rules.limit)?;

    let json = serde_json::to_string(&rules).map_err(|e| format!("Règles : {e}"))?;

    let id = PlaylistRepository::insert_smart(
        &state.pool,
        profil_id,
        &name,
        color.as_deref().unwrap_or("#8b5cf6"),
        icon.as_deref().unwrap_or("lucide:sparkles"),
        &json,
        is_mix.unwrap_or(false),
        source_id.unwrap_or(0),
    )
    .await
    .map_err(|e| format!("Création : {e}"))?;

    refresh_count(&state.pool, id, &rules).await;

    Ok(id)
}

/// Réécrit les règles d'une playlist intelligente.
#[tauri::command]
pub async fn update_smart_playlist(
    state: State<'_, AppState>,
    playlist_id: i64,
    name: Option<String>,
    color: Option<String>,
    icon: Option<String>,
    rules: rules::SmartRules,
) -> Result<(), String> {
    rules::build_where(&rules)?;
    rules::build_order(&rules.limit)?;

    let json = serde_json::to_string(&rules).map_err(|e| format!("Règles : {e}"))?;

    PlaylistRepository::update_smart(&state.pool, playlist_id, name.as_deref(), color.as_deref(), icon.as_deref(), &json)
    .await
    .map_err(|e| format!("Mise à jour : {e}"))?;

    refresh_count(&state.pool, playlist_id, &rules).await;

    Ok(())
}

/// Lit les règles enregistrées d'une playlist.
#[tauri::command]
pub async fn get_smart_playlist_rules(
    state: State<'_, AppState>,
    playlist_id: i64,
) -> Result<Option<rules::SmartRules>, String> {
    let json = PlaylistRepository::find_rules(&state.pool, playlist_id)
        .await
        .map_err(|e| format!("Lecture : {e}"))?;

    match json {
        Some(j) => serde_json::from_str(&j)
            .map(Some)
            .map_err(|e| format!("Règles illisibles : {e}")),
        None => Ok(None),
    }
}

/// Met à jour le compteur affiché dans la barre latérale.
///
/// Un échec n'interrompt rien : le compteur est un confort, et le perdre ne
/// justifie pas de refuser la création d'une playlist par ailleurs valide.
async fn refresh_count(pool: &sqlx::SqlitePool, id: i64, regles: &rules::SmartRules) {
    let source = PlaylistRepository::source_of(pool, id).await.unwrap_or(0);
    let Ok(n) = engine::count(pool, regles, Some(source)).await else {
        return;
    };
    let _ = PlaylistRepository::set_track_count(pool, id, n).await;
}
