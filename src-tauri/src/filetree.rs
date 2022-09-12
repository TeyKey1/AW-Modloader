//! The file tree is used to keep track on which files have been modified/created by which mods.
//! This allows to check whether mods collide with each other. And allows efficient file cleanup/restore on mod deactivation/deletion
use std::fs::File;
use std::vec;

use serde::{Deserialize, Serialize};
use sled::transaction::{TransactionError, UnabortableTransactionError};

use crate::db::{AppDb, BincodeTransactional, Key};
use crate::modmanager::{ModManagerError, Result};

const DB_FILE_TREE_NAME: &str = "filetree";

/// A single Tree file entry which contains the UID of the mod that owns this file
#[derive(Serialize, Deserialize)]
struct TreeFileEntry(u64);

pub struct FileTreeManager;

impl FileTreeManager {
    /// Checks if the provided list of file paths does not conflict with any existing file tree entries.
    /// Returns None if no conflicts are found and a vector containing the conflicting mod uid's with the corresponding conflicting path.
    pub fn get_conflicts(
        file_paths: &Vec<String>,
        db: &AppDb,
    ) -> Result<Option<Vec<(u64, String)>>> {
        let tree = db.open_tree(DB_FILE_TREE_NAME);

        let conflicts = tree
            .transaction::<_, _, UnabortableTransactionError>(|transaction| {
                let mut conflicts = vec![];

                for path in file_paths {
                    let file_tree_entry = transaction.b_get(&Key::<TreeFileEntry>::new(path))?;

                    if let Some(uid) = file_tree_entry {
                        conflicts.push((uid.0, path.to_owned()))
                    }
                }

                Ok(conflicts)
            })
            .map_err(|err| match err {
                TransactionError::Abort(err) => ModManagerError::Db {
                    msg: err.to_string(),
                },
                TransactionError::Storage(err) => ModManagerError::Db {
                    msg: err.to_string(),
                },
            })?;

        if conflicts.is_empty() {
            Ok(None)
        } else {
            Ok(Some(conflicts))
        }
    }

    /// Insert the list of file paths into the file tree with the corresponding mod uid
    pub fn insert_files(uid: u64, file_paths: &Vec<String>, db: &AppDb) -> Result<()> {
        let tree = db.open_tree(DB_FILE_TREE_NAME);

        tree.transaction::<_, _, UnabortableTransactionError>(|transaction| {
            for file_path in file_paths {
                transaction.b_insert(&Key::<TreeFileEntry>::new(file_path), &TreeFileEntry(uid))?;
            }

            Ok(())
        })
        .map_err(|err| match err {
            TransactionError::Abort(err) => ModManagerError::Db {
                msg: err.to_string(),
            },
            TransactionError::Storage(err) => ModManagerError::Db {
                msg: err.to_string(),
            },
        })?;

        Ok(())
    }

    /// Get all files in the tree owned by the provided mod uid
    pub fn get_files(uid: u64, db: &AppDb) -> Result<Vec<String>> {
        let tree = db.open_tree(DB_FILE_TREE_NAME);

        let mut found_paths = vec![];

        for entry in tree.iter() {
            let (key, value) = entry?;

            let tree_uid = bincode::deserialize::<TreeFileEntry>(&value)?;

            if tree_uid.0 != uid {
                continue;
            }

            let path =
                String::from_utf8(key.to_vec()).map_err(|e| ModManagerError::DeSerialization {
                    msg: format!("Failed to deserialize file tree key: {}", e),
                })?;

            found_paths.push(path);
        }

        return Ok(found_paths);
    }

    /// Remove all provided file paths from the tree
    pub fn remove_files(paths: &Vec<String>, db: &AppDb) -> Result<()> {
        let tree = db.open_tree(DB_FILE_TREE_NAME);

        tree.transaction::<_, _, UnabortableTransactionError>(|transaction| {
            for file_path in paths {
                transaction.b_remove(&Key::<TreeFileEntry>::new(file_path))?;
            }

            Ok(())
        })
        .map_err(|err| match err {
            TransactionError::Abort(err) => ModManagerError::Db {
                msg: err.to_string(),
            },
            TransactionError::Storage(err) => ModManagerError::Db {
                msg: err.to_string(),
            },
        })?;

        Ok(())
    }
}

/// Gets all paths of the files and dirs contained in an archive. Automatically removes modinfo.json path if it exists as it does not need to be injected into the game.
pub fn get_archive_dirs_and_files(archive: &File) -> Result<(Vec<String>, Vec<String>)> {
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
}
