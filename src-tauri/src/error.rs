use serde::{Deserialize, Serialize};
use sled::Error as SledError;
use ts_rs::TS;

use crate::config::ConfigError;
use crate::modmanager::error::ModManagerError;

/// The global error type of this application.
///
/// It is special as in that it can be serialized/deserialized to be sent to the frontend and does not implement the std Error trait.
/// The rationale behind this is that errors occurring in the backend (this code) are almost exclusively handled by the frontend.
/// Thus there needs to be a unified error type to send those errors to the frontend and handle them accordingly.
///
/// # Error Handling
/// The error handling principle of this app is simple. Generally panics are avoided as much as possible even for errors which are deemed unrecoverable. Panics are only used to detect obvious development bugs.
/// Unrecoverable errors are instead sent to the Frontend containing the reason of the error. The frontend then displays the Error message and closes the application. Recoverable errors are also sent to the Frontend and handled appropriately.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum AppError {
    // Unrecoverable error that leads to the termination of the app
    Unrecoverable { msg: String },
    // Recoverable error that has to be handled by the frontend
    Recoverable(RecoverableAppError),
}

pub type Result<T> = std::result::Result<T, AppError>;

impl From<RecoverableAppError> for AppError {
    fn from(error: RecoverableAppError) -> Self {
        Self::Recoverable(error)
    }
}

impl From<ConfigError> for AppError {
    fn from(error: ConfigError) -> Self {
        match error {
            ConfigError::DeSerialization { msg } => Self::Unrecoverable { msg },
            ConfigError::Io { msg } => Self::Unrecoverable { msg },
            ConfigError::GameLanguageNotSupported => Self::Recoverable(error.into()),
            ConfigError::InvalidGamePath(_) => Self::Recoverable(error.into()),
            ConfigError::TauriError { msg } => Self::Unrecoverable { msg },
        }
    }
}

impl From<ModManagerError> for AppError {
    fn from(error: ModManagerError) -> Self {
        match error {
            ModManagerError::Io { msg } => Self::Unrecoverable { msg },
            ModManagerError::Db { msg } => Self::Unrecoverable { msg },
            ModManagerError::DeSerialization { msg } => Self::Unrecoverable { msg },
            ModManagerError::InvalidArchive(_) => Self::Recoverable(error.into()),
            ModManagerError::ArchiveHandling { .. } => Self::Recoverable(error.into()),
            ModManagerError::InvalidModInfo { .. } => Self::Recoverable(error.into()),
            ModManagerError::ModNotExisting => Self::Unrecoverable {
                msg: String::from("Mod not existing, this is likely a bug."),
            },
            ModManagerError::ModAlreadyActive => Self::Unrecoverable {
                msg: String::from("Mod already active, this is likely a bug."),
            },
            ModManagerError::ModConflict { .. } => Self::Recoverable(error.into()),
            ModManagerError::ConfigError(error) => error.into(),
            ModManagerError::ModAlreadyDeactivated => Self::Unrecoverable {
                msg: String::from("Mod already deactivated, this is likely a bug."),
            },
            ModManagerError::AppNotInitialized => Self::Unrecoverable {
                msg: String::from("Tried to perform an action which requires the modloader config to be initilized when it was not initialized yet, this is likely a bug."),
            },
            ModManagerError::ModVersionMismatch { .. } => Self::Recoverable(error.into()),
            ModManagerError::TauriError { msg } => Self::Unrecoverable { msg },
        }
    }
}

/// Sled errors are almost always deemed unrecoverable as they are usually caused by configuration or hardware issues
impl From<SledError> for AppError {
    fn from(error: SledError) -> Self {
        Self::Unrecoverable {
            msg: error.to_string(),
        }
    }
}

/// Contains all recoverable errors that can occur in this application
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum RecoverableAppError {
    ConfigError(ConfigError),
    ModManagerError(ModManagerError),
}

impl From<ConfigError> for RecoverableAppError {
    fn from(error: ConfigError) -> Self {
        Self::ConfigError(error)
    }
}

impl From<ModManagerError> for RecoverableAppError {
    fn from(error: ModManagerError) -> Self {
        Self::ModManagerError(error)
    }
}
