//! Évaluation d'une playlist intelligente contre la bibliothèque.

use sqlx::{Sqlite, SqlitePool};

use super::field::TRACK_FROM;
use super::rules::{build_order, build_where, Bind, SmartRules};
use crate::mapper::playlist::playlist::playlist_track_view::PlaylistTrackView;

/// Colonnes rendues, dans la forme qu'attend `PlaylistTrackView`.
///
/// Les trois premières n'existent pas ici : le contenu d'une playlist
/// intelligente n'est pas rangé dans `playlist_items`, il est calculé.
///
/// `playlist_item_id` vaut zéro — il n'y a pas de ligne à retirer, et
/// l'interface le lit ainsi. `sort_index` vaut zéro aussi, et c'est un choix :
/// une playlist intelligente n'a **pas** de position enregistrée. L'ordre est
/// celui que la requête rend, et le rendre par une numérotation donnerait
/// l'illusion d'un rang qu'on pourrait modifier. Un `ROW_NUMBER() OVER ()`
/// aurait de surcroît numéroté dans l'ordre du balayage, pas dans l'ordre
/// trié — un numéro faux valant moins que pas de numéro.
const SELECT: &str = "\
    SELECT
        0                    AS playlist_item_id,
        ?                    AS playlist_id,
        0                    AS sort_index,
        lt.id                AS library_track_id,
        lt.title             AS title,
        COALESCE(lt.duration, lc.duration) AS duration,
        lt.play_count        AS play_count,
        lt.track_number      AS track_number,
        lt.disc_number       AS disc_number,
        la.id                AS album_id,
        la.title             AS album_title,
        a.id                 AS artist_id,
        a.name               AS artist_name,
        lf.path              AS path,
        lc.thumbnail_path    AS thumbnail_path";

/// Nombre de morceaux au-delà duquel on cesse d'en rendre.
///
/// Une règle mal posée — « durée supérieure à 0 » — désigne toute la
/// bibliothèque. Sans borne, l'interface recevrait quatorze mille lignes d'un
/// coup. La coupe explicite de l'utilisateur, quand elle existe, passe avant.
const PLAFOND: i64 = 5000;

fn lier<'q>(
    mut q: sqlx::query::QueryAs<'q, Sqlite, PlaylistTrackView, sqlx::sqlite::SqliteArguments<'q>>,
    binds: &'q [Bind],
) -> sqlx::query::QueryAs<'q, Sqlite, PlaylistTrackView, sqlx::sqlite::SqliteArguments<'q>> {
    for b in binds {
        q = match b {
            Bind::Text(s) => q.bind(s),
            Bind::Number(n) => q.bind(*n),
        };
    }
    q
}

/// Rend les morceaux qui satisfont les règles.
pub async fn evaluate(
    pool: &SqlitePool,
    playlist_id: i64,
    rules: &SmartRules,
) -> Result<Vec<PlaylistTrackView>, String> {
    let filtre = build_where(rules)?;
    let (ordre, ordre_binds) = build_order(&rules.limit)?;

    // Le plafond ne s'ajoute que si l'utilisateur n'a pas déjà coupé.
    let plafond = if ordre.contains(" LIMIT ") {
        String::new()
    } else {
        format!(" LIMIT {PLAFOND}")
    };

    let sql = format!("{SELECT}\n{TRACK_FROM}\nWHERE {}{ordre}{plafond}", filtre.sql);

    // L'ordre des liaisons suit celui des `?` dans le texte : l'identifiant de
    // playlist du SELECT, puis les valeurs du WHERE, puis celles de l'ORDER BY.
    let mut q = sqlx::query_as::<_, PlaylistTrackView>(&sql).bind(playlist_id);
    q = lier(q, &filtre.binds);
    q = lier(q, &ordre_binds);

    q.fetch_all(pool)
        .await
        .map_err(|e| format!("Évaluation des règles : {e}"))
}

