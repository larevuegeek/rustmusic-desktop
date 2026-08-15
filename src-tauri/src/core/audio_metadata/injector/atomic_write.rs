//! Remplacement sûr d'un fichier audio.
//!
//! # Le risque qu'on écarte
//! Modifier un tag oblige souvent à réécrire le fichier entier : un bloc de
//! métadonnées qui grandit décale tout l'audio qui suit. Si l'application ou
//! la machine s'arrête au milieu de cette réécriture, l'utilisateur perd le
//! morceau — pas un réglage, **son fichier**. C'est la seule opération de
//! RustMusic qui puisse détruire des données de l'utilisateur.
//!
//! # La parade
//! On n'écrit jamais par-dessus l'original. On écrit un fichier temporaire
//! **dans le même dossier**, puis on le renomme sur l'original. Le
//! renommage au sein d'un même système de fichiers est atomique : à tout
//! instant le chemin d'origine désigne soit l'ancien fichier complet, soit le
//! nouveau, jamais un état intermédiaire.
//!
//! Le même dossier n'est pas un détail : entre deux volumes, `rename` devient
//! une copie suivie d'une suppression, et l'atomicité disparaît.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Suffixe du fichier temporaire. Assez reconnaissable pour qu'un résidu
/// laissé par un plantage soit identifiable, et assez improbable pour ne pas
/// écraser un fichier de l'utilisateur.
const TEMP_SUFFIX: &str = ".rustmusic-tmp";

/// Écrit `contents` à la place de `target`, de façon atomique.
///
/// Les permissions de l'original sont reportées sur le remplaçant : sans ça,
/// un fichier en lecture seule ou aux droits restreints reviendrait aux droits
/// par défaut après édition.
pub fn replace_file(target: &Path, contents: &[u8]) -> io::Result<()> {
    replace_file_with(target, |out| out.write_all(contents))
}

/// Même garantie, mais le contenu est **écrit au fil de l'eau**.
///
/// # Pourquoi cette variante existe
/// Un DSD64 stéréo pèse couramment 300 Mo. Passer par `replace_file` obligerait
/// à en tenir deux copies en mémoire — l'original lu et le résultat construit —
/// pour changer un titre, pendant que l'utilisateur écoute peut-être autre
/// chose. Les conteneurs qui savent où est leur audio peuvent le recopier par
/// blocs (`io::copy`) et n'occuper qu'un tampon.
///
/// `fill` reçoit le fichier temporaire déjà ouvert ; le renommage n'a lieu que
/// si elle réussit.
pub fn replace_file_with<F>(target: &Path, fill: F) -> io::Result<()>
where
    F: FnOnce(&mut fs::File) -> io::Result<()>,
{
    let temp = temp_path_for(target)?;

    // Un résidu d'une tentative précédente ne doit pas faire échouer celle-ci.
    let _ = fs::remove_file(&temp);

    // Portée volontairement courte : le fichier doit être fermé avant le
    // renommage, sinon Windows refuse l'opération.
    let written = (|| -> io::Result<()> {
        let mut file = fs::File::create(&temp)?;
        fill(&mut file)?;
        // Le renommage est atomique, mais il ne garantit rien sur ce que le
        // disque a réellement enregistré. Sans cette synchronisation, une
        // coupure juste après le renommage peut laisser un fichier tronqué à
        // la place de l'original — précisément ce qu'on cherche à éviter.
        file.sync_all()
    })();

    if let Err(e) = written {
        let _ = fs::remove_file(&temp);
        return Err(e);
    }

    // Les permissions de l'original sont posées sur le remplaçant : sans ça,
    // un fichier en lecture seule reviendrait aux droits par défaut.
    let original = fs::metadata(target).map(|m| m.permissions()).ok();
    if let Some(perms) = &original {
        // Best-effort : un échec ici ne justifie pas de perdre l'édition.
        let _ = fs::set_permissions(&temp, perms.clone());

        // Windows refuse de renommer **par-dessus** un fichier en lecture
        // seule (vérifié : `os error 5`). L'attribut est donc levé le temps du
        // remplacement — le remplaçant, lui, le porte déjà. Sans ça, toute une
        // bibliothèque restaurée depuis une sauvegarde serait inéditable.
        if perms.readonly() {
            let mut writable = perms.clone();
            writable.set_readonly(false);
            let _ = fs::set_permissions(target, writable);
        }
    }

    match rename_with_retry(&temp, target) {
        Ok(()) => Ok(()),
        Err(e) => {
            // La cible est intacte : on lui rend son attribut et on ne laisse
            // pas traîner le temporaire derrière nous.
            if let Some(perms) = original {
                let _ = fs::set_permissions(target, perms);
            }
            let _ = fs::remove_file(&temp);
            Err(e)
        }
    }
}

