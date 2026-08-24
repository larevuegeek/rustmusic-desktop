//! Les règles d'une playlist intelligente, et leur traduction en requête.
//!
//! # Pourquoi filtrer en base plutôt qu'en mémoire
//! Une règle porte sur toute la bibliothèque — quatorze mille morceaux ici.
//! Les charger pour les trier en mémoire coûterait des dizaines de mégaoctets
//! à chaque affichage, et interdirait de composer avec les tags, qui sont du
//! JSON qu'on ne veut pas analyser quatorze mille fois.
//!
//! # Toute valeur est liée, jamais recopiée
//! Le texte d'une règle vient de l'utilisateur, et les noms de tags viennent
//! des fichiers. Rien de tout cela n'entre dans le texte de la requête : la
//! requête ne contient que des `?`, et les valeurs partent à côté.

use serde::{Deserialize, Serialize};

use super::field::{self, FieldKind};

/// Profondeur maximale des groupes imbriqués.
///
/// Un garde-fou, pas une limite de conception : trois niveaux couvrent tout ce
/// qui reste lisible dans une interface, et empêchent qu'un fichier de règles
/// forgé ne fasse récurser sans fin.
const PROFONDEUR_MAX: usize = 3;

/// Nombre de règles au-delà duquel on refuse.
///
/// Là encore un garde-fou : une requête à mille conditions ne vient pas d'un
/// utilisateur qui compose une playlist.
const REGLES_MAX: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Match {
    /// Toutes les conditions doivent être vraies.
    All,
    /// Au moins une condition doit l'être.
    Any,
}

/// Une condition, ou un groupe de conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Node {
    Group(Group),
    Rule(Rule),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    #[serde(rename = "match")]
    pub match_mode: Match,
    pub rules: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub field: String,
    pub op: Op,
    /// Absente pour les opérateurs qui n'en veulent pas — « est vide », par
    /// exemple. `Value::Pair` sert l'intervalle.
    #[serde(default)]
    pub value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Text(String),
    Number(f64),
    Pair(f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    // Texte
    Contains,
    NotContains,
    Is,
    IsNot,
    StartsWith,
    EndsWith,
    IsEmpty,
    IsNotEmpty,
    // Nombre
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Between,
    // Date
    Before,
    After,
    /// « au cours des N derniers jours »
    InLast,
    /// « pas au cours des N derniers jours » — inclut ce qui n'a jamais servi.
    NotInLast,
    // Booléen
    IsTrue,
    IsFalse,
    /// « fait partie des N plus écoutés », sur la valeur du champ.
    ///
    /// Le seul opérateur qui ne compare pas une piste à une valeur, mais la
    /// situe dans un classement calculé sur toute la bibliothèque. C'est ce qui
    /// permet « les styles les plus écoutés » : on ne peut pas les nommer à
    /// l'avance, ils changent à mesure qu'on écoute.
    InTopPlayed,
}

impl Op {
    /// Opérateurs qui ont un sens pour une nature de champ.
    pub fn for_kind(kind: FieldKind) -> &'static [Op] {
        use Op::*;
        match kind {
            FieldKind::Text => &[
                Contains, NotContains, Is, IsNot, StartsWith, EndsWith, IsEmpty, IsNotEmpty,
                InTopPlayed,
            ],
            FieldKind::Number => &[Eq, Neq, Gt, Gte, Lt, Lte, Between, IsEmpty, IsNotEmpty],
            FieldKind::Date => &[Before, After, InLast, NotInLast, IsEmpty, IsNotEmpty],
            FieldKind::Bool => &[IsTrue, IsFalse],
        }
    }

    pub fn label(self) -> &'static str {
        use Op::*;
        match self {
            Contains => "contient",
            NotContains => "ne contient pas",
            Is => "est",
            IsNot => "n'est pas",
            StartsWith => "commence par",
            EndsWith => "finit par",
            IsEmpty => "est vide",
            IsNotEmpty => "n'est pas vide",
            Eq => "égal à",
            Neq => "différent de",
            Gt => "supérieur à",
            Gte => "au moins",
            Lt => "inférieur à",
            Lte => "au plus",
            Between => "entre",
            Before => "avant le",
            After => "après le",
            InLast => "dans les N derniers jours",
            NotInLast => "pas depuis N jours",
            IsTrue => "est vrai",
            IsFalse => "est faux",
            InTopPlayed => "parmi les N plus écoutés",
        }
    }
}

