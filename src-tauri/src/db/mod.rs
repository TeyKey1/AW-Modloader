use std::path::Path;

use sled::{Config, Db, Mode, Tree};

mod db;
mod keys;

pub use db::BincodeDb;
pub use db::BincodeTransactional;
pub use keys::Key;

/// The App Database
///
/// This is a thin wrapper over [`sled::Db`] to implement the traits defined in the [`db`] module.
pub struct AppDb {
    inner: Db,
}

impl AppDb {
    /// Opens the App database. If no database exists a new one is created.
    ///
    /// # Panics
    /// If opening the DB fails, which is a fatal error that cannot be recovered and needs human intervention
    pub fn open(path: &Path, flush_interval_ms: u64, cache_capacity_bytes: u64) -> Self {
        let db = Config::default()
            .path(path)
            .cache_capacity(cache_capacity_bytes)
            .mode(Mode::HighThroughput)
            .flush_every_ms(Some(flush_interval_ms))
            .open()
            .expect("Failed to open the database");

        Self { inner: db }
    }

    /// Open a specific tree on the database
    ///
    /// # Panics
    /// If opening the specific tree fails, which is a fatal error that cannot be recovered and needs human intervention
    pub fn open_tree(&self, name: &str) -> Tree {
        self.inner
            .open_tree(name)
            .unwrap_or_else(|err| panic!("Failed to open DB tree '{}'. Caused by: {}", name, err))
    }

    pub fn get_inner(&self) -> &Db {
        &self.inner
    }
}

impl Drop for AppDb {
    /// Flushes the remaining buffers and makes them persistent on the disk before dropping the DB.
    ///
    /// # Panics
    /// If the flushing of the buffers fails.
    fn drop(&mut self) {
        log::debug!("Flushing DB to disk prior to shutdown");
        self.inner
            .flush()
            .expect("Failed to flush the DB during drop");
    }
}