/// Nombre de tentatives de renommage, et attente initiale entre deux essais
/// (doublée à chaque fois : 40, 80, 160, 320 ms — soit 600 ms au total).
const RENAME_ATTEMPTS: u32 = 5;
const RENAME_BACKOFF_MS: u64 = 40;

/// Renomme en réessayant brièvement.
///
/// Un remplacement peut échouer pour une raison **passagère** : un indexeur ou
/// un antivirus qui vient d'ouvrir le fichier fraîchement écrit le garde
/// quelques dizaines de millisecondes. Abandonner au premier refus ferait
/// perdre l'édition pour une seconde d'inattention d'un autre logiciel.
///
/// Le réessai ne masque rien : un vrai problème de droits, ou un fichier tenu
/// ouvert durablement, échoue les cinq fois et remonte.
fn rename_with_retry(from: &Path, to: &Path) -> io::Result<()> {
    let mut last = None;

    for attempt in 0..RENAME_ATTEMPTS {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last = Some(e);
                if attempt + 1 < RENAME_ATTEMPTS {
                    let wait = RENAME_BACKOFF_MS << attempt;
                    std::thread::sleep(std::time::Duration::from_millis(wait));
                }
            }
        }
    }

    Err(explain(last.expect("au moins une tentative a eu lieu")))
}

/// Remplace « Accès refusé » par quelque chose sur quoi l'utilisateur peut agir.
///
/// Le message brut du système ne dit pas quoi faire. Or la cause est presque
/// toujours l'une de deux, et la première surprend : **sur un partage réseau,
/// une simple lecture en cours suffit à bloquer le remplacement**, là où un
/// disque local l'accepte sans broncher.
fn explain(error: io::Error) -> io::Error {
    if error.kind() == io::ErrorKind::PermissionDenied {
        return io::Error::new(
            io::ErrorKind::PermissionDenied,
            "le fichier est ouvert par une autre application, ou protégé en \
             écriture. Sur un partage réseau, une lecture en cours suffit à \
             empêcher son remplacement.",
        );
    }
    error
}

/// Réécrit une plage d'octets **sans recopier le fichier**.
///
/// # Le compromis, en clair
/// Le remplacement atomique protège de tout, mais il recopie l'intégralité du
/// fichier : **2 613 ms mesurées pour changer 2,4 Ko de tags dans un FLAC de
/// 34 Mo sur un partage réseau**, contre **113 ms** en écrivant sur place. Un
/// facteur vingt-trois.
///
/// Ce que ça coûte : l'atomicité. Une coupure pendant l'écriture laisserait un
/// bloc de métadonnées incohérent. Mais deux choses limitent la portée du
/// risque, et c'est ce qui rend l'échange acceptable :
///
/// - **L'audio n'est jamais touché.** La partie irremplaçable du fichier reste
///   intacte quoi qu'il arrive ; seuls les tags seraient à refaire.
/// - **La fenêtre est vingt fois plus courte.** Recopier 34 Mo expose bien plus
///   longtemps qu'écrire 2,4 Ko.
///
/// L'appelant ne doit s'en servir que si le nouveau contenu occupe **exactement**
/// la place de l'ancien — sinon il décalerait l'audio. Les conteneurs
/// s'en assurent avant d'appeler, et retombent sur `replace_file_with` sinon.
pub fn write_in_place(target: &Path, offset: u64, data: &[u8]) -> io::Result<()> {
    use std::io::{Seek, SeekFrom};

    let mut file = fs::OpenOptions::new().write(true).open(target)?;
    file.seek(SeekFrom::Start(offset))?;
    file.write_all(data)?;
    file.sync_all()
}

