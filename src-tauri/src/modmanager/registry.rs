//! The mod registry
use std::fs;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::config::APP_SAVE_PATH;
use crate::db::{BincodeDb, Key};
use crate::DB;

use super::archive::{ModArchive, ModInfo};
use super::error::{ModManagerError, Result};
use super::injection::InjectionType;

pub const DB_MOD_TREE_NAME: &str = "modtree";
pub const MOD_REGISTRY_PATH: &str = "registry";

/// A single AW mod
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mod {
    pub name: String,
    /// Unique identifier for the mod, used for internal functions
    pub uid: u64,
    /// As multiple archive file extensions have to be supported the file extension of each mod is stored
    pub archive_file_extension: String,
    author: Option<String>,
    pub version: Option<Version>,
    info: Option<String>,
    /// Type of mod injection that is required to install this mod
    injection: InjectionType,
    /// Whether the mod is currently active and installed in the game or not
    is_active: bool,
}

impl Mod {
    /// Create a [`Mod`] from a [`ModInfo`] struct
    pub fn from_mod_info(mod_info: ModInfo, archive_file_extension: &str) -> Result<Self> {
        Ok(Self {
            name: mod_info.name,
            uid: DB.get_inner().generate_id()?,
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
    ) -> Result<Self> {
        Ok(Self {
            name: name.to_owned(),
            uid: DB.get_inner().generate_id()?,
            archive_file_extension: archive_file_extension.to_owned(),
            author: None,
            version: None,
            info: None,
            injection: injection_type,
            is_active: false,
        })
    }

    pub fn get_from_db(uid: u64) -> Result<Self> {
        let tree = DB.open_tree(DB_MOD_TREE_NAME);

        tree.b_get(&Key::<Mod>::new(uid.to_string().as_str()))?
            .ok_or(ModManagerError::ModNotExisting)
    }

    // If the mod is currently active in the game
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn injection_type(&self) -> InjectionType {
        self.injection.clone()
    }

    // If the mod already exists in the registry the Option contains the existing entry
    pub async fn is_already_existing(&self) -> Result<Option<Self>> {
        let mod_name = self.name.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let tree = DB.open_tree(DB_MOD_TREE_NAME);

            for existing_modification in tree.iter() {
                let (_, value) = existing_modification?;

                let existing_modification = bincode::deserialize::<Mod>(&value)?;

                if existing_modification.name.eq(&mod_name) {
                    return Ok(Some(existing_modification));
                }
            }

            Ok(None)
        })
        .await?
    }

    /// If the mod version of the current mod is newer than other
    ///
    /// If no version information is available the function assumes that self is newer
    pub fn is_newer_version(&self, other: &Self) -> bool {
        if other.version.is_some() && self.version.is_some() {
            return other.version.as_ref().unwrap() < self.version.as_ref().unwrap();
        }

        true
    }

    /// Insert the mod into the database
    pub async fn insert_into_db(self) -> Result<()> {
        tauri::async_runtime::spawn_blocking(move || {
            let tree = DB.open_tree(DB_MOD_TREE_NAME);

            tree.b_insert(&Key::new(&self.uid.to_string()), &self)
        })
        .await??;

        Ok(())
    }

    /// Save the mod's archive file in the registry folder
    pub async fn save_in_registry(&self, mod_info: ModArchive) -> Result<()> {
        let uid = self.uid;
        let extension = self.archive_file_extension.clone();
        tauri::async_runtime::spawn_blocking(move || {
            fs::copy(
                &mod_info.path,
                APP_SAVE_PATH
                    .join(MOD_REGISTRY_PATH)
                    .join(format!("{}.{}", uid, extension)),
            )?;

            Ok(())
        })
        .await?
    }

    /// Delete the mod from the registry folder and DB
    ///
    /// # Caution
    /// This does not check if the mod is still active. Make sure to check if the mod is active prior to deletion to avoid any mod files cluttering the game folder.
    pub async fn delete(self) -> Result<()> {
        tauri::async_runtime::spawn_blocking(move || {
            let tree = DB.open_tree(DB_MOD_TREE_NAME);

            tree.b_remove(&Key::<Mod>::new(&self.uid.to_string()))?;

            // Remove the mod from the registry
            match fs::remove_file(
                APP_SAVE_PATH
                    .join(MOD_REGISTRY_PATH)
                    .join(format!("{}.{}", self.uid, self.archive_file_extension)),
            ) {
                Ok(_) => Ok(()),
                Err(e) => match e.kind() {
                    // Ignore not found error as we are deleting the mod anyways
                    std::io::ErrorKind::NotFound => Ok(()),
                    _ => Err(e),
                },
            }
            .map_err(|e| e.into())
        })
        .await?
    }

    pub fn set_active(&mut self) -> Result<()> {
        let tree = DB.open_tree(DB_MOD_TREE_NAME);

        self.is_active = true;

        tree.b_insert(&Key::<Mod>::new(&self.uid.to_string()), self)?;

        Ok(())
    }

    pub fn deactivate(&mut self) -> Result<()> {
        let tree = DB.open_tree(DB_MOD_TREE_NAME);

        self.is_active = false;

        tree.b_insert(&Key::<Mod>::new(&self.uid.to_string()), self)?;

        Ok(())
    }
}
