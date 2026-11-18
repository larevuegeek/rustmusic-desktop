use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{env, fs, io};
use std::{fs::File, path::PathBuf};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub volume_initial: u8,
    pub device_default: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            volume_initial: 50,
            device_default: None,
        }
    }
}

pub struct SettingsManager;

impl SettingsManager {
    pub fn get_config_file() -> PathBuf {
        let home_dir = match env::var_os("HOME") {
            // Pour les systèmes Unix-like
            Some(path) => PathBuf::from(path),
            None => match env::var_os("USERPROFILE") {
                // Pour Windows
                Some(path) => PathBuf::from(path),
                None => PathBuf::new(), // Retourner un chemin vide en cas d'erreur
            },
        };

        let mut config_path = home_dir;
        config_path.push(".rust-music");

        //Si le dossier n'existe pas on le créé
        let exist_dir: bool = Path::new(&config_path).is_dir();
        if !exist_dir {
            let _ = fs::create_dir(&config_path);
        }

        //On ajoute le fichier de configuration
        config_path.push("config.json");

        config_path
    }

    pub fn save_config(config: &Config) -> io::Result<()> {
        write_config(&SettingsManager::get_config_file(), config)
    }

    /// Config enregistrée, ou les valeurs par défaut si le fichier manque ou ne
    /// se lit pas. Sans ce repli, rien ne créait jamais le fichier : chaque
    /// sauvegarde part d'un chargement, qui échouait. La sortie choisie
    /// retombait alors sur « défaut » à chaque lancement, et le DoP, qui a
    /// besoin de la carte ALSA, cédait sa place au DSD2PCM sans rien dire.
    pub fn load_config() -> io::Result<Config> {
        let config_path = SettingsManager::get_config_file();

        let file = match File::open(&config_path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(e) => return Err(e),
        };
        match serde_json::from_reader(file) {
            Ok(config) => Ok(config),
            Err(e) => {
                log::warn!("Config illisible ({}) : {e} — valeurs par défaut", config_path.display());
                Ok(Config::default())
            }
        }
    }
}

/// Écrit la config dans un fichier voisin, le force sur le disque, puis le renomme : un
/// arrêt brutal (plantage, coupure) laisse l'ancienne config ou la nouvelle, jamais un
/// fichier tronqué que la lecture suivante remplacerait par les valeurs par défaut.
fn write_config(path: &Path, config: &Config) -> io::Result<()> {
    let temporary = path.with_extension("json.tmp");
    let content = serde_json::to_vec(config)?;
    {
        let mut file = File::create(&temporary)?;
        io::Write::write_all(&mut file, &content)?;
        file.sync_all()?;
    }
    fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::{write_config, Config};

    #[test]
    fn saving_replaces_the_file_and_leaves_no_temporary() {
        let dir = std::env::temp_dir().join(format!("rustmusic-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        write_config(&path, &Config { volume_initial: 13, device_default: Some("Sortie".into()) }).unwrap();
        write_config(&path, &Config { volume_initial: 42, device_default: None }).unwrap();
        let read: Config = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!((read.volume_initial, read.device_default), (42, None));
        assert!(!dir.join("config.json.tmp").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
