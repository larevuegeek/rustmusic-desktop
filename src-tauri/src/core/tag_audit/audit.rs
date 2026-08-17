//! Ce qu'il y a à corriger dans une bibliothèque.
//!
//! # Une seule lecture, toutes les analyses
//! Chercher séparément les titres vides, les années manquantes, les albums
//! incohérents et les doublons ferait autant de parcours de la table. Sur
//! cinquante mille morceaux c'est le genre de détail qui fait la différence
//! entre un écran instantané et un écran qu'on évite d'ouvrir.
//!
//! La commande lit donc tout une fois, et ce module fait le reste **en
//! mémoire**. Il ne connaît ni SQL ni Tauri : il prend des lignes, il rend des
//! constats. C'est ce qui le rend testable — la moitié des cas traités ici
//! (les trous dans une numérotation, une compilation non déclarée, deux rips
//! du même disque) sont pénibles à fabriquer en base et triviaux à écrire en
//! tableau.
//!
//! # Ce qu'on signale, et ce qu'on ne signale pas
//! Un constat qui ne mène à aucune action est du bruit. Chaque catégorie doit
//! répondre à « et donc, je fais quoi ? » :
//! - un champ vide se remplit,
//! - une incohérence d'album se tranche,
//! - un doublon se supprime.
//!
//! On ne signale donc **pas** ce qui est simplement inhabituel : un album sans
//! genre, une pochette de faible résolution, un titre en majuscules. Ces
//! jugements relèvent des règles de nettoyage, pas de l'inventaire.

use std::collections::HashMap;

use crate::core::metadata_source::matching::normalize;

/// Tolérance de durée entre deux copies d'un même morceau.
///
/// Deux extractions du même disque ne donnent pas la même durée à
/// l'échantillon près : le silence de fin de piste, le décalage d'un
/// encodeur avec ou sans gapless, un lecteur qui arrondit. Deux secondes
/// couvrent ces écarts sans confondre deux versions réellement différentes —
/// une édition longue ou un live se distinguent de bien plus.
const DURATION_TOLERANCE: f64 = 2.0;

/// Formats dont on sait réécrire les tags.
///
/// Le reste — WAV, AIFF, Opus, AAC — se lit mais ne se corrige pas. Le dire
/// évite qu'on charge dans l'atelier des fichiers qu'il refusera d'écrire.
const WRITABLE: [&str; 4] = ["mp3", "flac", "dsf", "dff"];

/// Un morceau tel que l'inventaire a besoin de le connaître.
#[derive(Debug, Clone, Default)]
pub struct TrackRow {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub year: String,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub duration: Option<f64>,
    pub has_cover: bool,
    /// En minuscules, sans le point.
    pub extension: String,
}

/// Ce qu'un constat demande comme geste, et donc où il se range à l'écran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Un champ sans lequel le morceau est introuvable dans la bibliothèque.
    Essential,
    /// Un champ manquant qui gêne le classement, sans empêcher l'écoute.
    Incomplete,
    /// Les morceaux se contredisent entre eux.
    Coherence,
    /// Le même enregistrement présent plusieurs fois.
    Duplicate,
    /// Rien à corriger : le format n'accepte pas de réécriture.
    Blocked,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Essential => "essential",
            Severity::Incomplete => "incomplete",
            Severity::Coherence => "coherence",
            Severity::Duplicate => "duplicate",
            Severity::Blocked => "blocked",
        }
    }
}

/// Une catégorie de constat, avec les fichiers qu'elle désigne.
///
/// Les chemins sont l'unique raison d'être de cette structure : ce sont eux
/// qu'on verse dans l'atelier. Le reste sert à décider si on clique.
#[derive(Debug, Clone)]
pub struct AuditGroup {
    /// Identifiant stable, clé de traduction côté interface.
    pub kind: &'static str,
    pub severity: Severity,
    pub paths: Vec<String>,
    /// Quelques noms, pour que la ligne dise de quoi elle parle.
    pub samples: Vec<String>,
}

impl AuditGroup {
    fn new(kind: &'static str, severity: Severity) -> Self {
        Self {
            kind,
            severity,
            paths: Vec::new(),
            samples: Vec::new(),
        }
    }

    fn push(&mut self, path: &str, sample: &str) {
        self.paths.push(path.to_string());
        // Trois exemples suffisent à reconnaître de quoi il s'agit ; en
        // afficher vingt repousserait les autres catégories hors de l'écran.
        if self.samples.len() < 3 && !self.samples.iter().any(|s| s == sample) {
            self.samples.push(sample.to_string());
        }
    }
}

