fn main() {
    lastfm_key();
    main_thread_stack();
    windows_resources();
    // Dossiers de `icons_dir!` (src/app/icons.rs) : Cargo suit les fichiers lus, pas les
    // fichiers ajoutés ; surveiller les dossiers recompile quand une icône arrive.
    println!("cargo:rerun-if-changed=ui/assets/icons");
    println!("cargo:rerun-if-changed=ui/assets/icons-thick");
}

/// Ressources de l'exécutable Windows, posées par Tauri avant : l'icône (Explorateur,
/// raccourcis, épinglage) et les informations de version (nom « RustMusic » dans le
/// Gestionnaire des tâches et les propriétés du fichier). La fenêtre et la zone de
/// notification ont leurs propres icônes (`WindowOptions::icon`, `TrayIcon`).
fn windows_resources() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let icon = manifest.join("icons").join("icon.ico").display().to_string().replace('\\', "\\\\");
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let numbers: Vec<u16> = version.split(['.', '-']).take(3).map(|n| n.parse().unwrap_or(0)).collect();
    let (major, minor, patch) = (numbers[0], numbers.get(1).copied().unwrap_or(0), numbers.get(2).copied().unwrap_or(0));
    let rc = format!(
        r#"1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "CompanyName", "LaRevueGeeK"
      VALUE "FileDescription", "RustMusic"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "rustmusic"
      VALUE "LegalCopyright", "GPL-3.0-only"
      VALUE "OriginalFilename", "rustmusic.exe"
      VALUE "ProductName", "RustMusic"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    );
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("rustmusic.rc");
    std::fs::write(&out, rc).expect("ressource Windows écrite");
    // Sans compilateur de ressources (SDK absent), l'exécutable n'a pas d'icône : on le dit
    // sans faire échouer la compilation.
    if let Err(error) = embed_resource::compile(&out, embed_resource::NONE).manifest_optional() {
        println!("cargo:warning=icône de l'exécutable non intégrée : {error}");
    }
}

/// Pile du thread principal (fenêtre et vues) : 8 Mio comme sous macOS et Linux, au lieu du
/// 1 Mio de Windows. En debug, sans optimisation, les vues générées par `ui!` ont des cadres
/// de pile énormes et débordaient dès le lancement (`STATUS_STACK_OVERFLOW`). Réservé, pas
/// engagé : la mémoire n'est prise que si elle sert.
fn main_thread_stack() {
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os == "windows" {
        if env == "msvc" {
            println!("cargo:rustc-link-arg-bins=/STACK:8388608");
        } else {
            println!("cargo:rustc-link-arg-bins=-Wl,--stack,8388608");
        }
    }
}

/// Clé d'API Last.fm de RustMusic : les variables d'environnement si elles existent, sinon le fichier
/// `keys/lastfm.local` (ignoré par Git). Sans l'un ni l'autre, l'appli demande à chacun sa propre clé.
fn lastfm_key() {
    const NAMES: [&str; 2] = ["RUSTMUSIC_LASTFM_API_KEY", "RUSTMUSIC_LASTFM_API_SECRET"];
    const FILE: &str = "keys/lastfm.local";
    for name in NAMES {
        println!("cargo:rerun-if-env-changed={name}");
    }
    // Le dossier, toujours présent : surveiller un fichier absent ferait tout recompiler à chaque fois.
    println!("cargo:rerun-if-changed=keys");
    let Ok(text) = std::fs::read_to_string(FILE) else { return };
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim(), value.trim().trim_matches('"'));
        let from_env = std::env::var(key).is_ok_and(|v| !v.trim().is_empty());
        if NAMES.contains(&key) && !value.is_empty() && !from_env {
            println!("cargo:rustc-env={key}={value}");
        }
    }
}
