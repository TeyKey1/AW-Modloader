//! Configuration functionality
use std::fs::{self, OpenOptions};
use std::path::PathBuf;

use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

lazy_static! {
    pub static ref APP_SAVE_PATH: PathBuf = {
        let project_paths = ProjectDirs::from("com", "TeyKey1", "AW Modloader")
            .expect("Failed to determine a valid savepath for the application data");

        let save_path = project_paths.config_dir().to_owned();

        if !save_path.exists() {
            log::info!("App save directory does not exist yet, creating...");

            fs::create_dir_all(&save_path).expect(
                "Failed to create the configuration and storage directory of the application",
            );
        }

        save_path
    };
}

const CONFIG_FILE_NAME: &str = "config.json";
const AW_GAME_BASE_FOLDER_NAME: &str = "Armored Warfare MyCom";

/// Errors that can happen when working with the app configuration
#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum ConfigError {
    DeSerialization { msg: String },
    Io { msg: String },
    GameLanguageNotSupported,
    InvalidGamePath(InvalidGamePath),
    TauriError { msg: String },
}

/// The various ways a provided AW game path can be invalid
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "invalidGamePath")]
pub enum InvalidGamePath {
    NotExisting,
    /// Provided path points to a file instead of a directory
    NotADirectory,
    InvalidPath,
    /// If the folder name does not match with [`AW_GAME_BASE_FOLDER_NAME`]
    InvalidFolderName,
    /// If the localization folder cannot be found inside the provided game folder
    LocalizationNotFound,
}

impl From<serde_json::Error> for ConfigError {
    fn from(error: serde_json::Error) -> Self {
        match error.classify() {
            serde_json::error::Category::Io => Self::Io {
                msg: error.to_string(),
            },
            _ => Self::DeSerialization {
                msg: error.to_string(),
            },
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            msg: format!("Failed to interact with app configuration IO: {}", error),
        }
    }
}

impl From<tauri::Error> for ConfigError {
    fn from(error: tauri::Error) -> Self {
        Self::TauriError {
            msg: error.to_string(),
        }
    }
}

type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ModloaderConfig {
    app_language: Option<String>,
    game_path: Option<String>,
    game_language: Option<String>,
    dark_theme: bool,
}

impl ModloaderConfig {
    /// Loads an existing configuration file or creates a new default one, if not existing
    pub async fn load_config() -> Result<Self> {
        let config_path = APP_SAVE_PATH.join(CONFIG_FILE_NAME);

        tauri::async_runtime::spawn_blocking::<_, Result<Self>>(|| {
            if !config_path.exists() {
                log::info!("Could not find existing config.json file, creating a new one.");
                let new_config = Self::default();

                let config_file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(config_path)?;

                serde_json::to_writer_pretty(&config_file, &new_config)?;

                Ok(new_config)
            } else {
                let config_file = OpenOptions::new().read(true).open(config_path)?;

                let config = serde_json::from_reader::<_, Self>(&config_file)?;

                Ok(config)
            }
        })
        .await?
    }

    pub async fn set_advanced_config(
        &mut self,
        game_lang: String,
        game_path_string: String,
    ) -> Result<()> {
        self.set_game_path(game_path_string).await?;

        self.set_game_language(game_lang).await?;

        Ok(())
    }

    pub fn get_app_language(&self) -> Option<String> {
        self.app_language.clone()
    }

    pub async fn set_app_language(&mut self, lang: Option<String>) -> Result<()> {
        self.app_language = lang;
        self.save_config().await?;

        Ok(())
    }

    pub fn get_game_path(&self) -> Option<PathBuf> {
        self.game_path.as_ref().map(PathBuf::from)
    }

    pub async fn set_game_path(&mut self, game_path_string: String) -> Result<()> {
        // Check if provided game path is a valid AW game
        let game_path = dunce::canonicalize(&game_path_string)?;

        if !game_path.is_dir() {
            return Err(ConfigError::InvalidGamePath(InvalidGamePath::NotADirectory));
        }

        if !game_path
            .file_name()
            .ok_or(ConfigError::InvalidGamePath(InvalidGamePath::InvalidPath))?
            .eq_ignore_ascii_case(AW_GAME_BASE_FOLDER_NAME)
        {
            return Err(ConfigError::InvalidGamePath(
                InvalidGamePath::InvalidFolderName,
            ));
        }

        if !game_path.read_dir()?.any(|entry| match entry {
            Ok(entry) => entry.file_name().eq_ignore_ascii_case("localization"),
            Err(_) => false,
        }) {
            return Err(ConfigError::InvalidGamePath(
                InvalidGamePath::LocalizationNotFound,
            ));
        }

        self.game_path = Some(game_path_string);
        self.save_config().await?;

        Ok(())
    }

    pub fn get_game_language(&self) -> Option<String> {
        self.game_language.clone()
    }

    pub async fn set_game_language(&mut self, game_lang: String) -> Result<()> {
        self.game_language = Some(match game_lang.as_str() {
            "en" => Ok("English".to_owned()),
            "de" => Ok("German".to_owned()),
            "fr" => Ok("French".to_owned()),
            "pl" => Ok("Polish".to_owned()),
            "ru" => Ok("Russian".to_owned()),
            _ => Err(ConfigError::GameLanguageNotSupported),
        }?);

        self.save_config().await?;

        Ok(())
    }

    pub fn get_dark_theme(&self) -> bool {
        self.dark_theme
    }

    pub async fn set_dark_theme(&mut self, dark: bool) -> Result<()> {
        self.dark_theme = dark;
        self.save_config().await?;

        Ok(())
    }

    async fn save_config(&self) -> Result<()> {
        let config_path = APP_SAVE_PATH.join(CONFIG_FILE_NAME);

        let config_clone = self.clone();
        tauri::async_runtime::spawn_blocking::<_, Result<()>>(move || {
            let mut config_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(config_path)?;

            serde_json::to_writer_pretty(&mut config_file, &config_clone)?;

            Ok(())
        })
        .await?
    }
}