/// Nom lisible d'un morceau, pour les exemples.
fn label(row: &TrackRow) -> String {
    if !row.title.trim().is_empty() {
        return row.title.trim().to_string();
    }
    // Sans titre, le nom du fichier est la seule prise qu'on ait — et c'est
    // précisément le cas des morceaux qu'on vient corriger.
    row.path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&row.path)
        .to_string()
}

/// Clé d'album : l'artiste d'album s'il existe, sinon celui de la piste.
///
/// Se rabattre sur l'artiste de piste est ce qui permet de détecter une
/// compilation non déclarée : les morceaux se regroupent alors par titre
/// d'album, et leurs artistes divergents deviennent visibles.
fn album_key(row: &TrackRow) -> Option<String> {
    let album = normalize(&row.album);
    if album.is_empty() {
        return None;
    }
    let artist = if row.album_artist.trim().is_empty() {
        String::new()
    } else {
        normalize(&row.album_artist)
    };
    Some(format!("{artist}\u{1}{album}"))
}

/// Passe l'inventaire en revue.
pub fn analyse(rows: &[TrackRow]) -> Vec<AuditGroup> {
    let mut groups: Vec<AuditGroup> = Vec::new();
    let mut add = |group: AuditGroup| {
        if !group.paths.is_empty() {
            groups.push(group);
        }
    };

    // ─── Par morceau ───
    let mut no_title = AuditGroup::new("missing_title", Severity::Essential);
    let mut no_artist = AuditGroup::new("missing_artist", Severity::Essential);
    let mut no_album = AuditGroup::new("missing_album", Severity::Essential);
    let mut no_year = AuditGroup::new("missing_year", Severity::Incomplete);
    let mut no_number = AuditGroup::new("missing_track_number", Severity::Incomplete);
    let mut no_cover = AuditGroup::new("missing_cover", Severity::Incomplete);
    let mut not_writable = AuditGroup::new("not_writable", Severity::Blocked);

    for row in rows {
        let name = label(row);
        if row.title.trim().is_empty() {
            no_title.push(&row.path, &name);
        }
        if row.artist.trim().is_empty() {
            no_artist.push(&row.path, &name);
        }
        if row.album.trim().is_empty() {
            no_album.push(&row.path, &name);
        }
        if row.year.trim().is_empty() {
            no_year.push(&row.path, &name);
        }
        if row.track_number.is_none() {
            no_number.push(&row.path, &name);
        }
        if !row.has_cover {
            no_cover.push(&row.path, &name);
        }
        if !WRITABLE.contains(&row.extension.as_str()) {
            not_writable.push(&row.path, &name);
        }
    }

    add(no_title);
    add(no_artist);
    add(no_album);
    add(no_year);
    add(no_number);
    add(no_cover);
    add(not_writable);

    // ─── Par album ───
    let mut albums: HashMap<String, Vec<&TrackRow>> = HashMap::new();
    for row in rows {
        if let Some(key) = album_key(row) {
            albums.entry(key).or_default().push(row);
        }
    }

    let mut year_mismatch = AuditGroup::new("album_year_mismatch", Severity::Coherence);
    let mut compilation = AuditGroup::new("undeclared_compilation", Severity::Coherence);
    let mut duplicate_number = AuditGroup::new("duplicate_track_number", Severity::Coherence);
    let mut number_gap = AuditGroup::new("track_number_gap", Severity::Coherence);

    // Ordre stable : une table de hachage rend ses clés dans un ordre qui
    // change d'une exécution à l'autre, et un écran qui se réordonne tout seul
    // entre deux ouvertures donne l'impression que les données ont bougé.
    let mut keys: Vec<&String> = albums.keys().collect();
    keys.sort();

    for key in keys {
        let tracks = &albums[key];
        let name = tracks[0].album.trim().to_string();

        // Années divergentes. Les valeurs vides ne comptent pas : c'est le
        // constat « année manquante », déjà porté ailleurs.
        let mut years: Vec<&str> = tracks
            .iter()
            .map(|t| t.year.trim())
            .filter(|y| !y.is_empty())
            .collect();
        years.sort_unstable();
        years.dedup();
        if years.len() > 1 {
            for track in tracks {
                year_mismatch.push(&track.path, &name);
            }
        }

        // Compilation non déclarée : plusieurs artistes, aucun artiste
        // d'album pour les chapeauter. Sans ce champ, la plupart des lecteurs
        // éclatent l'album en autant d'entrées qu'il a d'artistes.
        let declared = tracks
            .iter()
            .any(|t| !t.album_artist.trim().is_empty());
        if !declared {
            let mut artists: Vec<String> = tracks
                .iter()
                .map(|t| normalize(&t.artist))
                .filter(|a| !a.is_empty())
                .collect();
            artists.sort();
            artists.dedup();
            if artists.len() > 1 {
                for track in tracks {
                    compilation.push(&track.path, &name);
                }
            }
        }

        // Numérotation, disque par disque : sur un coffret les numéros
        // repartent à 1, et les confondre signalerait des doublons partout.
        let mut discs: HashMap<u32, Vec<&TrackRow>> = HashMap::new();
        for track in tracks {
            discs.entry(track.disc_number.unwrap_or(1)).or_default().push(track);
        }

        let mut disc_keys: Vec<&u32> = discs.keys().collect();
        disc_keys.sort();

        for disc in disc_keys {
            let disc_tracks = &discs[disc];
            let mut seen: HashMap<u32, usize> = HashMap::new();
            for track in disc_tracks {
                if let Some(number) = track.track_number {
                    *seen.entry(number).or_insert(0) += 1;
                }
            }

            if seen.values().any(|count| *count > 1) {
                for track in disc_tracks {
                    duplicate_number.push(&track.path, &name);
                }
            }

            // Trou dans la séquence. On ne le cherche que si tous les
            // morceaux sont numérotés : sinon c'est « numéro manquant » qu'on
            // redirait sous un autre nom.
            let numbers: Vec<u32> = disc_tracks.iter().filter_map(|t| t.track_number).collect();
            if numbers.len() == disc_tracks.len() && !numbers.is_empty() {
                let max = *numbers.iter().max().unwrap();
                let unique = seen.len() as u32;
                // Un album incomplet — on n'a que les pistes 3, 7 et 12 —
                // n'est pas une erreur de tag. Le trou ne se signale que si la
                // séquence est presque complète.
                if max > unique && unique * 2 > max {
                    for track in disc_tracks {
                        number_gap.push(&track.path, &name);
                    }
                }
            }
        }
    }

    add(year_mismatch);
    add(compilation);
    add(duplicate_number);
    add(number_gap);

    // ─── Doublons ───
    add(find_duplicates(rows));

    groups
}