/// Chemin du fichier temporaire, dans le dossier de la cible.
fn temp_path_for(target: &Path) -> io::Result<PathBuf> {
    let parent = target.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "chemin sans dossier parent",
        )
    })?;
    let name = target.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "chemin sans nom de fichier")
    })?;

    let mut temp_name = name.to_os_string();
    temp_name.push(TEMP_SUFFIX);
    Ok(parent.join(temp_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dossier temporaire propre à ce test, supprimé à la fin.
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rustmusic-inject-{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn replaces_the_content_in_place() {
        let dir = temp_dir("replace");
        let file = dir.join("piste.flac");
        fs::write(&file, b"ancien contenu").unwrap();

        replace_file(&file, b"nouveau contenu").unwrap();

        assert_eq!(fs::read(&file).unwrap(), b"nouveau contenu");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn leaves_no_temporary_file_behind() {
        let dir = temp_dir("cleanup");
        let file = dir.join("piste.mp3");
        fs::write(&file, b"x").unwrap();

        replace_file(&file, b"y").unwrap();

        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(TEMP_SUFFIX))
            .collect();
        assert!(leftovers.is_empty(), "un fichier temporaire est resté");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_leftover_temp_file_does_not_block_the_write() {
        let dir = temp_dir("leftover");
        let file = dir.join("piste.dsf");
        fs::write(&file, b"original").unwrap();
        // Simule un plantage lors d'une édition précédente.
        fs::write(dir.join(format!("piste.dsf{TEMP_SUFFIX}")), b"leftover").unwrap();

        replace_file(&file, b"corrected").unwrap();

        assert_eq!(fs::read(&file).unwrap(), b"corrected");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replaces_a_read_only_file() {
        // Windows refuse de renommer par-dessus un fichier en lecture seule.
        // Beaucoup de bibliothèques restaurées depuis une sauvegarde, ou
        // copiées depuis un CD, portent cet attribut : sans traitement, elles
        // seraient toutes inéditables.
        let dir = temp_dir("readonly");
        let file = dir.join("piste.flac");
        fs::write(&file, b"ancien").unwrap();

        let mut perms = fs::metadata(&file).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&file, perms).unwrap();

        replace_file(&file, b"nouveau").unwrap();

        assert_eq!(fs::read(&file).unwrap(), b"nouveau");
        assert!(
            fs::metadata(&file).unwrap().permissions().readonly(),
            "l'attribut lecture seule a été perdu"
        );

        // Nettoyage : un dossier plein de fichiers en lecture seule résiste.
        let mut perms = fs::metadata(&file).unwrap().permissions();
        perms.set_readonly(false);
        let _ = fs::set_permissions(&file, perms);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_replacement_leaves_the_original_untouched() {
        // La garantie centrale du module : si le remplacement échoue, on doit
        // retrouver le fichier d'avant, pas un état intermédiaire.
        let dir = temp_dir("failed");
        let file = dir.join("piste.flac");
        fs::write(&file, b"contenu d'origine").unwrap();

        let result = replace_file_with(&file, |_| {
            Err(io::Error::other("panne simulée pendant l'écriture"))
        });

        assert!(result.is_err());
        assert_eq!(fs::read(&file).unwrap(), b"contenu d'origine");
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(TEMP_SUFFIX))
            .collect();
        assert!(leftovers.is_empty(), "un fichier temporaire est resté");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_temp_file_sits_next_to_the_target() {
        // Garantit l'atomicité du renommage : un temporaire placé dans le
        // dossier temp du système pourrait être sur un autre volume.
        let target = Path::new("/musique/album/piste.flac");
        let temp = temp_path_for(target).unwrap();
        assert_eq!(temp.parent(), target.parent());
    }
}
