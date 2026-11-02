//! Artistes multiples : « Chris Brown/Nicki Minaj » est-il un duo, et « AC/DC » un groupe ?
//!
//! Un nom douteux (virgule, barre, « & », « x »…) est vérifié une fois : tag `ARTISTS` du
//! fichier, sinon MusicBrainz (le nom entier existe-t-il ? sinon, ses morceaux ?). La décision
//! est gardée par nom ; sans réponse sûre, rien ne change et le nom attend l'avis de l'utilisateur.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::{Arc, RwLock};

use serde::Serialize;
use sqlx::SqlitePool;

use crate::core::audio_analyser::audio_analyser::{AudioAnalyser, ARTISTS_TAG};
use crate::entity::artist::artist::ArtistCreate;
use crate::entity::library::artist_split::{ArtistCheckRun, ArtistRelinkRow, ArtistSplitRule};
use crate::entity::library::library_artist::LibraryArtistCreate;
use crate::entity::library::library_track_artist::LibraryTrackArtistCreate;
use crate::helper::string::string::{normalize_name, normalize_sort_name, split_artists};
use crate::repository::artist::artist_repository::ArtistRepository;
use crate::repository::library::artist_split_repository::{key, ArtistSplitRepository};
use crate::repository::library::library_artist_repository::LibraryArtistRepository;
use crate::repository::library::library_track_artist_repository::LibraryTrackArtistRepository;
use crate::repository::library::library_track_repository::LibraryTrackRepository;

pub const WHOLE: &str = "whole";
pub const SPLIT: &str = "split";
pub const UNKNOWN: &str = "unknown";

/// Séparateurs douteux : ils séparent souvent deux artistes, parfois les mots d'un même nom.
const SEPARATORS: [&str; 8] = [", ", ",", "/", " & ", " + ", " x ", " and ", " et "];
/// Au-delà, trop de découpes possibles : on laisse l'utilisateur trancher.
const MAX_ATOMS: usize = 6;
/// Longueur maximale d'un champ ID3v1 : un nom de cette taille exacte est sans doute coupé.
const TRUNCATED_AT: usize = 30;

// ─── Règles en mémoire, lues par le scan ───

static RULES: RwLock<Option<Arc<HashMap<String, Vec<String>>>>> = RwLock::new(None);

/// Nom → artistes (un seul pour « garder entier »). Les noms « à vérifier » n'y sont pas.
pub fn rules() -> Arc<HashMap<String, Vec<String>>> {
    RULES.read().ok().and_then(|r| r.clone()).unwrap_or_default()
}

pub async fn reload_rules(pool: &SqlitePool) {
    let all = ArtistSplitRepository::find_all(pool).await.unwrap_or_default();
    let map: HashMap<String, Vec<String>> = all
        .iter()
        .filter_map(|r| match r.decision.as_str() {
            WHOLE => Some((key(&r.name), vec![r.name.clone()])),
            SPLIT => Some((key(&r.name), r.parts_list())).filter(|(_, p)| p.len() >= 2),
            _ => None,
        })
        .collect();
    if let Ok(mut w) = RULES.write() {
        *w = Some(Arc::new(map));
    }
}

/// Le tag `ARTISTS` d'un jeu de tags (`custom_tags`).
pub fn artists_tag(custom_tags: &[(String, String)]) -> Vec<String> {
    custom_tags.iter().filter(|(k, _)| k == ARTISTS_TAG).map(|(_, v)| v.clone()).collect()
}

/// Les artistes d'une piste : le tag `ARTISTS` s'il en nomme plusieurs, sinon la découpe
/// d'origine, chaque morceau douteux remplacé par sa règle s'il en a une.
pub fn resolve(raw: &str, tag: &[String], rules: &HashMap<String, Vec<String>>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let pieces: Vec<String> = if tag.len() >= 2 {
        tag.to_vec()
    } else {
        split_artists(raw).into_iter().flat_map(|p| rules.get(&key(&p)).cloned().unwrap_or_else(|| vec![p])).collect()
    };
    for p in pieces {
        if !out.iter().any(|o| key(o) == key(&p)) {
            out.push(p);
        }
    }
    out
}

// ─── Découpe ───

pub fn doubtful(name: &str) -> bool {
    atoms(name).0.len() >= 2
}