/// Le même enregistrement, à deux endroits.
///
/// Titre et artiste normalisés donnent les candidats ; la durée tranche. Sans
/// elle, deux versions d'un même morceau — studio et live, original et remix —
/// seraient déclarées identiques, et une suppression en masse coûterait cher.
fn find_duplicates(rows: &[TrackRow]) -> AuditGroup {
    let mut group = AuditGroup::new("duplicate", Severity::Duplicate);

    let mut buckets: HashMap<String, Vec<&TrackRow>> = HashMap::new();
    for row in rows {
        let title = normalize(&row.title);
        let artist = normalize(&row.artist);
        if title.is_empty() || artist.is_empty() {
            continue;
        }
        buckets
            .entry(format!("{artist}\u{1}{title}"))
            .or_default()
            .push(row);
    }

    let mut keys: Vec<&String> = buckets.keys().collect();
    keys.sort();

    for key in keys {
        let candidates = &buckets[key];
        if candidates.len() < 2 {
            continue;
        }

        // Regroupement par durée proche, de proche en proche : trois copies
        // à 180,0 / 181,5 / 183,0 secondes sont bien le même morceau, alors
        // qu'aucune comparaison deux à deux ne les relie toutes.
        let mut sorted: Vec<&&TrackRow> = candidates.iter().collect();
        sorted.sort_by(|a, b| {
            a.duration
                .unwrap_or(0.0)
                .partial_cmp(&b.duration.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut cluster: Vec<&&TrackRow> = Vec::new();
        let flush = |cluster: &mut Vec<&&TrackRow>, group: &mut AuditGroup| {
            if cluster.len() > 1 {
                let name = label(cluster[0]);
                for track in cluster.iter() {
                    group.push(&track.path, &name);
                }
            }
            cluster.clear();
        };

        for track in sorted {
            match cluster.last() {
                None => cluster.push(track),
                Some(previous) => {
                    let close = match (previous.duration, track.duration) {
                        (Some(a), Some(b)) => (a - b).abs() <= DURATION_TOLERANCE,
                        // Sans durée des deux côtés, titre et artiste
                        // identiques suffisent : c'est déjà un signal fort, et
                        // rien n'est supprimé sans que l'utilisateur regarde.
                        (None, None) => true,
                        _ => false,
                    };
                    if close {
                        cluster.push(track);
                    } else {
                        flush(&mut cluster, &mut group);
                        cluster.push(track);
                    }
                }
            }
        }
        flush(&mut cluster, &mut group);
    }

    group
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(path: &str, title: &str, artist: &str, album: &str) -> TrackRow {
        TrackRow {
            path: path.into(),
            title: title.into(),
            artist: artist.into(),
            album: album.into(),
            year: "2020".into(),
            track_number: Some(1),
            has_cover: true,
            extension: "flac".into(),
            duration: Some(200.0),
            ..Default::default()
        }
    }

    fn find<'a>(groups: &'a [AuditGroup], kind: &str) -> Option<&'a AuditGroup> {
        groups.iter().find(|g| g.kind == kind)
    }

    #[test]
    fn bibliotheque_saine_ne_signale_rien() {
        let rows = vec![
            TrackRow { track_number: Some(1), ..track("/a.flac", "Un", "Artiste", "Album") },
            TrackRow { track_number: Some(2), ..track("/b.flac", "Deux", "Artiste", "Album") },
        ];
        assert!(analyse(&rows).is_empty());
    }

    #[test]
    fn champs_vides_par_categorie() {
        let rows = vec![
            TrackRow { title: String::new(), ..track("/a.flac", "", "Artiste", "Album") },
            TrackRow { year: String::new(), ..track("/b.flac", "Deux", "Artiste", "Album") },
        ];
        let groups = analyse(&rows);

        let titles = find(&groups, "missing_title").expect("titre manquant");
        assert_eq!(titles.paths, vec!["/a.flac"]);
        assert_eq!(titles.severity, Severity::Essential);

        let years = find(&groups, "missing_year").expect("année manquante");
        assert_eq!(years.paths, vec!["/b.flac"]);
        assert_eq!(years.severity, Severity::Incomplete);
    }

    #[test]
    fn sans_titre_l_exemple_prend_le_nom_du_fichier() {
        let rows = vec![TrackRow {
            title: String::new(),
            ..track("/musique/track01.mp3", "", "Artiste", "Album")
        }];
        let groups = analyse(&rows);
        assert_eq!(find(&groups, "missing_title").unwrap().samples, vec!["track01.mp3"]);
    }

    #[test]
    fn format_non_reinscriptible() {
        let rows = vec![TrackRow {
            extension: "wav".into(),
            ..track("/a.wav", "Un", "Artiste", "Album")
        }];
        let groups = analyse(&rows);
        let blocked = find(&groups, "not_writable").expect("format bloqué");
        assert_eq!(blocked.severity, Severity::Blocked);
    }

    #[test]
    fn annees_divergentes_sur_un_meme_album() {
        let rows = vec![
            TrackRow { year: "1997".into(), ..track("/a.flac", "Un", "Artiste", "Album") },
            TrackRow { year: "1998".into(), track_number: Some(2), ..track("/b.flac", "Deux", "Artiste", "Album") },
        ];
        let groups = analyse(&rows);
        let mismatch = find(&groups, "album_year_mismatch").expect("années divergentes");
        // Tout l'album est concerné : on ne saurait pas dire laquelle est juste.
        assert_eq!(mismatch.paths.len(), 2);
        assert_eq!(mismatch.samples, vec!["Album"]);
    }

    #[test]
    fn une_annee_manquante_n_est_pas_une_divergence() {
        let rows = vec![
            TrackRow { year: "1997".into(), ..track("/a.flac", "Un", "Artiste", "Album") },
            TrackRow { year: String::new(), track_number: Some(2), ..track("/b.flac", "Deux", "Artiste", "Album") },
        ];
        let groups = analyse(&rows);
        assert!(find(&groups, "album_year_mismatch").is_none());
        assert!(find(&groups, "missing_year").is_some());
    }

    #[test]
    fn compilation_non_declaree() {
        let rows = vec![
            track("/a.flac", "Un", "Premier", "Bande originale"),
            TrackRow { track_number: Some(2), ..track("/b.flac", "Deux", "Second", "Bande originale") },
        ];
        let groups = analyse(&rows);
        assert_eq!(find(&groups, "undeclared_compilation").unwrap().paths.len(), 2);
    }

    #[test]
    fn artiste_d_album_renseigne_ne_signale_rien() {
        let rows = vec![
            TrackRow { album_artist: "Divers".into(), ..track("/a.flac", "Un", "Premier", "BO") },
            TrackRow {
                album_artist: "Divers".into(),
                track_number: Some(2),
                ..track("/b.flac", "Deux", "Second", "BO")
            },
        ];
        assert!(find(&analyse(&rows), "undeclared_compilation").is_none());
    }

    #[test]
    fn numeros_en_double() {
        let rows = vec![
            track("/a.flac", "Un", "Artiste", "Album"),
            track("/b.flac", "Deux", "Artiste", "Album"),
        ];
        assert_eq!(find(&analyse(&rows), "duplicate_track_number").unwrap().paths.len(), 2);
    }

    #[test]
    fn les_disques_d_un_coffret_ne_se_confondent_pas() {
        let rows = vec![
            TrackRow { disc_number: Some(1), ..track("/a.flac", "Un", "Artiste", "Album") },
            TrackRow { disc_number: Some(2), ..track("/b.flac", "Deux", "Artiste", "Album") },
        ];
        assert!(find(&analyse(&rows), "duplicate_track_number").is_none());
    }

    #[test]
    fn trou_dans_la_numerotation() {
        // 1, 2, 4 : la piste 3 manque.
        let rows = vec![
            TrackRow { track_number: Some(1), ..track("/a.flac", "Un", "Artiste", "Album") },
            TrackRow { track_number: Some(2), ..track("/b.flac", "Deux", "Artiste", "Album") },
            TrackRow { track_number: Some(4), ..track("/c.flac", "Trois", "Artiste", "Album") },
        ];
        assert_eq!(find(&analyse(&rows), "track_number_gap").unwrap().paths.len(), 3);
    }

    #[test]
    fn un_album_incomplet_n_est_pas_un_trou() {
        // On ne possède que les pistes 3 et 12 : c'est une compilation
        // personnelle, pas une numérotation fautive.
        let rows = vec![
            TrackRow { track_number: Some(3), ..track("/a.flac", "Un", "Artiste", "Album") },
            TrackRow { track_number: Some(12), ..track("/b.flac", "Deux", "Artiste", "Album") },
        ];
        assert!(find(&analyse(&rows), "track_number_gap").is_none());
    }

    #[test]
    fn doublons_a_deux_secondes_pres() {
        let rows = vec![
            TrackRow { duration: Some(180.0), ..track("/disque1/a.flac", "Même", "Artiste", "A") },
            TrackRow { duration: Some(181.4), ..track("/disque2/a.mp3", "Même", "Artiste", "B") },
        ];
        let groups = analyse(&rows);
        assert_eq!(find(&groups, "duplicate").unwrap().paths.len(), 2);
    }

    #[test]
    fn deux_versions_differentes_ne_sont_pas_des_doublons() {
        let rows = vec![
            TrackRow { duration: Some(180.0), ..track("/studio.flac", "Même", "Artiste", "A") },
            TrackRow { duration: Some(420.0), ..track("/live.flac", "Même", "Artiste", "B") },
        ];
        assert!(find(&analyse(&rows), "duplicate").is_none());
    }

    #[test]
    fn les_doublons_se_chainent_de_proche_en_proche() {
        // 180 / 181,5 / 183 : aucune paire extrême ne tient dans la tolérance,
        // mais les trois sont bien le même enregistrement.
        let rows = vec![
            TrackRow { duration: Some(180.0), ..track("/a.flac", "Même", "Artiste", "A") },
            TrackRow { duration: Some(181.5), ..track("/b.flac", "Même", "Artiste", "B") },
            TrackRow { duration: Some(183.0), ..track("/c.flac", "Même", "Artiste", "C") },
        ];
        assert_eq!(find(&analyse(&rows), "duplicate").unwrap().paths.len(), 3);
    }

    #[test]
    fn la_casse_et_les_accents_ne_cachent_pas_un_doublon() {
        let rows = vec![
            track("/a.flac", "Été", "Artiste", "A"),
            track("/b.flac", "ETE", "artiste", "B"),
        ];
        assert_eq!(find(&analyse(&rows), "duplicate").unwrap().paths.len(), 2);
    }
}
