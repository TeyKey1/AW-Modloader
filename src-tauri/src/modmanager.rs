use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, File};

use semver::Version;
use serde::{Deserialize, Serialize};
use tauri::Window;
use tokio::sync::oneshot;
use ts_rs::TS;

use crate::config::{ConfigError, ModloaderConfig, APP_SAVE_PATH};
use crate::db::{AppDb, BincodeDb, Key};
use crate::filetree::{self, FileTreeManager};

pub const DB_MOD_TREE_NAME: &str = "modtree";
const MOD_REGISTRY_PATH: &str = "registry";

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum ModManagerError {
    Io {
        msg: String,
    },
    /// All database errors
    Db {
        msg: String,
    },
    DeSerialization {
        msg: String,
    },
    InvalidArchive(InvalidArchive),
    /// Errors happening during the handling of archives (compression, decompression)
    ArchiveHandling {
        msg: String,
    },
    /// Errors happening while parsing a potential modinfo.json file
    InvalidModInfo {
        msg: String,
    },
    ModNotExisting,
    ModAlreadyActive,
    ModAlreadyDeactivated,
    /// If the mod being added has an older version than the identical mod already existing in the registry
    ModVersionMismatch {
        mismatch: (String, String),
    },
    /// The initial configuration data required for the modloader has not been provided
    AppNotInitialized,
    /// If the mod conflicts with other mods eg. modifies/uses the same files. The returned tuple contains the conflicting mod name and the conflicting file path
    ModConflict {
        conflict: Vec<(String, String)>,
    },
    ConfigError(ConfigError),
    TauriError {
        msg: String,
    },
}

pub type Result<T> = std::result::Result<T, ModManagerError>;

impl From<tauri::Error> for ModManagerError {
    fn from(error: tauri::Error) -> Self {
        Self::TauriError {
            msg: error.to_string(),
        }
    }
}

impl From<std::io::Error> for ModManagerError {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            msg: error.to_string(),
        }
    }
}

impl From<sled::Error> for ModManagerError {
    fn from(error: sled::Error) -> Self {
        match error {
            sled::Error::Io(error) => Self::Io {
                msg: error.to_string(),
            },
            _ => Self::Db {
                msg: error.to_string(),
            },
        }
    }
}

impl From<bincode::Error> for ModManagerError {
    fn from(error: bincode::Error) -> Self {
        match *error {
            bincode::ErrorKind::Io(error) => Self::Io {
                msg: error.to_string(),
            },
            _ => Self::DeSerialization {
                msg: error.to_string(),
            },
        }
    }
}

impl From<compress_tools::Error> for ModManagerError {
    fn from(error: compress_tools::Error) -> Self {
        match error {
            compress_tools::Error::Io(error) => Self::Io {
                msg: error.to_string(),
            },
            _ => Self::ArchiveHandling {
                msg: error.to_string(),
            },
        }
    }
}

impl From<ConfigError> for ModManagerError {
    fn from(error: ConfigError) -> Self {
        Self::ConfigError(error)
    }
}

/// Errors that can happen when handling the mod archives
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "invalidArchive")]
pub enum InvalidArchive {
    /// Provided path to the mod archive does not exist
    PathNotExisting,
    /// Provided path to the mod archive is not a file
    PathNotFile,
    /// Provided file has no extension
    NoExtension,
    /// Invalid file extension
    InvalidExtension,
}

pub struct ModManager;

impl ModManager {
    /// Creates the ModManager and performs the necessary initialization
    pub fn new() -> Result<Self> {
        fs::create_dir_all(APP_SAVE_PATH.join(MOD_REGISTRY_PATH))?;

        Ok(Self)
    }

