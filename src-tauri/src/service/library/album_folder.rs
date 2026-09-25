//! Le dossier qui identifie un album, disques multiples rassemblés.
//!
//! L'artiste ne peut pas servir d'identité : une compilation sans tag
//! « artiste de l'album » fondait un album par piste. Le dossier, lui, ne
//! ment pas — à condition de remonter d'un cran sur les `CD1`, `CD2`…

use regex::Regex;
use std::sync::LazyLock;

/// Un sous-dossier qui ne porte qu'un numéro de disque.
static DISQUE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(cd|disc|disque|disk|vol)[\s._-]*\d{1,2}$").expect("motif de disque")
});

/// Sépare un chemin sur le dernier séparateur, quel qu'il soit.
fn couper(chemin: &str) -> (&str, &str) {
    match chemin.rfind(['/', '\\']) {
        Some(i) => (&chemin[..i], &chemin[i + 1..]),
        None => ("", chemin),
    }
}

/// Le dossier de l'album pour le fichier donné. Vide si le chemin n'en a pas.
pub fn racine_album(chemin: &str) -> String {
    let (dossier, _) = couper(chemin);
    if dossier.is_empty() {
        return String::new();
    }
    let (parent, nom) = couper(dossier);
    if !parent.is_empty() && DISQUE.is_match(nom) {
        parent.to_string()
    } else {
        dossier.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_album_ordinaire_garde_son_dossier() {
        assert_eq!(
            racine_album(r"S:\Musique\Daft Punk\1997 - Homework\01.flac"),
            r"S:\Musique\Daft Punk\1997 - Homework"
        );
    }

    #[test]
    fn les_disques_d_un_coffret_se_rejoignent() {
        let un = racine_album(r"S:\Musique\NRJ\2019 - Hits\CD1\01.flac");
        let deux = racine_album(r"S:\Musique\NRJ\2019 - Hits\CD3\04.flac");
        assert_eq!(un, r"S:\Musique\NRJ\2019 - Hits");
        assert_eq!(un, deux);
    }

    #[test]
    fn les_ecritures_du_numero_de_disque() {
        for nom in ["CD 1", "cd2", "Disc 3", "Disque 1", "disk4", "Vol. 2", "CD10"] {
            assert_eq!(
                racine_album(&format!(r"S:\A\Album\{nom}\01.flac")),
                r"S:\A\Album",
                "« {nom} » aurait dû être reconnu"
            );
        }
    }

    #[test]
    fn un_dossier_qui_ressemble_a_un_disque_sans_l_etre() {
        // Un vrai titre d'album, pas un numéro de disque.
        assert_eq!(
            racine_album(r"S:\A\CD Single Collection\01.flac"),
            r"S:\A\CD Single Collection"
        );
    }

    #[test]
    fn les_barres_obliques_marchent_aussi() {
        assert_eq!(racine_album("/home/d/Musique/Album/CD2/01.flac"), "/home/d/Musique/Album");
    }

    #[test]
    fn un_chemin_sans_dossier_ne_donne_rien() {
        assert_eq!(racine_album("01.flac"), "");
    }

    #[test]
    fn un_disque_a_la_racine_reste_ou_il_est() {
        // Remonter donnerait une chaîne vide : on préfère garder le dossier.
        assert_eq!(racine_album(r"CD1\01.flac"), "CD1");
    }
}