/// « Akhenaton, Freeman & Le Rat » → ([Akhenaton, Freeman, Le Rat], [", ", " & "]).
fn atoms(name: &str) -> (Vec<String>, Vec<String>) {
    let lower = name.to_lowercase();
    let (mut atoms, mut seps) = (Vec::new(), Vec::new());
    let mut start = 0;
    let mut i = 0;
    while i < name.len() {
        // Le séparateur le plus long d'abord : « , » ne doit pas couper « , » + espace en deux.
        let hit = SEPARATORS.iter().filter(|s| lower[i..].starts_with(**s)).max_by_key(|s| s.len());
        match hit {
            Some(sep) if i > start => {
                atoms.push(name[start..i].trim().to_string());
                seps.push(name[i..i + sep.len()].to_string());
                i += sep.len();
                start = i;
            }
            _ => i += name[i..].chars().next().map_or(1, char::len_utf8),
        }
    }
    atoms.push(name[start..].trim().to_string());
    // « AC/DC » : deux atomes, à vérifier ; « / » seul ou en bord : pas une séparation.
    if atoms.iter().any(|a| a.is_empty()) {
        return (vec![name.trim().to_string()], vec![]);
    }
    (atoms, seps)
}

fn segment(atoms: &[String], seps: &[String], from: usize, to: usize) -> String {
    let mut s = atoms[from].clone();
    for k in from..to {
        s.push_str(&seps[k]);
        s.push_str(&atoms[k + 1]);
    }
    s.trim().to_string()
}

/// Toutes les découpes en au moins deux morceaux, des moins nombreux aux plus nombreux.
fn compositions(n: usize) -> Vec<Vec<(usize, usize)>> {
    let mut all: Vec<Vec<(usize, usize)>> = (1..(1u32 << (n - 1)))
        .map(|mask| {
            let (mut parts, mut start) = (Vec::new(), 0);
            for k in 0..n - 1 {
                if mask & (1 << k) != 0 {
                    parts.push((start, k));
                    start = k + 1;
                }
            }
            parts.push((start, n - 1));
            parts
        })
        .collect();
    all.sort_by_key(|p| p.len());
    all
}

/// Ce qu'on a conclu pour un nom.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Whole,
    Split(Vec<String>),
    /// Sans réponse sûre : la découpe proposée à l'utilisateur.
    Unknown(Vec<String>),
}