    pub fn get_initial_mod_data(&self, db: &AppDb) -> Result<HashMap<u64, Mod>> {
        let tree = db.open_tree(DB_MOD_TREE_NAME);

        let mut hashmap = HashMap::new();

        for result in tree.iter() {
            let (key, modification) = result?;

            let key: u64 = String::from_utf8(key.to_vec())
                .map_err(|e| ModManagerError::DeSerialization { msg: e.to_string() })?
                .parse::<u64>()
                .map_err(|e| ModManagerError::DeSerialization { msg: e.to_string() })?;
            let modification: Mod = bincode::deserialize(&modification)?;

            hashmap.insert(key, modification);
        }

        Ok(hashmap)
    }

    /// Add a new mod to the registry
    pub async fn add_mod(&self, archive_path: &str, db: &AppDb, window: Window) -> Result<()> {
        let path = dunce::canonicalize(archive_path)?;

        if !path.exists() {
            return Err(ModManagerError::InvalidArchive(
                InvalidArchive::PathNotExisting,
            ));
        }

        if !path.is_file() {
            return Err(ModManagerError::InvalidArchive(InvalidArchive::PathNotFile));
        }

        let archive_file = File::open(&path)?;

        let archive_extension = path
            .extension()
            .ok_or(ModManagerError::InvalidArchive(InvalidArchive::NoExtension))?
            .to_str()
            .ok_or(ModManagerError::InvalidArchive(
                InvalidArchive::InvalidExtension,
            ))?;
        let archive_name = path
            .file_name()
            .unwrap_or(&OsStr::new("Unknown"))
            .to_str()
            .unwrap_or("Unknown")
            .split('.')
            .next()
            .unwrap_or("Unknown");

        log::debug!("Adding mod with archive name {} to registry", archive_name);

        // Check if archive can be read
        let files = compress_tools::list_archive_files(&archive_file)?;

        let mut mod_info = None;
        // Try to find modinfo.json file
        for path in files.iter() {
            if !path.eq_ignore_ascii_case("modinfo.json") {
                continue;
            }

            log::debug!("Found modinfo file {}, trying to deserialize...", path);

            let mut mod_info_file = vec![];
            compress_tools::uncompress_archive_file(&archive_file, &mut mod_info_file, path)?;

            mod_info = Some(
                serde_json::from_slice::<ModInfo>(&mod_info_file)
                    .map_err(|e| ModManagerError::InvalidModInfo { msg: e.to_string() })?,
            );
            log::debug!("Successfully deserialized modinfo file");
        }

        let mut modification;

        if mod_info.is_none() {
            modification = Mod::new(
                archive_name,
                InjectionType::Localization,
                archive_extension,
                db,
            )?;
        } else {
            modification = Mod::from_mod_info(mod_info.unwrap(), archive_extension, db)?;
        }

        // Check if mod already exists in registry
        let tree = db.open_tree(DB_MOD_TREE_NAME);

        for existing_modification in tree.iter() {
            let (_, value) = existing_modification?;

            let existing_modification = bincode::deserialize::<Mod>(&value)?;

            if existing_modification.name.eq(&modification.name) {
                log::debug!("Mod is already existing in the registry, checking version");

                if existing_modification.version.is_some() && modification.version.is_some() {
                    if existing_modification.version.as_ref().unwrap()
                        >= modification.version.as_ref().unwrap()
                    {
                        return Err(ModManagerError::ModVersionMismatch {
                            mismatch: (
                                modification.version.unwrap().to_string(),
                                existing_modification.version.unwrap().to_string(),
                            ),
                        });
                    }
                } else {
                    // ask user for overwrite permission
                    let (oneshot_sender, oneshot_receiver) = oneshot::channel();
                    window.once("add-mod-overwrite", |event| {
                        println!("event payload:  {:?}", event.payload());
                        let overwrite = serde_json::from_str::<bool>(event.payload().expect(
                            "Received None as event payload but expected payload to contain value",
                        ))
                        .expect("Expected bool but failed to deserialize");

                        oneshot_sender.send(overwrite).expect(
                            "Oneshot receiver has been dropped before sender could send the value",
                        );
                    });

                    window.emit("add-mod-ask-overwrite", &modification.name)?;

                    let overwrite = oneshot_receiver
                        .await
                        .expect("Oneshot sender has been dropped bevore a value could be received");

                    if !overwrite {
                        return Ok(());
                    }
                }

                // remove active mod files of old mod version
                log::debug!(
                    "Removing all active mod files of the old mod version to install the new one"
                );

                if existing_modification.is_active {
                    Self::remove_active_mod_files(existing_modification.uid, db)?;
                }

                modification.uid = existing_modification.uid;

                break;
            }
        }

        // save archive file in registry
        fs::copy(
            &path,
            APP_SAVE_PATH
                .join(MOD_REGISTRY_PATH)
                .join(format!("{}.{}", modification.uid, archive_extension)),
        )?;

        tree.b_insert(&Key::new(&modification.uid.to_string()), &modification)?;

        Ok(())
    }

