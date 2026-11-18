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
        let config_path = SettingsManager::get_config_file();

        let file = File::create(config_path)?;
        serde_json::to_writer(file, config)?;

        Ok(())
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
