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
fn split_path(chemin: &str) -> (&str, &str) {
    match chemin.rfind(['/', '\\']) {
        Some(i) => (&chemin[..i], &chemin[i + 1..]),
        None => ("", chemin),
    }
}

/// Le dossier de l'album pour le fichier donné. Vide si le chemin n'en a pas.
pub fn album_root(chemin: &str) -> String {
    let (dossier, _) = split_path(chemin);
    if dossier.is_empty() {
        return String::new();
    }
    let (parent, nom) = split_path(dossier);
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
    fn an_ordinary_album_keeps_its_folder() {
        assert_eq!(
            album_root(r"S:\Musique\Daft Punk\1997 - Homework\01.flac"),
            r"S:\Musique\Daft Punk\1997 - Homework"
        );
    }

    #[test]
    fn box_set_discs_come_together() {
        let un = album_root(r"S:\Musique\NRJ\2019 - Hits\CD1\01.flac");
        let deux = album_root(r"S:\Musique\NRJ\2019 - Hits\CD3\04.flac");
        assert_eq!(un, r"S:\Musique\NRJ\2019 - Hits");
        assert_eq!(un, deux);
    }

    #[test]
    fn disc_number_spellings() {
        for nom in ["CD 1", "cd2", "Disc 3", "Disque 1", "disk4", "Vol. 2", "CD10"] {
            assert_eq!(
                album_root(&format!(r"S:\A\Album\{nom}\01.flac")),
                r"S:\A\Album",
                "« {nom} » aurait dû être reconnu"
            );
        }
    }

    #[test]
    fn a_folder_that_looks_like_a_disc_but_is_not() {
        // Un vrai titre d'album, pas un numéro de disque.
        assert_eq!(
            album_root(r"S:\A\CD Single Collection\01.flac"),
            r"S:\A\CD Single Collection"
        );
    }

    #[test]
    fn forward_slashes_work_too() {
        assert_eq!(album_root("/home/d/Musique/Album/CD2/01.flac"), "/home/d/Musique/Album");
    }

    #[test]
    fn a_path_without_folder_gives_nothing() {
        assert_eq!(album_root("01.flac"), "");
    }

    #[test]
    fn a_disc_at_the_root_stays_where_it_is() {
        // Remonter donnerait une chaîne vide : on préfère garder le dossier.
        assert_eq!(album_root(r"CD1\01.flac"), "CD1");
    }
}
