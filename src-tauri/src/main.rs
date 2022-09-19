#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use std::collections::HashMap;
use std::fs::File;

use lazy_static::lazy_static;
use modmanager::{ModChangedEvent, ModManager};
use simplelog::{
    ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode, WriteLogger,
};
use tauri::{Manager, RunEvent, State, Window};

mod config;
mod db;
mod error;
mod modmanager;

use config::{ConfigError, ModloaderConfig};
use db::AppDb;
use error::Result;
use modmanager::Mod;

const DB_PATH: &str = "db";
const DB_FLUSH_INTERVAL: u64 = 500;
const DB_CACHE_CAPACITY: u64 = 10_485_760;

lazy_static! {
    /// The database of the application
    static ref DB: AppDb = {
        AppDb::open(
            &config::APP_SAVE_PATH.join(DB_PATH),
            DB_FLUSH_INTERVAL,
            DB_CACHE_CAPACITY,
        )
    };
}

fn main() {
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            set_advanced_config,
            config_is_initialized,
            configure_dark_mode,
            get_dark_mode,
            get_app_language,
            set_app_language,
            get_advanced_config,
            add_new_mod,
            get_initial_mod_data,
            delete_mod,
            activate_mod,
            deactivate_mod
        ])
        .manage(
            ModManager::new()
                .map_err(|e| log::error!("Failed to initialize ModManager: {:?}", e))
                .unwrap(),
        )
        .setup(|app| {
            CombinedLogger::init(vec![
                TermLogger::new(
                    LevelFilter::Info,
                    Config::default(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                ),
                WriteLogger::new(
                    LevelFilter::Debug,
                    Config::default(),
                    File::create(config::APP_SAVE_PATH.join("modloader.log")).unwrap(),
                ),
            ])
            .expect("Failed to create logger");

            let main_window = app.get_window("main").unwrap();

            tauri::async_runtime::spawn(async move {
                let mod_tree = DB.open_tree(modmanager::DB_MOD_TREE_NAME);

                let mut mod_tree_subscriber = mod_tree.watch_prefix(vec![]);

                while let Some(event) = (&mut mod_tree_subscriber).await {
                    let mod_changed_event = match event {
                        sled::Event::Insert { key, value } => {
                            let key: u64 = String::from_utf8(key.to_vec())
                                .unwrap()
                                .parse()
                                .expect("Failed to parse database key to u64");
                            let modification: Mod = bincode::deserialize(&value)
                                .expect("Failed to deserialize Mod struct from db data");

                            ModChangedEvent::InsertUpdate(key, modification)
                        }
                        sled::Event::Remove { key } => {
                            let key: u64 = String::from_utf8(key.to_vec())
                                .unwrap()
                                .parse()
                                .expect("Failed to parse database key to u64");

                            ModChangedEvent::Delete(key)
                        }
                    };

                    main_window
                        .emit("mod-tree-data-changed", mod_changed_event)
                        .expect("Failed to send event to frontend");
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_, event| {
        if let RunEvent::Exit = event {
            DB.flush()
        }
    });
}

// Configuration related commands
/// Check if the config contains all required configuration parameters and is initialized
#[tauri::command]
async fn config_is_initialized() -> Result<bool> {
    let config = ModloaderConfig::load_config().await?;

    Ok(config.get_game_path().is_some() && config.get_game_language().is_some())
}

/// Set the advanced configuration of the app (game language and game path)
#[tauri::command]
async fn set_advanced_config(
    game_lang: String,
    game_path: String,
    mod_manager: State<'_, ModManager>,
) -> Result<()> {
    let mut config = ModloaderConfig::load_config().await?;

    let current_game_lang = config.get_game_language();
    let current_game_path = config.get_game_path();

    let game_lang_cmp = match game_lang.as_str() {
        "en" => Ok("English".to_owned()),
        "de" => Ok("German".to_owned()),
        "fr" => Ok("French".to_owned()),
        "pl" => Ok("Polish".to_owned()),
        "ru" => Ok("Russian".to_owned()),
        _ => Err(ConfigError::GameLanguageNotSupported),
    }?;

    if current_game_lang.is_some()
        && current_game_path.is_some()
        && current_game_lang.unwrap() == game_lang_cmp
        && current_game_path.unwrap().to_string_lossy() == game_path
    {
        // Nothing changed in the configuration
        return Ok(());
    }

    // Deactivate all active mods as the config change requires a change of the mod installation folder inside the game installation
    mod_manager.deactivate_all_mods().await?;

    config.set_advanced_config(game_lang, game_path).await?;

    Ok(())
}

#[tauri::command]
async fn configure_dark_mode(dark: bool) -> Result<bool> {
    let mut config = ModloaderConfig::load_config().await?;

    config.set_dark_theme(dark).await?;

    Ok(dark)
}

#[tauri::command]
async fn get_dark_mode() -> Result<bool> {
    let config = ModloaderConfig::load_config().await?;

    Ok(config.get_dark_theme())
}

#[tauri::command]
async fn get_app_language() -> Result<Option<String>> {
    let config = ModloaderConfig::load_config().await?;

    Ok(config.get_app_language())
}

#[tauri::command]
async fn set_app_language(lang: String) -> Result<()> {
    let mut config = ModloaderConfig::load_config().await?;

    config.set_app_language(Some(lang)).await?;

    Ok(())
}

#[tauri::command]
async fn get_advanced_config() -> Result<(Option<String>, Option<String>)> {
    let config = ModloaderConfig::load_config().await?;

    Ok((
        config.get_game_language(),
        config
            .get_game_path()
            .map(|path| path.to_string_lossy().to_string()),
    ))
}

// Mod related commands
#[tauri::command]
fn get_initial_mod_data(mod_manager: State<'_, ModManager>) -> Result<HashMap<u64, Mod>> {
    let data = mod_manager.get_initial_mod_data()?;

    Ok(data)
}

#[tauri::command]
async fn add_new_mod(
    mod_manager: State<'_, ModManager>,
    window: Window,
    archive_path: String,
) -> Result<()> {
    mod_manager.add_mod(&archive_path, window).await?;

    Ok(())
}

#[tauri::command]
async fn delete_mod(mod_manager: State<'_, ModManager>, uid: u64) -> Result<()> {
    mod_manager.delete_mod(uid).await?;

    Ok(())
}

#[tauri::command]
async fn activate_mod(mod_manager: State<'_, ModManager>, uid: u64) -> Result<()> {
    mod_manager.activate_mod(uid).await?;

    Ok(())
}

#[tauri::command]
async fn deactivate_mod(mod_manager: State<'_, ModManager>, uid: u64) -> Result<()> {
    mod_manager.deactivate_mod(uid).await?;

    Ok(())
}
