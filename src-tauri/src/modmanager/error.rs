//! Modmanager error types

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::archive::InvalidArchive;
use crate::config::ConfigError;

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
