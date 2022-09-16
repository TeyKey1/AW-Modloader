use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize};
use tauri::Window;
use tokio::sync::oneshot;

use crate::config::APP_SAVE_PATH;
use crate::DB;

mod archive;
pub mod error;
mod filetree;
mod injection;
mod registry;

use archive::ModArchive;
use error::{ModManagerError, Result};
use filetree::FileTreeManager;
use injection::InjectionType;

pub use registry::{Mod, DB_MOD_TREE_NAME, MOD_REGISTRY_PATH};

#[derive(Debug, Serialize, Deserialize)]
struct OverwriteEventPayload {
    overwrite: bool,
}

pub struct ModManager;

impl ModManager {
    /// Creates the ModManager and performs the necessary initialization
    pub fn new() -> Result<Self> {
        fs::create_dir_all(APP_SAVE_PATH.join(MOD_REGISTRY_PATH))?;

        Ok(Self)
    }

    pub fn get_initial_mod_data(&self) -> Result<HashMap<u64, Mod>> {
        let tree = DB.open_tree(DB_MOD_TREE_NAME);

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
    pub async fn add_mod(&self, archive_path: &str, window: Window) -> Result<()> {
        let mod_archive = ModArchive::open(archive_path).await?;

        log::debug!(
            "Adding mod with archive name {} to registry",
            mod_archive.name
        );

        let mod_info = mod_archive.get_modinfo().await?;

        let mut modification;

        if mod_info.is_none() {
            modification = Mod::new(
                &mod_archive.name,
                InjectionType::Localization,
                &mod_archive.extension,
            )?;
        } else {
            modification = Mod::from_mod_info(mod_info.unwrap(), &mod_archive.extension)?;
        }

        // Check if mod already exists in registry
        if let Some(existing_modification) = modification.is_already_existing().await? {
            if modification.is_newer_version(&existing_modification) {
                // ask user for overwrite permission
                let (oneshot_sender, oneshot_receiver) = oneshot::channel();
                window.once("add-mod-overwrite", |event| {
                    let overwrite: OverwriteEventPayload =
                        serde_json::from_str(event.payload().expect(
                            "Received None as event payload but expected payload to contain value",
                        ))
                        .expect("Expected OverwriteEventPayload but failed to deserialize");

                    oneshot_sender.send(overwrite.overwrite).expect(
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
            } else {
                return Err(ModManagerError::ModVersionMismatch {
                    mismatch: (
                        modification.version.unwrap().to_string(),
                        existing_modification.version.unwrap().to_string(),
                    ),
                });
            }

            // remove active mod files of old mod version
            log::debug!(
                "Removing all active mod files of the old mod version to install the new one"
            );

            if existing_modification.is_active() {
                existing_modification
                    .injection_type()
                    .eject_mod(&existing_modification)
                    .await?;
            }

            modification.uid = existing_modification.uid;
        }

        modification.save_in_registry(mod_archive).await?;

        modification.insert_into_db().await?;

        Ok(())
    }

    /// Deletes a mod from the registry and deactivates it prior to removal if necessary
    pub async fn delete_mod(&self, uid: u64) -> Result<()> {
        let modification = Mod::get_from_db(uid)?;

        if modification.is_active() {
            // Remove all active mod files injected into the game
            modification
                .injection_type()
                .eject_mod(&modification)
                .await?;
        }

        log::info!("Removing mod {} from registry", modification.name);

        modification.delete().await?;

        Ok(())
    }

    /// Activates a registered mod and injects it into the game
    pub async fn activate_mod(&self, uid: u64) -> Result<()> {
        let modification = Mod::get_from_db(uid)?;

        if modification.is_active() {
            return Err(ModManagerError::ModAlreadyActive);
        }

        let mod_archive = ModArchive::open(APP_SAVE_PATH.join(MOD_REGISTRY_PATH).join(format!(
            "{}.{}",
            modification.uid, modification.archive_file_extension
        )))
        .await?;

        let (mod_file_list, mod_dir_list) = mod_archive.get_archive_dirs_and_files().await?;

        // Check if the mod conflicts with any currently activated mods
        let conflicts = FileTreeManager::get_conflicts(&mod_file_list)?;

        if let Some(conflicts) = conflicts {
            let mut conflict_list = vec![];

            for (uid, conflicting_path) in conflicts {
                let conflicting_mod = Mod::get_from_db(uid)?;

                conflict_list.push((conflicting_mod.name, conflicting_path));
            }

            return Err(ModManagerError::ModConflict {
                conflict: conflict_list,
            });
        }

        modification
            .injection_type()
            .inject_mod(modification, mod_archive, mod_dir_list, mod_file_list)
            .await
    }

    pub async fn deactivate_mod(&self, uid: u64) -> Result<()> {
        let mut modification = Mod::get_from_db(uid)?;

        if !modification.is_active() {
            return Err(ModManagerError::ModAlreadyDeactivated);
        }

        modification
            .injection_type()
            .eject_mod(&modification)
            .await?;

        modification.deactivate()
    }

    /// Deactivates all active mods
    pub async fn deactivate_all_mods(&self) -> Result<()> {
        let tree = DB.open_tree(DB_MOD_TREE_NAME);

        for modification in tree.iter() {
            let (_, modification) = modification?;

            let modification = bincode::deserialize::<Mod>(&modification)?;

            if modification.is_active() {
                self.deactivate_mod(modification.uid).await?;
            }
        }

        Ok(())
    }
}

/// Event which is sent to the Frontend if any Mod in the database changes or is deleted
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ModChangedEvent {
    Delete(u64),
    InsertUpdate(u64, Mod),
}