/// Décide pour `name`. `known` dit si un nom existe tel quel (base locale, puis MusicBrainz).
pub async fn decide<F, Fut>(name: &str, tag: &[String], mut known: F) -> Result<(Verdict, &'static str), String>
where
    F: FnMut(String, bool) -> Fut,
    Fut: Future<Output = Result<bool, String>>,
{
    // 1. Le tag ARTISTS du fichier : ceux de ses noms qui figurent dans celui-ci.
    let wanted = key(name);
    let inside: Vec<String> = tag.iter().filter(|t| wanted.contains(&key(t))).cloned().collect();
    if inside.iter().any(|t| key(t) == wanted) {
        return Ok((Verdict::Whole, "tag"));
    }
    if inside.len() >= 2 {
        return Ok((Verdict::Split(inside), "tag"));
    }

    // Tag tronqué à 30 caractères (vieille limite ID3v1) : « …, Aur » ; le dernier morceau n'est pas fiable.
    let (atoms, seps) = atoms(name);
    if name.chars().count() == TRUNCATED_AT {
        return Ok((Verdict::Unknown(atoms), "musicbrainz"));
    }

    // 2. Le nom entier existe : un seul artiste (le nom lui-même est toujours connu localement).
    if known(name.to_string(), false).await? {
        return Ok((Verdict::Whole, "musicbrainz"));
    }

    // 3. Sinon, la découpe la plus courte dont chaque morceau existe.
    if atoms.len() < 2 || atoms.len() > MAX_ATOMS {
        return Ok((Verdict::Unknown(atoms), "musicbrainz"));
    }
    let mut memo: HashMap<String, bool> = HashMap::new();
    for comp in compositions(atoms.len()) {
        let parts: Vec<String> = comp.iter().map(|&(a, b)| segment(&atoms, &seps, a, b)).collect();
        let mut all = true;
        for p in &parts {
            let ok = match memo.get(p) {
                Some(v) => *v,
                None => {
                    let v = known(p.clone(), true).await?;
                    memo.insert(p.clone(), v);
                    v
                }
            };
            if !ok {
                all = false;
                break;
            }
        }
        if all {
            return Ok((Verdict::Split(parts), "musicbrainz"));
        }
    }
    Ok((Verdict::Unknown(atoms), "musicbrainz"))
}

// ─── Vérification ───

/// Les noms douteux de la bibliothèque qui n'ont pas encore de règle (ou tous, avec `force`).
pub async fn candidates(pool: &SqlitePool, force: bool) -> Result<Vec<String>, sqlx::Error> {
    let done: HashSet<String> = if force {
        HashSet::new()
    } else {
        // Les noms restés sans réponse sont repris : MusicBrainz s'enrichit, et la vérification aussi.
        ArtistSplitRepository::find_all(pool).await?.iter().filter(|r| r.decision != UNKNOWN).map(|r| key(&r.name)).collect()
    };
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for raw in ArtistSplitRepository::distinct_local_artists(pool).await? {
        for p in split_artists(&raw) {
            let k = key(&p);
            if doubtful(&p) && !done.contains(&k) && seen.insert(k) {
                out.push(p);
            }
        }
    }
    out.sort_by_key(|n| n.to_lowercase());
    Ok(out)
}

/// Le tag ARTISTS d'un des fichiers qui portent ce nom : déjà en base, sinon relu sur le disque.
async fn tag_for(pool: &SqlitePool, name: &str) -> Vec<String> {
    let rows = ArtistSplitRepository::tracks_mentioning(pool, name).await.unwrap_or_default();
    let Some(row) = rows.first() else { return vec![] };
    let stored = tag_of_row(row);
    if !stored.is_empty() {
        return stored;
    }
    let Some(path) = row.path.clone() else { return vec![] };
    tauri::async_runtime::spawn_blocking(move || {
        AudioAnalyser::analyse_for_import(&std::path::PathBuf::from(path))
            .map(|f| artists_tag(&f.tags.custom_tags))
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

fn tag_of_row(row: &ArtistRelinkRow) -> Vec<String> {
    let custom: Vec<(String, String)> = row
        .tags
        .as_deref()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok())
        .and_then(|v| serde_json::from_value(v["custom_tags"].clone()).ok())
        .unwrap_or_default();
    artists_tag(&custom)
}

/// Un pas de la vérification : le nom en cours, puis (`result`) ce qui a été décidé pour lui.
pub struct Step<'a> {
    pub done: usize,
    pub total: usize,
    pub name: &'a str,
    /// (décision, artistes) une fois le nom tranché.
    pub result: Option<(&'a str, &'a [String])>,
}

/// Vérifie `names` et enregistre une règle pour chacun ; trois échecs réseau d'affilée l'arrêtent.
pub async fn check(pool: &SqlitePool, names: &[String], mut progress: impl FnMut(Step)) -> Result<i64, String> {
    let run = ArtistSplitRepository::start_run(pool).await.map_err(|e| e.to_string())?;
    let local: Arc<HashSet<String>> = Arc::new(ArtistSplitRepository::known_names(pool).await.unwrap_or_default().into_iter().collect());
    // Réponses de MusicBrainz, partagées entre les noms : « Chris Brown » n'est demandé qu'une fois.
    let cache: Arc<std::sync::Mutex<HashMap<String, bool>>> = Arc::default();
    let mut checked = 0i64;
    let mut changed: Vec<String> = Vec::new();
    let mut error: Option<String> = None;
    let mut failures = 0;

    for (i, name) in names.iter().enumerate() {
        progress(Step { done: i, total: names.len(), name, result: None });
        let tag = tag_for(pool, name).await;
        let verdict = decide(name, &tag, |n: String, part: bool| {
            let (local, cache) = (local.clone(), cache.clone());
            async move {
                // Un morceau simple déjà présent ici n'a pas besoin du réseau ; un morceau lui-même
                // composé (« A, A ») peut être un ancien nom combiné : il est vérifié en ligne.
                if part && !doubtful(&n) && local.contains(&normalize_name(&n)) {
                    return Ok(true);
                }
                if let Some(v) = cache.lock().ok().and_then(|c| c.get(&key(&n)).copied()) {
                    return Ok(v);
                }
                let v = crate::core::metadata_source::musicbrainz::artist_exists(&n).await?;
                if let Ok(mut c) = cache.lock() {
                    c.insert(key(&n), v);
                }
                Ok(v)
            }
        })
        .await;
        // Un nom en échec attend la prochaine fois ; trois de suite : le réseau est coupé, on s'arrête.
        let (verdict, source) = match verdict {
            Ok(v) => {
                failures = 0;
                v
            }
            Err(e) => {
                failures += 1;
                log::warn!("🎤 « {name} » non vérifié : {e}");
                error = Some(e);
                if failures >= 3 {
                    break;
                }
                continue;
            }
        };
        let (decision, parts) = match &verdict {
            Verdict::Whole => (WHOLE, vec![]),
            Verdict::Split(p) => (SPLIT, p.clone()),
            Verdict::Unknown(p) => (UNKNOWN, p.clone()),
        };
        ArtistSplitRepository::upsert(pool, name, decision, &parts, source, Some(run)).await.map_err(|e| e.to_string())?;
        checked += 1;
        progress(Step { done: i + 1, total: names.len(), name, result: Some((decision, &parts)) });
        if matches!(verdict, Verdict::Split(_)) {
            changed.push(name.clone());
        }
    }

    ArtistSplitRepository::finish_run(pool, run, checked, error.as_deref()).await.map_err(|e| e.to_string())?;
    reload_rules(pool).await;
    relink(pool, &changed).await?;
    log::info!("🎤 Artistes vérifiés : {checked}/{} ({} séparés)", names.len(), changed.len());
    Ok(run)
}

/// Décision de l'utilisateur (garder entier, séparer, ou annuler une correction) ; les pistes suivent.
pub async fn set_rule(pool: &SqlitePool, name: &str, decision: &str, parts: &[String]) -> Result<(), String> {
    let parts: Vec<String> = parts.iter().map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
    if decision == SPLIT && parts.len() < 2 {
        return Err("Au moins deux artistes pour séparer".into());
    }
    let run = ArtistSplitRepository::find(pool, name).await.map_err(|e| e.to_string())?.and_then(|r| r.run_id);
    ArtistSplitRepository::upsert(pool, name, decision, &parts, "user", run).await.map_err(|e| e.to_string())?;
    reload_rules(pool).await;
    relink(pool, &[name.to_string()]).await
}

// ─── Liens piste ↔ artistes ───

/// Relie de nouveau les pistes qui mentionnent ces noms, selon les règles du moment.
pub async fn relink(pool: &SqlitePool, names: &[String]) -> Result<(), String> {
    if names.is_empty() {
        return Ok(());
    }
    let rules = rules();
    let mut rows: HashMap<String, ArtistRelinkRow> = HashMap::new();
    for n in names {
        for r in ArtistSplitRepository::tracks_mentioning(pool, n).await.map_err(|e| e.to_string())? {
            rows.insert(r.id.clone(), r);
        }
    }
    let mut libraries = HashSet::new();
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    for row in rows.values() {
        let Some(raw) = row.artist.as_deref().filter(|a| !a.trim().is_empty()) else { continue };
        let names = resolve(raw, &tag_of_row(row), &rules);
        ArtistSplitRepository::delete_track_links(&mut *tx, &row.id).await.map_err(|e| e.to_string())?;
        let mut first: Option<String> = None;
        for n in &names {
            let artist = ArtistRepository::insert_artist(&mut *tx, ArtistCreate { name: n.clone(), name_normalized: normalize_name(n), sort_name: normalize_sort_name(n) })
                .await
                .map_err(|e| e.to_string())?;
            LibraryArtistRepository::insert_library_artist(&mut *tx, LibraryArtistCreate { library_id: row.library_id, artist_id: artist.id.clone() })
                .await
                .map_err(|e| e.to_string())?;
            LibraryTrackArtistRepository::insert_library_track_artist(
                &mut *tx,
                LibraryTrackArtistCreate { library_id: row.library_id, artist_id: artist.id.clone(), library_track_id: row.id.clone() },
            )
            .await
            .map_err(|e| e.to_string())?;
            first.get_or_insert(artist.id);
        }
        if let Some(main) = first.filter(|m| row.artist_id.as_deref() != Some(m.as_str())) {
            LibraryTrackRepository::set_artist(&mut *tx, &row.id, &main).await.map_err(|e| e.to_string())?;
        }
        libraries.insert(row.library_id);
    }
    for lib in libraries {
        LibraryArtistRepository::recount_tracks(&mut *tx, lib).await.map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())
}

// ─── Bilan, pour la fenêtre de fin ───

#[derive(Debug, Serialize)]
pub struct RuleView {
    pub name: String,
    pub parts: Vec<String>,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct CheckReport {
    pub run: Option<ArtistCheckRun>,
    /// Séparés : ce qui a changé, annulable.
    pub split: Vec<RuleView>,
    /// Confirmés comme un seul artiste (rien n'a changé).
    pub whole: Vec<RuleView>,
    /// Sans réponse sûre, toutes vérifications confondues.
    pub unknown: Vec<RuleView>,
    /// Restés sans réponse lors de cette vérification.
    pub run_unknown: usize,
    /// Photo connue d'un artiste cité (clé : nom en minuscules).
    pub images: HashMap<String, String>,
}

fn view(r: &ArtistSplitRule) -> RuleView {
    RuleView { name: r.name.clone(), parts: r.parts_list(), source: r.source.clone() }
}

/// Photos connues des artistes, par nom normalisé (comme `artists.name_normalized`).
pub async fn photos(pool: &SqlitePool) -> HashMap<String, String> {
    ArtistSplitRepository::images(pool).await.unwrap_or_default().into_iter().collect()
}

/// Celles de ces noms, par nom en minuscules (la clé que lit l'interface).
pub fn photos_of<'a>(photos: &HashMap<String, String>, names: impl IntoIterator<Item = &'a String>) -> HashMap<String, String> {
    names.into_iter().filter_map(|n| photos.get(&normalize_name(n)).map(|img| (key(n), img.clone()))).collect()
}

pub async fn report(pool: &SqlitePool, run_id: Option<i64>) -> Result<CheckReport, sqlx::Error> {
    let run = ArtistSplitRepository::find_run(pool, run_id).await?;
    let rules = match &run {
        Some(r) => ArtistSplitRepository::find_by_run(pool, r.id).await?,
        None => vec![],
    };
    let unknown = ArtistSplitRepository::find_unknown(pool).await?;
    let split: Vec<RuleView> = rules.iter().filter(|r| r.decision == SPLIT).map(view).collect();
    let whole: Vec<RuleView> = rules.iter().filter(|r| r.decision == WHOLE).map(view).collect();
    let unknown: Vec<RuleView> = unknown.iter().map(view).collect();
    let photos = photos(pool).await;
    let mut images = HashMap::new();
    for v in split.iter().chain(&whole).chain(&unknown) {
        images.extend(photos_of(&photos, std::iter::once(&v.name).chain(&v.parts)));
    }
    Ok(CheckReport { run_unknown: rules.iter().filter(|r| r.decision == UNKNOWN).count(), run, split, whole, unknown, images })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn atoms_keep_names_and_separators() {
        assert_eq!(atoms("Akhenaton, Freeman & Le Rat"), (s(&["Akhenaton", "Freeman", "Le Rat"]), s(&[", ", " & "])));
        assert_eq!(atoms("Chris Brown/Nicki Minaj").0, s(&["Chris Brown", "Nicki Minaj"]));
        assert_eq!(atoms("Jason Derulo X David Guetta").0, s(&["Jason Derulo", "David Guetta"]), "la casse ne compte pas");
        assert_eq!(atoms("Radiohead").0, s(&["Radiohead"]));
        assert_eq!(atoms("Max/").0, s(&["Max/"]), "un séparateur en bord ne coupe pas");
        assert!(doubtful("AC/DC") && !doubtful("Pink Floyd") && !doubtful("Xavier"), "« x » seul, pas dans un mot");
    }

    #[test]
    fn rules_only_touch_the_named_piece() {
        let mut rules = HashMap::new();
        rules.insert(key("Chris Brown/Nicki Minaj"), s(&["Chris Brown", "Nicki Minaj"]));
        rules.insert(key("AC/DC"), s(&["AC/DC"]));
        assert_eq!(resolve("Chris Brown/Nicki Minaj", &[], &rules), s(&["Chris Brown", "Nicki Minaj"]));
        assert_eq!(resolve("Chris Brown/Nicki Minaj; Drake", &[], &rules), s(&["Chris Brown", "Nicki Minaj", "Drake"]));
        assert_eq!(resolve("AC/DC", &[], &rules), s(&["AC/DC"]));
        assert_eq!(resolve("Daft Punk;Pharrell Williams", &[], &HashMap::new()), s(&["Daft Punk", "Pharrell Williams"]), "sans règle : la découpe d'origine");
        assert_eq!(resolve("X/Y", &s(&["Ex", "Why", "Ex"]), &rules), s(&["Ex", "Why"]), "le tag ARTISTS passe avant, sans doublon");
    }

    /// Le réseau simulé : « known » répond selon une liste.
    async fn verdict(name: &str, tag: &[&str], known: &[&str]) -> Verdict {
        let known: HashSet<String> = known.iter().map(|k| key(k)).collect();
        decide(name, &s(tag), |n, _| {
            let v = known.contains(&key(&n));
            async move { Ok(v) }
        })
        .await
        .unwrap()
        .0
    }

    #[tokio::test]
    async fn a_known_whole_name_stays_whole() {
        assert_eq!(verdict("AC/DC", &[], &["AC/DC"]).await, Verdict::Whole);
        assert_eq!(verdict("Earth, Wind & Fire", &[], &["Earth, Wind & Fire", "Earth"]).await, Verdict::Whole);
    }

    #[tokio::test]
    async fn an_unknown_whole_name_is_split_where_every_part_exists() {
        assert_eq!(verdict("Chris Brown/Nicki Minaj", &[], &["Chris Brown", "Nicki Minaj"]).await, Verdict::Split(s(&["Chris Brown", "Nicki Minaj"])));
        // Le moins de morceaux possible : « Lilly Wood & The Prick » reste un groupe.
        assert_eq!(
            verdict("Lilly Wood & The Prick and Robin Schulz", &[], &["Lilly Wood & The Prick", "Robin Schulz", "Lilly Wood", "The Prick"]).await,
            Verdict::Split(s(&["Lilly Wood & The Prick", "Robin Schulz"]))
        );
    }

    #[tokio::test]
    async fn without_a_sure_answer_nothing_is_split() {
        assert_eq!(verdict("Alonzo, Djamel Fezari, Aurélie", &[], &["Alonzo"]).await, Verdict::Unknown(s(&["Alonzo", "Djamel Fezari", "Aurélie"])));
        // 30 caractères pile : coupé par l'ID3v1, même si « Aur » existe quelque part.
        assert!(matches!(verdict("Marwa Loud, Djamel Fezari, Aur", &[], &["Marwa Loud", "Djamel Fezari", "Aur"]).await, Verdict::Unknown(_)));
    }

    #[tokio::test]
    async fn the_artists_tag_answers_without_the_network() {
        assert_eq!(verdict("Chris Brown/Nicki Minaj", &["Chris Brown", "Nicki Minaj", "Drake"], &[]).await, Verdict::Split(s(&["Chris Brown", "Nicki Minaj"])));
        assert_eq!(verdict("AC/DC", &["AC/DC"], &[]).await, Verdict::Whole);
    }

    async fn base() -> SqlitePool {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'Socle')").execute(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO library (id, profil_id, name) VALUES (1, 1, 'test')").execute(&pool).await.unwrap();
        for (id, tag) in [("t1", "Chris Brown/Nicki Minaj"), ("t2", "AC/DC"), ("t3", "Chris Brown")] {
            sqlx::query("INSERT INTO library_cache (path, artist) VALUES (?, ?)").bind(format!("/{id}.flac")).bind(tag).execute(&pool).await.unwrap();
            let cache: i64 = sqlx::query_scalar("SELECT id FROM library_cache WHERE path = ?").bind(format!("/{id}.flac")).fetch_one(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_files (id, library_id, path, filename, extension, size, status) VALUES (?, 1, ?, 'f', 'flac', 1, 'indexed')")
                .bind(format!("f-{id}")).bind(format!("/{id}.flac")).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, cache_id, title, title_normalized) VALUES (?, 1, ?, ?, 'T', 't')")
                .bind(id).bind(format!("f-{id}")).bind(cache).execute(&pool).await.unwrap();
        }
        crate::service::library::artist_link_repair::repair(&pool, Some(1)).await.unwrap();
        pool
    }

    async fn artists_of(pool: &SqlitePool, track: &str) -> Vec<String> {
        sqlx::query_scalar("SELECT a.name FROM library_track_artists l JOIN artists a ON a.id = l.artist_id WHERE l.library_track_id = ? ORDER BY l.rowid")
            .bind(track).fetch_all(pool).await.unwrap()
    }

    async fn listed(pool: &SqlitePool) -> Vec<String> {
        LibraryArtistRepository::find_all_artists_by_library_id(pool, 1).await.unwrap().into_iter().map(|a| a.name).collect()
    }

    #[tokio::test]
    async fn splitting_relinks_tracks_and_undo_puts_them_back() {
        let pool = base().await;
        assert_eq!(candidates(&pool, false).await.unwrap(), s(&["AC/DC", "Chris Brown/Nicki Minaj"]));
        assert_eq!(artists_of(&pool, "t1").await, s(&["Chris Brown/Nicki Minaj"]));

        set_rule(&pool, "Chris Brown/Nicki Minaj", SPLIT, &s(&["Chris Brown", "Nicki Minaj"])).await.unwrap();
        assert_eq!(artists_of(&pool, "t1").await, s(&["Chris Brown", "Nicki Minaj"]));
        let main: String = sqlx::query_scalar("SELECT a.name FROM library_tracks t JOIN artists a ON a.id = t.artist_id WHERE t.id = 't1'").fetch_one(&pool).await.unwrap();
        assert_eq!(main, "Chris Brown");
        let names = listed(&pool).await;
        assert!(!names.contains(&"Chris Brown/Nicki Minaj".to_string()), "l'artiste combiné disparaît de la liste : {names:?}");
        assert!(names.contains(&"Nicki Minaj".to_string()));
        assert_eq!(artists_of(&pool, "t2").await, s(&["AC/DC"]), "les autres pistes ne bougent pas");

        // Annulation : la piste retrouve son artiste d'origine.
        set_rule(&pool, "Chris Brown/Nicki Minaj", WHOLE, &[]).await.unwrap();
        assert_eq!(artists_of(&pool, "t1").await, s(&["Chris Brown/Nicki Minaj"]));
        assert!(!listed(&pool).await.contains(&"Nicki Minaj".to_string()), "plus de piste : plus dans la liste");
        assert!(candidates(&pool, false).await.unwrap() == s(&["AC/DC"]), "une décision prise n'est plus reproposée");
        reload_rules(&pool).await;
    }

    /// Sur une copie de base réelle, avec MusicBrainz :
    /// `RUSTMUSIC_DB_COPY=… cargo test --lib real_library_artists -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn real_library_artists() {
        let path = std::env::var("RUSTMUSIC_DB_COPY").expect("RUSTMUSIC_DB_COPY");
        let pool = SqlitePoolOptions::new().max_connections(1).connect(&format!("sqlite:{path}")).await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        reload_rules(&pool).await;
        // `RUSTMUSIC_NAMES="A|B"` : ces noms seulement, même déjà décidés.
        let names = match std::env::var("RUSTMUSIC_NAMES") {
            Ok(list) => list.split('|').map(str::to_string).collect(),
            Err(_) => candidates(&pool, false).await.unwrap(),
        };
        println!("{} noms douteux", names.len());
        let run = check(&pool, &names, |s| if s.result.is_none() && s.done % 25 == 0 { println!("{}/{} {}", s.done, s.total, s.name) }).await.unwrap();
        let r = report(&pool, Some(run)).await.unwrap();
        println!("\n== SÉPARÉS ({})", r.split.len());
        for v in &r.split { println!("  {}  →  {}  [{}]", v.name, v.parts.join(" · "), v.source); }
        println!("\n== GARDÉS ENTIERS ({})", r.whole.len());
        for v in &r.whole { println!("  {}  [{}]", v.name, v.source); }
        println!("\n== À VÉRIFIER ({})", r.unknown.len());
        for v in &r.unknown { println!("  {}", v.name); }
        println!("\nerreur : {:?}", r.run.and_then(|x| x.error));
    }

    #[tokio::test]
    async fn a_split_needs_two_artists() {
        let pool = base().await;
        assert!(set_rule(&pool, "AC/DC", SPLIT, &s(&["AC"])).await.is_err());
    }
}
