fn main() {
    lastfm_key();
    tauri_build::build()
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
