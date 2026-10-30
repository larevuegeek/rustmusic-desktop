//! Commandes de lancement, à la foobar2000 : `RustMusic /play`, `--next`, `-pause`…
//!
//! Au premier lancement, elles attendent que l'interface les réclame (`take_launch_commands`) ;
//! si l'appli tourne déjà, la seconde instance les lui transmet (`cli-command`) puis se ferme.

use std::sync::Mutex;

/// Nom accepté → commande transmise à l'interface. Le reste (chemins, `-psn_…` de macOS) est ignoré.
const COMMANDS: &[(&str, &str)] = &[
    ("play", "play"),
    ("pause", "pause"),
    ("playpause", "toggle"),
    ("toggle", "toggle"),
    ("stop", "stop"),
    ("next", "next"),
    ("prev", "previous"),
    ("previous", "previous"),
    ("show", "show"),
    ("exit", "quit"),
    ("quit", "quit"),
];

static PENDING: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// Commandes reconnues dans `args`, dans l'ordre et sans doublon.
pub fn parse<S: AsRef<str>>(args: impl IntoIterator<Item = S>) -> Vec<&'static str> {
    let mut out = Vec::new();
    for arg in args {
        let arg = arg.as_ref();
        let Some(name) = arg.strip_prefix("--").or_else(|| arg.strip_prefix('-')).or_else(|| arg.strip_prefix('/')) else { continue };
        let name = name.to_ascii_lowercase();
        if let Some((_, cmd)) = COMMANDS.iter().find(|(n, _)| *n == name) {
            if !out.contains(cmd) {
                out.push(*cmd);
            }
        }
    }
    out
}

/// Garde les commandes du lancement, le temps que l'interface soit prête.
pub fn remember_startup() {
    let cmds = parse(std::env::args().skip(1));
    if !cmds.is_empty() {
        log::info!("⌨️  Commandes de lancement : {cmds:?}");
    }
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = cmds;
}

/// Une seule fois : l'interface les exécute à la fin de son démarrage.
#[tauri::command]
pub fn take_launch_commands() -> Vec<String> {
    std::mem::take(&mut *PENDING.lock().unwrap_or_else(|e| e.into_inner()))
        .into_iter()
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn windows_and_unix_styles_are_both_understood() {
        assert_eq!(parse(["/play"]), vec!["play"]);
        assert_eq!(parse(["--next", "-PAUSE", "/PlayPause"]), vec!["next", "pause", "toggle"]);
        assert_eq!(parse(["/prev", "/previous"]), vec!["previous"], "deux alias, une commande");
    }

    #[test]
    fn paths_and_unknown_options_are_ignored() {
        // macOS ajoute `-psn_0_1234` quand l'appli vient du Finder.
        assert!(parse(["C:\\Musique\\a.flac", "-psn_0_1234", "--inconnu", "play"]).is_empty());
        assert_eq!(parse(["/exit"]), vec!["quit"]);
    }
}