    /// Deletes a mod from the registry and deactivates it prior to removal if necessary
    pub fn delete_mod(&self, uid: u64, db: &AppDb) -> Result<()> {
        let tree = db.open_tree(DB_MOD_TREE_NAME);

        let removed = tree
            .b_remove(&Key::<Mod>::new(uid.to_string().as_str()))?
            .ok_or(ModManagerError::ModNotExisting)?;

        if removed.is_active {
            // Remove all active mod files injected into the game
            Self::remove_active_mod_files(uid, db)?;
        }

        // Remove the mod from the registry
        match fs::remove_file(APP_SAVE_PATH.join(MOD_REGISTRY_PATH).join(format!(
            "{}.{}",
            removed.uid, removed.archive_file_extension
        ))) {
            Ok(_) => Ok(()),
            Err(e) => match e.kind() {
                // Ignore not found error as we are deleting the mod anyways
                std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(e),
            },
        }?;

        log::info!("Mod {} successfully removed from registry", removed.name);

        Ok(())
    }

    /// Activates a registered mod and injects it into the game
    pub fn activate_mod(&self, uid: u64, db: &AppDb) -> Result<()> {
        let tree = db.open_tree(DB_MOD_TREE_NAME);

        let mut modification = tree
            .b_get(&Key::<Mod>::new(uid.to_string().as_str()))?
            .ok_or(ModManagerError::ModNotExisting)?;

        if modification.is_active {
            return Err(ModManagerError::ModAlreadyActive);
        }

        let mod_archive = File::open(APP_SAVE_PATH.join(MOD_REGISTRY_PATH).join(format!(
            "{}.{}",
            modification.uid, modification.archive_file_extension
        )))?;

        let (mod_file_list, mod_dir_list) = filetree::get_archive_dirs_and_files(&mod_archive)?;

        let conflicts = FileTreeManager::get_conflicts(&mod_file_list, db)?;

        // Check if the mod conflicts with any currently activated mods
        if let Some(conflicts) = conflicts {
            let mut conflict_list = vec![];

            for (uid, conflicting_path) in conflicts {
                let conflicting_mod = tree
                    .b_get(&Key::<Mod>::new(uid.to_string().as_str()))?
                    .ok_or(ModManagerError::ModNotExisting)?;

                conflict_list.push((conflicting_mod.name, conflicting_path));
            }

            return Err(ModManagerError::ModConflict {
                conflict: conflict_list,
            });
        }

        match modification.injection {
            InjectionType::Localization => {
                let config = ModloaderConfig::load_config()?;

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

                let game_localization_path =
                    config
                        .get_game_path()
                        .expect("Called activate_mod when game path has not been defined yet")
                        .join("localization")
                        .join(config.get_game_language().expect(
                            "Called activate_mod when game language has not been defined yet",
                        ));

                // Uncompress all required mod files and place them into the appropriate AW folder
                for file in mod_file_list.iter() {
                    let new_file = File::create(game_localization_path.join(&file))?;
                    compress_tools::uncompress_archive_file(&mod_archive, &new_file, &file)?;
                }

                // Add newly added files to the file tree to detect future mod collisions
                FileTreeManager::insert_files(modification.uid, &mod_file_list, db)?;

                // Set added mod as active
                modification.is_active = true;
                tree.b_insert(&Key::<Mod>::new(uid.to_string().as_str()), &modification)?;

                Ok(())
            }
        }
    }