/// Le tri et la coupe d'une playlist intelligente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limit {
    pub count: Option<i64>,
    /// Clé de champ, ou `random`.
    pub sort: Option<String>,
    #[serde(default)]
    pub desc: bool,
}

/// La définition complète, telle qu'elle est enregistrée.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartRules {
    #[serde(flatten)]
    pub root: Group,
    #[serde(default)]
    pub limit: Option<Limit>,
}

/// Une requête prête à exécuter : le texte, et les valeurs à lier dans l'ordre.
#[derive(Debug)]
pub struct BuiltQuery {
    pub sql: String,
    pub binds: Vec<Bind>,
}

/// Une valeur liée. Le type compte : SQLite compare un texte et un nombre
/// différemment, et lier « 4 » là où on attend 4 fausserait la comparaison.
#[derive(Debug, Clone, PartialEq)]
pub enum Bind {
    Text(String),
    Number(f64),
}

/// Construit la clause `WHERE` correspondant aux règles.
///
/// Rend une erreur explicite plutôt qu'un filtre approximatif : une règle qu'on
/// ne sait pas traduire doit se voir, pas se taire.
pub fn build_where(rules: &SmartRules) -> Result<BuiltQuery, String> {
    let total = compter(&rules.root);
    if total > REGLES_MAX {
        return Err(format!("Trop de règles ({total}, maximum {REGLES_MAX})."));
    }

    let mut binds = Vec::new();
    let sql = groupe_sql(&rules.root, &mut binds, 0)?;
    Ok(BuiltQuery { sql, binds })
}

fn compter(groupe: &Group) -> usize {
    groupe
        .rules
        .iter()
        .map(|n| match n {
            Node::Rule(_) => 1,
            Node::Group(g) => compter(g),
        })
        .sum()
}

fn groupe_sql(groupe: &Group, binds: &mut Vec<Bind>, profondeur: usize) -> Result<String, String> {
    if profondeur > PROFONDEUR_MAX {
        return Err(format!("Groupes imbriqués trop profondément (maximum {PROFONDEUR_MAX})."));
    }

    // Un groupe vide ne filtre rien. `1=1` plutôt qu'une chaîne vide : la
    // requête reste valide sans que l'appelant ait à traiter ce cas à part.
    if groupe.rules.is_empty() {
        return Ok("1=1".to_string());
    }

    let liant = match groupe.match_mode {
        Match::All => " AND ",
        Match::Any => " OR ",
    };

    let morceaux: Result<Vec<String>, String> = groupe
        .rules
        .iter()
        .map(|n| match n {
            Node::Rule(r) => regle_sql(r, binds),
            Node::Group(g) => groupe_sql(g, binds, profondeur + 1),
        })
        .collect();

    Ok(format!("({})", morceaux?.join(liant)))
}

