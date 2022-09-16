//! Functions to manage and interact with the mod archives
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::error::{ModManagerError, Result};
use super::injection::InjectionType;

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

/// Intermediate Struct used to open a mod archive and read its contents
pub struct ModArchive {
    pub name: String,
    pub extension: String,
    pub path: PathBuf,
}

impl ModArchive {
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = dunce::canonicalize(path)?;

        if !path.exists() {
            return Err(ModManagerError::InvalidArchive(
                InvalidArchive::PathNotExisting,
            ));
        }

        if !path.is_file() {
            return Err(ModManagerError::InvalidArchive(InvalidArchive::PathNotFile));
        }

        // Get archive name and extension
        let archive_extension = path
            .extension()
            .ok_or(ModManagerError::InvalidArchive(InvalidArchive::NoExtension))?
            .to_str()
            .ok_or(ModManagerError::InvalidArchive(
                InvalidArchive::InvalidExtension,
            ))?
            .to_owned();
        let archive_name = path
            .file_name()
            .unwrap_or_else(|| OsStr::new("Unknown"))
            .to_str()
            .unwrap_or("Unknown")
            .split('.')
            .next()
            .unwrap_or("Unknown")
            .to_owned();

        Ok(Self {
            name: archive_name,
            extension: archive_extension,
            path,
        })
    }

    pub async fn get_modinfo(&self) -> Result<Option<ModInfo>> {
        let path_clone = self.path.clone();
        tauri::async_runtime::spawn_blocking::<_, Result<Option<ModInfo>>>(|| {
            let archive_file = File::open(path_clone)?;

            let files = compress_tools::list_archive_files(&archive_file)?;

            // Try to find modinfo.json file
            for path in files.iter() {
                if !path.eq_ignore_ascii_case("modinfo.json") {
                    continue;
                }

                log::debug!("Found modinfo file {}, trying to deserialize...", path);

                let mut mod_info_file = vec![];
                compress_tools::uncompress_archive_file(&archive_file, &mut mod_info_file, path)?;

                return Ok(Some(
                    serde_json::from_slice::<ModInfo>(&mod_info_file)
                        .map_err(|e| ModManagerError::InvalidModInfo { msg: e.to_string() })?,
                ));
            }

            Ok(None)
        })
        .await?
    }

    /// Gets all paths of the files and dirs contained in an archive. Automatically removes modinfo.json path if it exists as it does not need to be injected into the game.
    pub async fn get_archive_dirs_and_files(&self) -> Result<(Vec<String>, Vec<String>)> {
        let path = self.path.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let archive = File::open(path)?;
            let mut files = compress_tools::list_archive_files(archive)?;
            let mut dirs = vec![];

            let mut idx = 0;

            while idx < files.len() {
                let path = &files[idx];

                if path.ends_with('/') {
                    dirs.push(files.remove(idx));
                    continue;
                }

                if path.contains("modinfo.json") {
                    files.remove(idx);
                    continue;
                }

                idx += 1;
            }

            Ok((files, dirs))
        })
        .await?
    }
}

/// The modinfo.json file definition which contains additional information of a mod used by this app
#[derive(Debug, Serialize, Deserialize)]
pub struct ModInfo {
    pub name: String,
    pub author: String,
    pub version: String,
    pub info: String,
    /// Type of mod injection that is required to install this mod
    pub injection: InjectionType,
}
