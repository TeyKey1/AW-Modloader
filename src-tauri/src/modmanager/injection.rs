//! Implementation of various mod injection types to inject the mod into the game
use std::fs::{self, File};

use serde::{Deserialize, Serialize};

use crate::config::ModloaderConfig;

use super::archive::ModArchive;
use super::error::{ModManagerError, Result};
use super::filetree::FileTreeManager;
use super::registry::Mod;

/// The different injection techniques used to install mods in AW.
///
/// Currently only the Localization injection is implemented. The system can be enhanced to support any new mod injection techniques.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InjectionType {
    /// Injection of the mod via the localization folder
    #[serde(alias = "localization")]
    Localization,
}

impl From<&str> for InjectionType {
    fn from(string: &str) -> Self {
        match string.to_lowercase().as_str() {
            "localization" => InjectionType::Localization,
            _ => panic!("Failed to transform string into InjectionType enum"),
        }
    }
}

impl InjectionType {
    pub async fn inject_mod(
        &self,
        mut modification: Mod,
        mod_archive: ModArchive,
        mod_dir_list: Vec<String>,
        mod_file_list: Vec<String>,
    ) -> Result<()> {
        match self {
            InjectionType::Localization => {
                let config = ModloaderConfig::load_config().await?;

                tauri::async_runtime::spawn_blocking(move || {
                    // Create required directories if they do not yet exist
                    for mod_dir in mod_dir_list.iter() {
                        fs::create_dir_all(
                            config
                                .get_game_path()
                                .ok_or(ModManagerError::AppNotInitialized)?
                                .join("localization")
                                .join(
                                    config
                                        .get_game_language()
                                        .ok_or(ModManagerError::AppNotInitialized)?,
                                )
                                .join(mod_dir),
                        )?;
                    }

                    let game_localization_path = config
                        .get_game_path()
                        .expect("Called activate_mod when game path has not been defined yet")
                        .join("localization")
                        .join(config.get_game_language().expect(
                            "Called activate_mod when game language has not been defined yet",
                        ));

                    // Uncompress all required mod files and place them into the appropriate AW folder
                    let mod_archive = File::open(mod_archive.path)?;

                    for file in mod_file_list.iter() {
                        let new_file = File::create(game_localization_path.join(file))?;
                        compress_tools::uncompress_archive_file(&mod_archive, &new_file, file)?;
                    }

                    // Add newly added files to the file tree to detect future mod collisions
                    FileTreeManager::insert_files(modification.uid, &mod_file_list)?;

                    // Set added mod as active
                    modification.set_active()
                })
                .await?
            }
        }
    }

    /// Remove mod files from the game and deactivate it
    pub async fn eject_mod(&self, modification: &Mod) -> Result<()> {
        match self {
            InjectionType::Localization => {
                let config = ModloaderConfig::load_config().await?;

                let modification = modification.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    // get mod file paths from tree and remove them
                    let file_paths = FileTreeManager::get_files(modification.uid)?;

                    for path in file_paths.iter() {
                        if let Err(err) = fs::remove_file(
                            config
                                .get_game_path()
                                .ok_or(ModManagerError::AppNotInitialized)?
                                .join("localization")
                                .join(
                                    config
                                        .get_game_language()
                                        .ok_or(ModManagerError::AppNotInitialized)?,
                                )
                                .join(path),
                        ) {
                            match err.kind() {
                                std::io::ErrorKind::NotFound => (),
                                _ => return Err(err.into()),
                            }
                        }
                    }

                    // update tree
                    FileTreeManager::remove_files(&file_paths)
                })
                .await?
            }
        }
    }
}
