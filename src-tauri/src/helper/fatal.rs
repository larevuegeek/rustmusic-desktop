//! Rapporter une panne fatale survenue **avant** que la fenêtre existe.
//!
//! # Le problème que ça résout
//! Une application graphique n'a pas de console sous Windows. Un `panic!` ou un
//! `expect` avant la création de la fenêtre part donc dans le vide : pas de
//! message, pas de trace dans le journal — les panics ne passent pas par le
//! logger —, et l'utilisateur constate seulement que « l'application ne se
//! lance plus ».
//!
//! C'est exactement ce qui est arrivé avec une base migrée par une version plus
//! récente que le binaire installé : un diagnostic d'une ligne, invisible
//! pendant des jours.
//!
//! Trois sorties valent mieux qu'une : le journal pour l'après-coup, la sortie
//! d'erreur pour qui lance depuis un terminal, et une boîte de dialogue native
//! pour l'utilisateur qui a double-cliqué.

/// Affiche l'erreur par tous les canaux disponibles, puis quitte.
pub fn fatal(title: &str, message: &str) -> ! {
    log::error!("{title} — {message}");
    eprintln!("\n{title}\n{message}\n");
    show_dialog(title, message);
    std::process::exit(1);
}

#[cfg(target_os = "windows")]
fn show_dialog(title: &str, message: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    // Appel direct, comme pour `GetDriveTypeW` ailleurs dans le projet : ça
    // évite d'ajouter une dépendance entière pour une seule fonction.
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(hwnd: isize, text: *const u16, caption: *const u16, utype: u32) -> i32;
    }

    /// `MB_ICONERROR | MB_SETFOREGROUND`. `MB_OK` vaut zéro : l'écrire
    /// n'ajouterait rien qu'un avertissement du linter.
    const FLAGS: u32 = 0x0000_0010 | 0x0001_0000;

    let wide = |s: &str| -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    };

    let text = wide(message);
    let caption = wide(title);
    unsafe {
        MessageBoxW(0, text.as_ptr(), caption.as_ptr(), FLAGS);
    }
}

#[cfg(not(target_os = "windows"))]
fn show_dialog(_title: &str, _message: &str) {
    // Sous macOS et Linux, l'application est lancée depuis un terminal ou un
    // gestionnaire de bureau qui conserve la sortie d'erreur : le message
    // écrit plus haut est déjà visible.
}

/// Traduit un échec de migration en phrase qui dit quoi faire.
///
/// On filtre sur la **variante**, pas sur le texte de l'erreur : `Display` rend
/// une phrase en anglais qui peut changer d'une version de sqlx à l'autre, et
/// un diagnostic qui se dégrade en silence à la montée de version ne vaut rien.
pub fn explain_migration(error: &sqlx::Error) -> String {
    use sqlx::migrate::MigrateError;

    let detail = error.to_string();

    if let sqlx::Error::Migrate(inner) = error {
        match inner.as_ref() {
            // Le cas courant, et le plus déroutant : la base porte une
            // migration que ce binaire ne connaît pas. Autrement dit une
            // version plus récente est passée par là — un build de
            // développement le plus souvent, puisque les deux partagent le
            // même fichier.
            MigrateError::VersionMissing(version) => {
                return format!(
                    "La base de données a été mise à jour par une version plus récente                      de RustMusic.

                     Cette version-ci ne connaît pas toutes ses modifications et refuse                      de l'ouvrir, pour ne pas l'abîmer.

                     Installez la dernière version de RustMusic.

                     Détail : migration {version} absente de ce binaire."
                )
            }
            // Une migration déjà appliquée a été réécrite depuis : son
            // empreinte ne correspond plus.
            MigrateError::VersionMismatch(version) => {
                return format!(
                    "Une migration déjà appliquée a été modifiée depuis.

                     La base et cette version de RustMusic ne décrivent plus le même                      schéma.

                     Détail : migration {version}, empreinte différente."
                )
            }
            _ => {}
        }
    }

    format!("Impossible de préparer la base de données.

Détail : {detail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrate_error(inner: sqlx::migrate::MigrateError) -> sqlx::Error {
        sqlx::Error::from(inner)
    }

    #[test]
    fn une_base_plus_recente_dit_quoi_faire() {
        let error = migrate_error(sqlx::migrate::MigrateError::VersionMissing(202608170001));
        let message = explain_migration(&error);

        // Le diagnostic, puis le geste : sans le second, l'utilisateur sait
        // seulement que quelque chose ne va pas.
        assert!(message.contains("version plus récente"));
        assert!(message.contains("Installez la dernière version"));
        // Et le détail brut, pour le rapport de bogue.
        assert!(message.contains("202608170001"));
    }

    #[test]
    fn une_migration_modifiee_est_distinguee() {
        let error = migrate_error(sqlx::migrate::MigrateError::VersionMismatch(202608170001));
        assert!(explain_migration(&error).contains("modifiée depuis"));
    }

    #[test]
    fn les_autres_pannes_restent_lisibles() {
        let error = sqlx::Error::PoolTimedOut;
        let message = explain_migration(&error);
        assert!(message.contains("Impossible de préparer la base"));
        // Jamais de message vide : le détail est toujours joint.
        assert!(message.len() > 40);
    }
}