    pub fn deactivate_mod(&self, uid: u64, db: &AppDb) -> Result<()> {
        let tree = db.open_tree(DB_MOD_TREE_NAME);

        let mut modification = tree
            .b_get(&Key::<Mod>::new(uid.to_string().as_str()))?
            .ok_or(ModManagerError::ModNotExisting)?;

        if !modification.is_active {
            return Err(ModManagerError::ModAlreadyDeactivated);
        }

        Self::remove_active_mod_files(uid, db)?;

        // update mod
        modification.is_active = false;

        tree.b_insert(
            &Key::<Mod>::new(modification.uid.to_string().as_str()),
            &modification,
        )?;

        Ok(())
    }

    /// Deactivates all active mods
    pub fn deactivate_all_mods(&self, db: &AppDb) -> Result<()> {
        let tree = db.open_tree(DB_MOD_TREE_NAME);

        for modification in tree.iter() {
            let (_, modification) = modification?;

            let modification = bincode::deserialize::<Mod>(&modification)?;

            if modification.is_active {
                self.deactivate_mod(modification.uid, db)?;
            }
        }

        Ok(())
    }

    /// Removes all actively injected mod files of the provided mod uid from the game
    fn remove_active_mod_files(uid: u64, db: &AppDb) -> Result<()> {
        // get mod file paths from tree and remove them
        let file_paths = FileTreeManager::get_files(uid, db)?;

        let config = ModloaderConfig::load_config()?;

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
        FileTreeManager::remove_files(&file_paths, db)?;

        Ok(())
    }
}

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

/// Event which is sent to the Frontend if any Mod in the database changes or is deleted
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ModChangedEvent {
    Delete(u64),
    InsertUpdate(u64, Mod),
}

/// A single AW mod
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mod {
    name: String,
    /// Unique identifier for the mod, used for internal functions
    uid: u64,
    /// As multiple archive file extensions have to be supported the file extension of each mod is stored
    archive_file_extension: String,
    author: Option<String>,
    version: Option<Version>,
    info: Option<String>,
    /// Type of mod injection that is required to install this mod
    injection: InjectionType,
    /// Whether the mod is currently active and installed in the game or not
    is_active: bool,
}

impl Mod {
    /// Create a [`Mod`] from a [`ModInfo`] struct
    pub fn from_mod_info(
        mod_info: ModInfo,
        archive_file_extension: &str,
        db: &AppDb,
    ) -> Result<Self> {
        Ok(Self {
            name: mod_info.name,
            uid: db.get_inner().generate_id()?,
            archive_file_extension: archive_file_extension.to_owned(),
            author: Some(mod_info.author),
            version: Some(
                Version::parse(&mod_info.version)
                    .map_err(|e| ModManagerError::InvalidModInfo { msg: e.to_string() })?,
            ),
            info: Some(mod_info.info),
            injection: mod_info.injection,
            is_active: false,
        })
    }

    /// Create a [`Mod`] from name and injection type
    ///
    /// This is used if no modinfo.json file is found. Otherwise use [`from_mod_info()`]
    pub fn new(
        name: &str,
        injection_type: InjectionType,
        archive_file_extension: &str,
        db: &AppDb,
    ) -> Result<Self> {
        Ok(Self {
            name: name.to_owned(),
            uid: db.get_inner().generate_id()?,
            archive_file_extension: archive_file_extension.to_owned(),
            author: None,
            version: None,
            info: None,
            injection: injection_type,
            is_active: false,
        })
    }
}

/// The modinfo.json file definition which contains additional information of a mod used by this app
#[derive(Debug, Serialize, Deserialize)]
pub struct ModInfo {
    name: String,
    author: String,
    version: String,
    info: String,
    /// Type of mod injection that is required to install this mod
    injection: InjectionType,
}