fn texte(valeur: &Option<Value>, op: Op) -> Result<String, String> {
    match valeur {
        Some(Value::Text(s)) => Ok(s.clone()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        _ => Err(format!("L'opérateur « {} » attend une valeur.", op.label())),
    }
}

fn nombre(valeur: &Option<Value>, op: Op) -> Result<f64, String> {
    match valeur {
        Some(Value::Number(n)) => Ok(*n),
        // Une saisie numérique arrive parfois en texte depuis l'interface.
        // L'accepter évite de renvoyer une erreur pour un « 4 » parfaitement
        // clair.
        Some(Value::Text(s)) => s
            .trim()
            .replace(',', ".")
            .parse::<f64>()
            .map_err(|_| format!("« {s} » n'est pas un nombre.")),
        _ => Err(format!("L'opérateur « {} » attend un nombre.", op.label())),
    }
}

fn regle_sql(regle: &Rule, binds: &mut Vec<Bind>) -> Result<String, String> {
    let champ = field::resolve(&regle.field)
        .ok_or_else(|| format!("Champ inconnu : « {} ».", regle.field))?;

    // Le tag éventuel se lie AVANT la valeur : c'est l'ordre des `?` dans
    // l'expression finale, et l'inverser ferait chercher le tag nommé « 4 ».
    let nom_tag = champ.bind.clone();
    if let Some(nom) = champ.bind {
        binds.push(Bind::Text(nom));
    }

    let e = &champ.sql;
    use Op::*;

    let sql = match regle.op {
        // ── Texte ──
        // `LIKE` est insensible à la casse en SQLite pour l'ASCII ; le `LOWER`
        // des deux côtés étend ça aux accents des tags français.
        Contains | NotContains | StartsWith | EndsWith | Is | IsNot => {
            let v = texte(&regle.value, regle.op)?;
            let motif = match regle.op {
                Contains | NotContains => format!("%{}%", echapper(&v)),
                StartsWith => format!("{}%", echapper(&v)),
                EndsWith => format!("%{}", echapper(&v)),
                _ => echapper(&v),
            };
            binds.push(Bind::Text(motif.to_lowercase()));
            let negation = matches!(regle.op, NotContains | IsNot);
            // Un champ NULL ne « contient » rien, mais il « ne contient pas » —
            // sans le COALESCE, la négation laisserait filer les NULL.
            format!(
                "LOWER(COALESCE({e}, '')) {}LIKE ? ESCAPE '\\'",
                if negation { "NOT " } else { "" }
            )
        }
        IsEmpty => format!("({e} IS NULL OR TRIM(CAST({e} AS TEXT)) = '')"),
        IsNotEmpty => format!("({e} IS NOT NULL AND TRIM(CAST({e} AS TEXT)) <> '')"),

        // ── Nombre ──
        Eq | Neq | Gt | Gte | Lt | Lte => {
            let n = nombre(&regle.value, regle.op)?;
            binds.push(Bind::Number(n));
            let signe = match regle.op {
                Eq => "=",
                Neq => "<>",
                Gt => ">",
                Gte => ">=",
                Lt => "<",
                _ => "<=",
            };
            // `CAST` parce qu'un champ de tag est du texte : sans lui, « année
            // supérieure à 1990 » comparerait des chaînes.
            format!("CAST({e} AS REAL) {signe} ?")
        }
        Between => {
            let (bas, haut) = match &regle.value {
                Some(Value::Pair(a, b)) => (*a, *b),
                _ => return Err("L'opérateur « entre » attend deux valeurs.".to_string()),
            };
            // Bornes remises dans l'ordre : saisir 10 puis 2 est une maladresse,
            // pas une demande de résultat vide.
            let (bas, haut) = if bas <= haut { (bas, haut) } else { (haut, bas) };
            binds.push(Bind::Number(bas));
            binds.push(Bind::Number(haut));
            format!("CAST({e} AS REAL) BETWEEN ? AND ?")
        }

        // ── Date ──
        Before | After => {
            let v = texte(&regle.value, regle.op)?;
            binds.push(Bind::Text(v));
            let signe = if regle.op == Before { "<" } else { ">" };
            format!("({e} IS NOT NULL AND {e} {signe} ?)")
        }
        InLast => {
            let jours = nombre(&regle.value, regle.op)?;
            binds.push(Bind::Number(jours));
            format!("({e} IS NOT NULL AND {e} >= datetime('now', '-' || ? || ' days'))")
        }
        NotInLast => {
            let jours = nombre(&regle.value, regle.op)?;
            binds.push(Bind::Number(jours));
            // Le NULL est inclus volontairement : « pas écouté depuis trente
            // jours » doit ramener ce qui ne l'a jamais été. C'est la lecture
            // qu'on attend, et l'autre ne sert à rien.
            format!("({e} IS NULL OR {e} < datetime('now', '-' || ? || ' days'))")
        }

        // ── Booléen ──
        IsTrue => format!("COALESCE({e}, 0) = 1"),
        IsFalse => format!("COALESCE({e}, 0) = 0"),

        // ── Classement ──
        //
        // « Les styles les plus écoutés » ne se nomment pas à l'avance : ils
        // changent à mesure qu'on écoute. La condition ne compare donc pas une
        // valeur, elle interroge un classement recalculé à chaque évaluation.
        //
        // On somme les écoutes **par valeur du champ** — toutes les pistes d'un
        // genre, pas seulement la plus jouée — puis on garde les N premières.
        // C'est ce qui distingue « les genres les plus écoutés » de « les genres
        // qui contiennent le morceau le plus écouté ».
        InTopPlayed => {
            let n = nombre(&regle.value, regle.op)?;
            if n < 1.0 {
                return Err("Le classement demande au moins un rang.".to_string());
            }

            // L'expression du champ est nommée `v` dans la sous-requête, et
            // c'est cet alias qu'on filtre, groupe et trie. Sans lui, elle
            // serait recopiée cinq fois — et un champ de tag porte un `?`, donc
            // il faudrait cinq liaisons dans le bon ordre. Une seule répétition
            // demande une seule liaison supplémentaire, celle de l'intérieur.
            if let Some(nom) = nom_tag {
                binds.push(Bind::Text(nom));
            }
            binds.push(Bind::Number(n));

            format!(
                "{e} IN (\
                   SELECT v FROM (\
                     SELECT {e} AS v, SUM(COALESCE(lt.play_count, 0)) AS ecoutes \
                     {from} \
                     GROUP BY v \
                     HAVING v IS NOT NULL AND TRIM(CAST(v AS TEXT)) <> '' \
                     ORDER BY ecoutes DESC \
                     LIMIT ?\
                   )\
                 )",
                e = e,
                from = field::TRACK_FROM,
            )
        }
    };

    Ok(sql)
}

/// Neutralise les jokers de `LIKE` dans une valeur saisie.
///
/// Sans ça, chercher un titre contenant « 100% » ramènerait tout : `%` est le
/// joker de `LIKE`, et l'utilisateur ne le sait pas.
fn echapper(v: &str) -> String {
    v.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

/// Construit la clause `ORDER BY` et la coupe.
///
/// Rendue à part : elle se place après le `WHERE` dans la requête, et ses
/// valeurs liées doivent suivre les siennes.
pub fn build_order(limit: &Option<Limit>) -> Result<(String, Vec<Bind>), String> {
    let Some(limit) = limit else {
        return Ok((String::new(), Vec::new()));
    };

    let mut binds = Vec::new();
    let mut sortie = String::new();

    if let Some(cle) = limit.sort.as_deref() {
        if cle == "random" {
            // Une playlist « au hasard » se veut différente à chaque ouverture :
            // pas de graine, c'est le propos.
            sortie.push_str(" ORDER BY RANDOM()");
        } else {
            let champ = field::resolve(cle)
                .ok_or_else(|| format!("Champ de tri inconnu : « {cle} »."))?;
            if let Some(nom) = champ.bind {
                binds.push(Bind::Text(nom));
            }
            // Les vides en dernier quel que soit le sens : c'est ce qu'on veut
            // en triant, et l'inverse n'apprend rien.
            sortie.push_str(&format!(
                " ORDER BY ({} IS NULL), {} {}",
                champ.sql,
                champ.sql,
                if limit.desc { "DESC" } else { "ASC" }
            ));
        }
    }

    if let Some(n) = limit.count {
        if n > 0 {
            sortie.push_str(&format!(" LIMIT {n}"));
        }
    }

    Ok((sortie, binds))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regle(field: &str, op: Op, value: Option<Value>) -> Node {
        Node::Rule(Rule {
            field: field.to_string(),
            op,
            value,
        })
    }

    fn racine(mode: Match, rules: Vec<Node>) -> SmartRules {
        SmartRules {
            root: Group {
                match_mode: mode,
                rules,
            },
            limit: None,
        }
    }

    #[test]
    fn une_regle_de_texte_lie_son_motif() {
        let r = racine(
            Match::All,
            vec![regle("artist", Op::Contains, Some(Value::Text("Nirvana".into())))],
        );
        let q = build_where(&r).unwrap();
        assert!(q.sql.contains("LIKE ?"), "sql : {}", q.sql);
        assert_eq!(q.binds, vec![Bind::Text("%nirvana%".into())]);
        // La valeur ne doit jamais apparaître dans le texte de la requête.
        assert!(!q.sql.to_lowercase().contains("nirvana"));
    }

    #[test]
    fn les_jokers_saisis_sont_neutralises() {
        // Chercher « 100% » ne doit pas tout ramener.
        let r = racine(
            Match::All,
            vec![regle("title", Op::Contains, Some(Value::Text("100%".into())))],
        );
        let q = build_where(&r).unwrap();
        assert_eq!(q.binds, vec![Bind::Text("%100\\%%".into())]);
    }

    #[test]
    fn la_negation_ne_laisse_pas_filer_les_vides() {
        // Un champ NULL « ne contient pas » : sans COALESCE, SQLite rendrait
        // NULL, donc faux, et la règle raterait ces lignes.
        let r = racine(
            Match::All,
            vec![regle("genre", Op::NotContains, Some(Value::Text("rap".into())))],
        );
        let q = build_where(&r).unwrap();
        assert!(q.sql.contains("COALESCE"), "sql : {}", q.sql);
        assert!(q.sql.contains("NOT LIKE"), "sql : {}", q.sql);
    }

    #[test]
    fn un_intervalle_remet_ses_bornes_dans_l_ordre() {
        let r = racine(
            Match::All,
            vec![regle("rating", Op::Between, Some(Value::Pair(5.0, 2.0)))],
        );
        let q = build_where(&r).unwrap();
        assert_eq!(q.binds, vec![Bind::Number(2.0), Bind::Number(5.0)]);
    }

    #[test]
    fn le_nombre_saisi_en_texte_est_accepte() {
        let r = racine(
            Match::All,
            vec![regle("rating", Op::Gte, Some(Value::Text("4".into())))],
        );
        let q = build_where(&r).unwrap();
        assert_eq!(q.binds, vec![Bind::Number(4.0)]);
    }

    #[test]
    fn un_nombre_illisible_est_refuse_clairement() {
        let r = racine(
            Match::All,
            vec![regle("rating", Op::Gte, Some(Value::Text("beaucoup".into())))],
        );
        let err = build_where(&r).unwrap_err();
        assert!(err.contains("n'est pas un nombre"), "message : {err}");
    }

    #[test]
    fn pas_ecoute_depuis_inclut_ce_qui_ne_l_a_jamais_ete() {
        let r = racine(
            Match::All,
            vec![regle("last_played_at", Op::NotInLast, Some(Value::Number(30.0)))],
        );
        let q = build_where(&r).unwrap();
        assert!(q.sql.contains("IS NULL"), "sql : {}", q.sql);
    }

    #[test]
    fn le_tag_se_lie_avant_la_valeur() {
        // L'ordre des liaisons suit l'ordre des `?` dans le texte. L'inverser
        // ferait chercher le tag nommé « Sub Pop ».
        let r = racine(
            Match::All,
            vec![regle(
                "tag:custom:Label",
                Op::Is,
                Some(Value::Text("Sub Pop".into())),
            )],
        );
        let q = build_where(&r).unwrap();
        assert_eq!(
            q.binds,
            vec![Bind::Text("Label".into()), Bind::Text("sub pop".into())]
        );
    }

    #[test]
    fn un_champ_inconnu_est_refuse() {
        let r = racine(
            Match::All,
            vec![regle("lt.title; DROP TABLE x", Op::Is, Some(Value::Text("a".into())))],
        );
        assert!(build_where(&r).unwrap_err().contains("Champ inconnu"));
    }

    #[test]
    fn les_groupes_s_imbriquent_avec_leurs_liants() {
        let r = SmartRules {
            root: Group {
                match_mode: Match::All,
                rules: vec![
                    regle("artist", Op::Is, Some(Value::Text("A".into()))),
                    Node::Group(Group {
                        match_mode: Match::Any,
                        rules: vec![
                            regle("genre", Op::Is, Some(Value::Text("Rock".into()))),
                            regle("genre", Op::Is, Some(Value::Text("Grunge".into()))),
                        ],
                    }),
                ],
            },
            limit: None,
        };
        let q = build_where(&r).unwrap();
        assert!(q.sql.contains(" AND "), "sql : {}", q.sql);
        assert!(q.sql.contains(" OR "), "sql : {}", q.sql);
        assert_eq!(q.binds.len(), 3);
    }

    #[test]
    fn un_groupe_vide_ne_filtre_rien() {
        let q = build_where(&racine(Match::All, vec![])).unwrap();
        // Une condition atomique, sans parenthèses : elle ne peut pas être
        // recoupée par la précédence d'un liant parent.
        assert_eq!(q.sql, "1=1");
        assert!(q.binds.is_empty());
    }

    #[test]
    fn l_imbrication_sans_fin_est_refusee() {
        // Un fichier de règles forgé ne doit pas faire récurser indéfiniment.
        let mut noeud = Group {
            match_mode: Match::All,
            rules: vec![regle("title", Op::IsNotEmpty, None)],
        };
        for _ in 0..(PROFONDEUR_MAX + 2) {
            noeud = Group {
                match_mode: Match::All,
                rules: vec![Node::Group(noeud)],
            };
        }
        let r = SmartRules {
            root: noeud,
            limit: None,
        };
        assert!(build_where(&r).unwrap_err().contains("profondément"));
    }

    #[test]
    fn le_tri_au_hasard_est_reconnu() {
        let (sql, binds) = build_order(&Some(Limit {
            count: Some(50),
            sort: Some("random".into()),
            desc: false,
        }))
        .unwrap();
        assert!(sql.contains("RANDOM()"));
        assert!(sql.contains("LIMIT 50"));
        assert!(binds.is_empty());
    }

    #[test]
    fn un_champ_de_tri_inconnu_est_refuse() {
        let err = build_order(&Some(Limit {
            count: None,
            sort: Some("; DROP TABLE x".into()),
            desc: false,
        }))
        .unwrap_err();
        assert!(err.contains("tri inconnu"), "message : {err}");
    }

    #[test]
    fn la_coupe_ignore_un_nombre_absurde() {
        // Une limite négative ne doit pas produire un `LIMIT -3` silencieux.
        let (sql, _) = build_order(&Some(Limit {
            count: Some(-3),
            sort: None,
            desc: false,
        }))
        .unwrap();
        assert!(!sql.contains("LIMIT"), "sql : {sql}");
    }

    #[test]
    fn les_regles_se_relisent_apres_serialisation() {
        // Elles sont enregistrées en JSON dans la base : ce qui s'écrit doit se
        // relire à l'identique, sans quoi une playlist deviendrait illisible.
        let r = SmartRules {
            root: Group {
                match_mode: Match::Any,
                rules: vec![
                    regle("rating", Op::Gte, Some(Value::Number(4.0))),
                    regle("tag:custom:Label", Op::Is, Some(Value::Text("Sub Pop".into()))),
                    regle("last_played_at", Op::NotInLast, Some(Value::Number(30.0))),
                ],
            },
            limit: Some(Limit {
                count: Some(25),
                sort: Some("random".into()),
                desc: false,
            }),
        };
        let json = serde_json::to_string(&r).unwrap();
        let relu: SmartRules = serde_json::from_str(&json).unwrap();
        let a = build_where(&r).unwrap();
        let b = build_where(&relu).unwrap();
        assert_eq!(a.sql, b.sql);
        assert_eq!(a.binds, b.binds);
    }
}

#[cfg(test)]
mod tests_relecture {
    use super::*;

    /// Les règles telles qu'elles sont réellement écrites en base.
    ///
    /// Copiées d'une playlist créée par l'interface, pas construites à la main :
    /// c'est la forme qu'il faut savoir relire pour rouvrir une playlist, et
    /// une structure fabriquée en test ne prouverait rien sur celle-là.
    const ENREGISTRE: &str = r#"{
        "match": "all",
        "rules": [{ "field": "created_at", "op": "in_last", "value": 30.0 }],
        "limit": { "count": null, "sort": "created_at", "desc": true }
    }"#;

    #[test]
    fn des_regles_ecrites_par_l_interface_se_relisent() {
        let r: SmartRules = serde_json::from_str(ENREGISTRE).expect("relecture");
        assert_eq!(r.root.match_mode, Match::All);
        assert_eq!(r.root.rules.len(), 1);

        let Node::Rule(regle) = &r.root.rules[0] else {
            panic!("une condition, pas un groupe");
        };
        assert_eq!(regle.field, "created_at");
        assert_eq!(regle.op, Op::InLast);

        let limite = r.limit.as_ref().expect("coupe");
        assert!(limite.count.is_none());
        assert_eq!(limite.sort.as_deref(), Some("created_at"));
        assert!(limite.desc);

        // Et elles doivent rester traduisibles.
        build_where(&r).expect("filtre");
        build_order(&r.limit).expect("tri");
    }

    #[test]
    fn une_valeur_entiere_sans_decimale_se_relit_aussi() {
        // L'interface envoie parfois `4` et parfois `4.0` selon qu'elle vient
        // d'une saisie ou d'une recette. Les deux doivent passer.
        for brut in [
            r#"{"match":"all","rules":[{"field":"rating","op":"gte","value":4}]}"#,
            r#"{"match":"all","rules":[{"field":"rating","op":"gte","value":4.0}]}"#,
            r#"{"match":"all","rules":[{"field":"rating","op":"gte","value":"4"}]}"#,
        ] {
            let r: SmartRules = serde_json::from_str(brut).unwrap_or_else(|e| {
                panic!("relecture de {brut} : {e}");
            });
            let q = build_where(&r).expect("filtre");
            assert_eq!(q.binds, vec![Bind::Number(4.0)], "pour {brut}");
        }
    }

    #[test]
    fn des_regles_sans_coupe_se_relisent() {
        // `limit` est facultatif : une playlist enregistrée sans coupe ne doit
        // pas devenir illisible.
        let r: SmartRules =
            serde_json::from_str(r#"{"match":"any","rules":[]}"#).expect("relecture");
        assert!(r.limit.is_none());
        assert_eq!(r.root.match_mode, Match::Any);
    }
}

#[cfg(test)]
mod tests_charge_interface {
    use super::*;

    /// Le corps exact que l'interface envoie à `count_smart_playlist`.
    ///
    /// La saisie passe par un champ de texte : la valeur part donc en chaîne,
    /// et `limit` est explicitement nul pour que le comptage ignore la coupe.
    #[test]
    fn la_charge_du_comptage_se_traduit_correctement() {
        let brut = r#"{
            "match": "all",
            "rules": [{ "field": "play_count", "op": "gt", "value": "1" }],
            "limit": null
        }"#;

        let r: SmartRules = serde_json::from_str(brut).expect("relecture");
        let q = build_where(&r).expect("filtre");

        assert!(q.sql.contains("lt.play_count"), "sql : {}", q.sql);
        assert!(q.sql.contains('>'), "sql : {}", q.sql);
        assert_eq!(q.binds, vec![Bind::Number(1.0)]);
    }

    #[test]
    fn la_recette_des_plus_ecoutes_se_traduit_correctement() {
        // Telle que le fichier de recettes la définit : la valeur y est un
        // nombre, pas une chaîne.
        let brut = r#"{
            "match": "all",
            "rules": [{ "field": "play_count", "op": "gt", "value": 0 }],
            "limit": { "count": 100, "sort": "play_count", "desc": true }
        }"#;

        let r: SmartRules = serde_json::from_str(brut).expect("relecture");
        let q = build_where(&r).expect("filtre");
        assert_eq!(q.binds, vec![Bind::Number(0.0)]);

        let (ordre, _) = build_order(&r.limit).expect("tri");
        assert!(ordre.contains("DESC"), "ordre : {ordre}");
        assert!(ordre.contains("LIMIT 100"), "ordre : {ordre}");
    }
}