/// Compte les morceaux retenus, sans les rapporter.
///
/// Sert l'aperçu pendant qu'on compose les règles : montrer le nombre à chaque
/// frappe coûterait cher si l'on rapatriait les lignes.
pub async fn count(pool: &SqlitePool, rules: &SmartRules) -> Result<i64, String> {
    let filtre = build_where(rules)?;

    // La coupe ne s'applique pas au comptage : on veut savoir combien de
    // morceaux répondent aux règles, pas combien seront affichés.
    let sql = format!("SELECT COUNT(*)\n{TRACK_FROM}\nWHERE {}", filtre.sql);

    let mut q = sqlx::query_scalar::<_, i64>(&sql);
    for b in &filtre.binds {
        q = match b {
            Bind::Text(s) => q.bind(s.clone()),
            Bind::Number(n) => q.bind(*n),
        };
    }

    q.fetch_one(pool)
        .await
        .map_err(|e| format!("Comptage : {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::smart_playlist::rules::{Group, Limit, Match, Node, Op, Rule, Value};
    use sqlx::sqlite::SqlitePoolOptions;

    async fn base() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("base mémoire");
        sqlx::migrate!("./SQL/").run(&pool).await.expect("migrations");

        // `OR IGNORE` : les migrations posent déjà un profil par défaut, et
        // s'appuyer dessus rendrait le test dépendant de son contenu.
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'T')")
            .execute(&pool)
            .await
            .expect("profil");
        sqlx::query("INSERT OR IGNORE INTO library (id, profil_id, name) VALUES (1, 1, 'L')")
            .execute(&pool)
            .await
            .expect("bibliothèque");
        pool
    }

    /// Pose une piste complète. `tags` est le JSON du fichier, comme au scan.
    #[allow(clippy::too_many_arguments)]
    async fn piste(
        pool: &SqlitePool,
        id: &str,
        titre: &str,
        artiste: &str,
        genre: &str,
        note: Option<f64>,
        ecoutes: i64,
        tags: Option<&str>,
    ) {
        let aid = format!("a-{}", artiste.to_lowercase());
        sqlx::query(
            "INSERT OR IGNORE INTO artists (id, name, name_normalized, sort_name)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&aid)
        .bind(artiste)
        .bind(artiste.to_lowercase())
        .bind(artiste)
        .execute(pool)
        .await
        .expect("artiste");

        let chemin = format!("S:/m/{id}.flac");

        sqlx::query("INSERT INTO library_cache (path, genre, duration) VALUES (?, ?, 200)")
            .bind(&chemin)
            .bind(genre)
            .execute(pool)
            .await
            .expect("cache");

        let cid: i64 = sqlx::query_scalar("SELECT id FROM library_cache WHERE path = ?")
            .bind(&chemin)
            .fetch_one(pool)
            .await
            .expect("cache id");

        sqlx::query(
            "INSERT INTO library_files (id, library_id, cache_id, path, filename, extension, size, status)
             VALUES (?, 1, ?, ?, ?, 'flac', 1, 'ok')",
        )
        .bind(format!("f-{id}"))
        .bind(cid)
        .bind(&chemin)
        .bind(format!("{id}.flac"))
        .execute(pool)
        .await
        .expect("fichier");

        sqlx::query(
            "INSERT INTO library_tracks
                 (id, library_id, file_id, cache_id, artist_id, title, title_normalized,
                  duration, rating, play_count, tags)
             VALUES (?, 1, ?, ?, ?, ?, ?, 200, ?, ?, ?)",
        )
        .bind(id)
        .bind(format!("f-{id}"))
        .bind(cid)
        .bind(&aid)
        .bind(titre)
        .bind(titre.to_lowercase())
        .bind(note)
        .bind(ecoutes)
        .bind(tags)
        .execute(pool)
        .await
        .expect("piste");
    }

    fn r(field: &str, op: Op, value: Option<Value>) -> Node {
        Node::Rule(Rule {
            field: field.into(),
            op,
            value,
        })
    }

    fn regles(mode: Match, n: Vec<Node>, limit: Option<Limit>) -> SmartRules {
        SmartRules {
            root: Group {
                match_mode: mode,
                rules: n,
            },
            limit,
        }
    }

    #[tokio::test]
    async fn une_regle_de_note_selectionne_les_bonnes_pistes() {
        let pool = base().await;
        piste(&pool, "t1", "Un", "Nirvana", "Grunge", Some(5.0), 3, None).await;
        piste(&pool, "t2", "Deux", "Nirvana", "Grunge", Some(2.0), 0, None).await;
        piste(&pool, "t3", "Trois", "Queen", "Rock", None, 9, None).await;

        let rg = regles(
            Match::All,
            vec![r("rating", Op::Gte, Some(Value::Number(4.0)))],
            None,
        );
        assert_eq!(count(&pool, &rg).await.expect("comptage"), 1);

        let pistes = evaluate(&pool, 7, &rg).await.expect("évaluation");
        assert_eq!(pistes.len(), 1);
        assert_eq!(pistes[0].library_track_id, "t1");
        // La playlist doit se reconnaître dans ce qu'elle rend.
        assert_eq!(pistes[0].playlist_id, 7);
    }

    #[tokio::test]
    async fn une_note_absente_n_est_pas_zero() {
        // `rating` NULL veut dire « jamais noté ». Ni « au moins 3 » ni
        // « moins de 3 » ne doivent le ramener : sans note, la comparaison n'a
        // pas de sens, et le compter comme zéro fausserait toute règle de
        // sélection par qualité.
        let pool = base().await;
        piste(&pool, "t1", "Un", "A", "G", None, 0, None).await;

        for op in [Op::Gte, Op::Lt] {
            let rg = regles(
                Match::All,
                vec![r("rating", op, Some(Value::Number(3.0)))],
                None,
            );
            assert_eq!(count(&pool, &rg).await.expect("comptage"), 0);
        }
    }

    #[tokio::test]
    async fn un_groupe_any_reunit_ses_conditions() {
        let pool = base().await;
        piste(&pool, "t1", "Un", "Nirvana", "Grunge", None, 0, None).await;
        piste(&pool, "t2", "Deux", "Queen", "Rock", None, 0, None).await;
        piste(&pool, "t3", "Trois", "Miles", "Jazz", None, 0, None).await;

        let rg = regles(
            Match::Any,
            vec![
                r("genre", Op::Is, Some(Value::Text("Grunge".into()))),
                r("genre", Op::Is, Some(Value::Text("Rock".into()))),
            ],
            None,
        );
        assert_eq!(count(&pool, &rg).await.expect("comptage"), 2);
    }

    #[tokio::test]
    async fn une_regle_porte_sur_un_tag_libre() {
        // Le cœur de la richesse : filtrer sur un tag que seul le fichier porte.
        let pool = base().await;
        piste(
            &pool, "t1", "Un", "A", "G", None, 0,
            Some(r#"{"custom_tags":[["Label","Sub Pop"]]}"#),
        )
        .await;
        piste(
            &pool, "t2", "Deux", "B", "G", None, 0,
            Some(r#"{"custom_tags":[["Label","Motown"]]}"#),
        )
        .await;

        let rg = regles(
            Match::All,
            vec![r(
                "tag:custom:Label",
                Op::Is,
                Some(Value::Text("Sub Pop".into())),
            )],
            None,
        );
        let pistes = evaluate(&pool, 1, &rg).await.expect("évaluation");
        assert_eq!(pistes.len(), 1);
        assert_eq!(pistes[0].library_track_id, "t1");
    }

    #[tokio::test]
    async fn une_regle_porte_sur_un_champ_de_tag_nomme() {
        let pool = base().await;
        piste(&pool, "t1", "Un", "A", "G", None, 0, Some(r#"{"composer":"Mozart"}"#)).await;
        piste(&pool, "t2", "Deux", "B", "G", None, 0, Some(r#"{"composer":"Bach"}"#)).await;

        let rg = regles(
            Match::All,
            vec![r("tag:composer", Op::Contains, Some(Value::Text("moz".into())))],
            None,
        );
        assert_eq!(count(&pool, &rg).await.expect("comptage"), 1);
    }

    #[tokio::test]
    async fn la_negation_ramene_aussi_les_champs_vides() {
        // Une piste sans compositeur « n'a pas Mozart comme compositeur ». Sans
        // le COALESCE, SQLite rendrait NULL — donc faux — et la règle raterait
        // exactement les lignes qu'elle devrait retenir.
        let pool = base().await;
        piste(&pool, "t1", "Un", "A", "G", None, 0, Some(r#"{"composer":"Mozart"}"#)).await;
        piste(&pool, "t2", "Deux", "B", "G", None, 0, None).await;

        let rg = regles(
            Match::All,
            vec![r(
                "tag:composer",
                Op::NotContains,
                Some(Value::Text("mozart".into())),
            )],
            None,
        );
        assert_eq!(count(&pool, &rg).await.expect("comptage"), 1);
    }

    #[tokio::test]
    async fn la_coupe_et_le_tri_sont_appliques() {
        let pool = base().await;
        for (id, note) in [("t1", 1.0), ("t2", 5.0), ("t3", 3.0)] {
            piste(&pool, id, id, "A", "G", Some(note), 0, None).await;
        }

        let rg = regles(
            Match::All,
            vec![r("rating", Op::IsNotEmpty, None)],
            Some(Limit {
                count: Some(2),
                sort: Some("rating".into()),
                desc: true,
            }),
        );
        let pistes = evaluate(&pool, 1, &rg).await.expect("évaluation");
        assert_eq!(pistes.len(), 2);
        assert_eq!(pistes[0].library_track_id, "t2");
        assert_eq!(pistes[1].library_track_id, "t3");

        // Le comptage ignore la coupe : il dit combien de morceaux répondent aux
        // règles, pas combien seront affichés.
        assert_eq!(count(&pool, &rg).await.expect("comptage"), 3);
    }

    #[tokio::test]
    async fn l_ordre_rendu_est_celui_demande_dans_les_deux_sens() {
        // C'est l'ordre des lignes qui fait foi, pas un rang : une playlist
        // intelligente n'a pas de position enregistrée.
        let pool = base().await;
        piste(&pool, "t1", "Un", "A", "G", Some(1.0), 0, None).await;
        piste(&pool, "t2", "Deux", "A", "G", Some(5.0), 0, None).await;

        for (desc, premier) in [(true, "t2"), (false, "t1")] {
            let rg = regles(
                Match::All,
                vec![r("rating", Op::IsNotEmpty, None)],
                Some(Limit {
                    count: None,
                    sort: Some("rating".into()),
                    desc,
                }),
            );
            let pistes = evaluate(&pool, 1, &rg).await.expect("évaluation");
            assert_eq!(pistes[0].library_track_id, premier, "sens desc={desc}");
        }
    }

    #[tokio::test]
    async fn un_champ_inconnu_fait_echouer_proprement() {
        // Pas de filtre silencieusement ignoré : une règle intraduisible doit se
        // voir, sinon la playlist rend autre chose que ce qu'on a demandé sans
        // que rien ne l'indique.
        let pool = base().await;
        let rg = regles(
            Match::All,
            vec![r(
                "champ_qui_n_existe_pas",
                Op::Is,
                Some(Value::Text("x".into())),
            )],
            None,
        );
        let err = evaluate(&pool, 1, &rg).await.unwrap_err();
        assert!(err.contains("Champ inconnu"), "message : {err}");
    }

    #[tokio::test]
    async fn des_regles_vides_ramenent_toute_la_bibliotheque() {
        let pool = base().await;
        piste(&pool, "t1", "Un", "A", "G", None, 0, None).await;
        piste(&pool, "t2", "Deux", "B", "G", None, 0, None).await;

        let rg = regles(Match::All, vec![], None);
        assert_eq!(count(&pool, &rg).await.expect("comptage"), 2);
    }

    #[tokio::test]
    async fn un_pourcentage_saisi_ne_devient_pas_un_joker() {
        // « contient 100% » ne doit pas ramener toute la bibliothèque.
        let pool = base().await;
        piste(&pool, "t1", "Pur à 100%", "A", "G", None, 0, None).await;
        piste(&pool, "t2", "Autre chose", "B", "G", None, 0, None).await;

        let rg = regles(
            Match::All,
            vec![r("title", Op::Contains, Some(Value::Text("100%".into())))],
            None,
        );
        assert_eq!(count(&pool, &rg).await.expect("comptage"), 1);
    }

    #[tokio::test]
    async fn le_classement_somme_les_ecoutes_par_valeur() {
        // Le point qui fait toute la différence : « les genres les plus
        // écoutés » n'est pas « les genres qui contiennent le morceau le plus
        // écouté ». Ici le Jazz a une piste à 100 écoutes, le Rock en a trois à
        // 50 — soit 150. Trier sur le maximum donnerait le Jazz ; sommer donne
        // le Rock, et c'est ce qu'on veut dire.
        let pool = base().await;
        piste(&pool, "j1", "Jazz1", "A", "Jazz", None, 100, None).await;
        piste(&pool, "r1", "Rock1", "B", "Rock", None, 50, None).await;
        piste(&pool, "r2", "Rock2", "B", "Rock", None, 50, None).await;
        piste(&pool, "r3", "Rock3", "B", "Rock", None, 50, None).await;
        piste(&pool, "z1", "Zero", "C", "Ambient", None, 0, None).await;

        let rg = regles(
            Match::All,
            vec![r("genre", Op::InTopPlayed, Some(Value::Number(1.0)))],
            None,
        );
        let pistes = evaluate(&pool, 1, &rg).await.expect("évaluation");
        assert_eq!(pistes.len(), 3, "le genre le plus écouté doit être le Rock");
        for p in &pistes {
            assert!(p.library_track_id.starts_with('r'), "piste inattendue : {}", p.library_track_id);
        }
    }

    #[tokio::test]
    async fn le_classement_retient_le_nombre_demande() {
        let pool = base().await;
        piste(&pool, "a1", "A", "X", "Jazz", None, 30, None).await;
        piste(&pool, "b1", "B", "X", "Rock", None, 20, None).await;
        piste(&pool, "c1", "C", "X", "Ambient", None, 10, None).await;

        for (n, attendu) in [(1.0, 1), (2.0, 2), (3.0, 3), (9.0, 3)] {
            let rg = regles(
                Match::All,
                vec![r("genre", Op::InTopPlayed, Some(Value::Number(n)))],
                None,
            );
            assert_eq!(
                count(&pool, &rg).await.expect("comptage"),
                attendu,
                "pour les {n} premiers"
            );
        }
    }

    #[tokio::test]
    async fn le_classement_ignore_les_valeurs_vides() {
        // Un genre absent n'est pas un genre : il ne doit pas occuper un rang
        // du classement, sinon « les 3 styles les plus écoutés » en rendrait
        // deux plus un paquet de pistes sans genre.
        let pool = base().await;
        piste(&pool, "a1", "A", "X", "Jazz", None, 5, None).await;
        piste(&pool, "b1", "B", "X", "", None, 900, None).await;

        let rg = regles(
            Match::All,
            vec![r("genre", Op::InTopPlayed, Some(Value::Number(1.0)))],
            None,
        );
        let pistes = evaluate(&pool, 1, &rg).await.expect("évaluation");
        assert_eq!(pistes.len(), 1);
        assert_eq!(pistes[0].library_track_id, "a1");
    }

    #[tokio::test]
    async fn le_classement_fonctionne_sur_un_tag_libre() {
        // La liaison du nom de tag doit se faire deux fois — dehors et dans la
        // sous-requête — et dans le bon ordre. Une inversion chercherait le tag
        // nommé « 1 ».
        let pool = base().await;
        piste(&pool, "a1", "A", "X", "G", None, 100, Some(r#"{"custom_tags":[["Label","Sub Pop"]]}"#)).await;
        piste(&pool, "b1", "B", "X", "G", None, 1, Some(r#"{"custom_tags":[["Label","Motown"]]}"#)).await;

        let rg = regles(
            Match::All,
            vec![r("tag:custom:Label", Op::InTopPlayed, Some(Value::Number(1.0)))],
            None,
        );
        let pistes = evaluate(&pool, 1, &rg).await.expect("évaluation");
        assert_eq!(pistes.len(), 1);
        assert_eq!(pistes[0].library_track_id, "a1");
    }

    #[tokio::test]
    async fn un_rang_nul_est_refuse() {
        let pool = base().await;
        let rg = regles(
            Match::All,
            vec![r("genre", Op::InTopPlayed, Some(Value::Number(0.0)))],
            None,
        );
        let err = evaluate(&pool, 1, &rg).await.unwrap_err();
        assert!(err.contains("au moins un rang"), "message : {err}");
    }

    #[tokio::test]
    async fn les_plus_ecoutes_se_composent_avec_le_reste() {
        // La recette « mes morceaux préférés dans mes styles favoris » : le
        // classement doit se combiner aux autres conditions sans les gêner.
        let pool = base().await;
        piste(&pool, "a1", "A", "X", "Rock", Some(5.0), 90, None).await;
        piste(&pool, "a2", "B", "X", "Rock", Some(1.0), 80, None).await;
        piste(&pool, "b1", "C", "X", "Jazz", Some(5.0), 1, None).await;

        let rg = regles(
            Match::All,
            vec![
                r("genre", Op::InTopPlayed, Some(Value::Number(1.0))),
                r("rating", Op::Gte, Some(Value::Number(4.0))),
            ],
            None,
        );
        let pistes = evaluate(&pool, 1, &rg).await.expect("évaluation");
        assert_eq!(pistes.len(), 1);
        assert_eq!(pistes[0].library_track_id, "a1");
    }

    #[tokio::test]
    async fn chaque_piste_ne_sort_qu_une_fois() {
        // L'interface indexe la liste par `library_track_id`. Une jointure qui
        // se multiplierait — un album à plusieurs artistes, par exemple —
        // rendrait la même piste deux fois, et l'affichage refuserait la liste
        // entière pour clé en double. Le test verrouille l'unicité à la source.
        let pool = base().await;
        piste(&pool, "t1", "Un", "A", "Rock", Some(5.0), 3, None).await;
        piste(&pool, "t2", "Deux", "A", "Rock", Some(4.0), 2, None).await;
        piste(&pool, "t3", "Trois", "B", "Jazz", Some(4.0), 1, None).await;

        // Un album partagé par deux artistes : la configuration qui, avec une
        // jointure de trop, dupliquerait les lignes.
        sqlx::query("INSERT INTO library_albums (id, library_id, artist_id, title, title_normalized) VALUES ('al1', 1, 'a-a', 'Commun', 'commun')")
            .execute(&pool)
            .await
            .expect("album");
        sqlx::query("UPDATE library_tracks SET library_album_id = 'al1'")
            .execute(&pool)
            .await
            .expect("rattachement");
        sqlx::query("INSERT INTO library_album_artists (library_id, library_album_id, artist_id) VALUES (1, 'al1', 'a-a'), (1, 'al1', 'a-b')")
            .execute(&pool)
            .await
            .ok();

        let rg = regles(
            Match::All,
            vec![r("rating", Op::IsNotEmpty, None)],
            None,
        );
        let pistes = evaluate(&pool, 1, &rg).await.expect("évaluation");

        let mut vus = std::collections::HashSet::new();
        for p in &pistes {
            assert!(
                vus.insert(p.library_track_id.clone()),
                "piste rendue deux fois : {}",
                p.library_track_id
            );
        }
        assert_eq!(pistes.len(), 3);
    }

    /// Le cycle complet d'une playlist intelligente, contre une vraie base.
    ///
    /// Créer puis relire puis modifier passe par trois traductions successives —
    /// structure vers JSON, JSON vers base, base vers structure. Chacune peut se
    /// tromper sans que le compilateur n'y voie rien.
    #[tokio::test]
    async fn une_playlist_se_cree_se_relit_et_se_modifie() {
        let pool = base().await;
        piste(&pool, "t1", "Un", "A", "Rock", Some(5.0), 3, None).await;
        piste(&pool, "t2", "Deux", "A", "Rock", Some(1.0), 0, None).await;

        let depart = SmartRules {
            root: Group {
                match_mode: Match::All,
                rules: vec![r("rating", Op::Gte, Some(Value::Number(4.0)))],
            },
            limit: Some(Limit {
                count: Some(10),
                sort: Some("rating".into()),
                desc: true,
            }),
        };

        // ── Création ──
        let json = serde_json::to_string(&depart).expect("sérialisation");
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO playlists (profil_id, name, color, icon, is_smart, rules)
             VALUES (1, 'Test', '#fff', 'i', 1, ?) RETURNING id",
        )
        .bind(&json)
        .fetch_one(&pool)
        .await
        .expect("création");

        // ── Relecture ──
        let relu: Option<String> =
            sqlx::query_scalar("SELECT rules FROM playlists WHERE id = ? AND is_smart = 1")
                .bind(id)
                .fetch_optional(&pool)
                .await
                .expect("lecture")
                .flatten();

        let relu: SmartRules =
            serde_json::from_str(&relu.expect("règles présentes")).expect("relecture");
        assert_eq!(evaluate(&pool, id, &relu).await.expect("évaluation").len(), 1);

        // ── Modification ──
        let modifie = SmartRules {
            root: Group {
                match_mode: Match::Any,
                rules: vec![r("play_count", Op::Gt, Some(Value::Number(0.0)))],
            },
            limit: None,
        };
        let json = serde_json::to_string(&modifie).expect("sérialisation");

        sqlx::query(
            "UPDATE playlists SET
                 name = COALESCE(?, name),
                 color = COALESCE(?, color),
                 icon = COALESCE(?, icon),
                 rules = ?, is_smart = 1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?",
        )
        .bind(Some("Renommée"))
        .bind(None::<String>)
        .bind(None::<String>)
        .bind(&json)
        .bind(id)
        .execute(&pool)
        .await
        .expect("modification");

        // ── Les nouvelles règles ont bien pris ──
        let apres: String =
            sqlx::query_scalar("SELECT rules FROM playlists WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .expect("relecture 2");
        let apres: SmartRules = serde_json::from_str(&apres).expect("relecture 2");
        let pistes = evaluate(&pool, id, &apres).await.expect("évaluation 2");
        assert_eq!(pistes.len(), 1);
        assert_eq!(pistes[0].library_track_id, "t1");

        // Le nom a changé, la couleur non — `COALESCE` avec `NULL` conserve.
        let (nom, couleur): (String, String) =
            sqlx::query_as("SELECT name, color FROM playlists WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .expect("identité");
        assert_eq!(nom, "Renommée");
        assert_eq!(couleur, "#fff");
    }
}
