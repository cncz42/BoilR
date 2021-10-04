use failure::*;
use std::path::{Path, PathBuf};

use crate::{gog::gog_config::GogConfig, platform::Platform};

use super::{
    gog_game::{GogGame, GogShortcut},
    GogSettings,
};

pub struct GogPlatform {
    pub settings: GogSettings,
}

impl Platform<GogShortcut, GogErrors> for GogPlatform {
    fn enabled(&self) -> bool {
        self.settings.enabled
    }

    fn name(&self) -> &str {
        "Gog"
    }

    fn get_shortcuts(&self) -> Result<Vec<GogShortcut>, GogErrors> {
        let gog_location = self
            .settings
            .location
            .as_ref()
            .map(|location| Path::new(&location).to_path_buf())
            .unwrap_or_else(default_location);
        if !gog_location.exists() {
            return Err(GogErrors::PathNotFound { path: gog_location });
        }
        let config_path = gog_location.join("config.json");
        if !config_path.exists() {
            return Err(GogErrors::ConfigFileNotFound { path: config_path });
        }
        let install_locations = get_install_locations(config_path)?;
        dbg!(&install_locations);
        let mut game_folders = vec![];
        for install_location in install_locations {
            let path = Path::new(&install_location);
            if path.exists() {
                let dirs = path.read_dir();
                if let Ok(dirs) = dirs {
                    for dir in dirs {
                        if let Ok(dir) = dir {
                            if let Ok(file_type) = dir.file_type() {
                                if file_type.is_dir() {
                                    game_folders.push(dir.path());
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut games = vec![];
        for game_folder in &game_folders {
            if let Ok(files) = game_folder.read_dir() {
                for file in files {
                    if let Ok(file) = file {
                        if let Some(file_name) = file.file_name().to_str() {
                            if file_name.starts_with("goggame-") {
                                if let Some(extension) = file.path().extension() {
                                    if let Some(extension) = extension.to_str() {
                                        if extension == "info" {
                                            // Finally we know we can parse this as a game
                                            if let Ok(content) =
                                                std::fs::read_to_string(file.path())
                                            {
                                                if let Ok(gog_game) =
                                                    serde_json::from_str::<GogGame>(&content)
                                                {
                                                    games.push((gog_game, game_folder));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut shortcuts = vec![];
        for (game, game_folder) in games {
            if let Some(folder_path) = game_folder.to_str() {
                if let Some(tasks) = &game.play_tasks {
                    if let Some(primary_task) = tasks.iter().find(|t| {
                        t.is_primary.unwrap_or_default()
                            && t.task_type == "FileTask"
                            && t.category.as_ref().unwrap_or(&String::from("")) == "game"
                    }) {
                        if let Some(task_path) = &primary_task.path {
                            let full_path = game_folder.join(&task_path);
                            if let Some(full_path) = full_path.to_str() {
                                let folder_path = folder_path.to_string();

                                let working_dir = match &primary_task.working_dir {
                                    Some(working_dir) => game_folder
                                        .join(working_dir)
                                        .to_str()
                                        .unwrap_or(folder_path.as_str())
                                        .to_string(),
                                    None => folder_path.to_string(),
                                };
                                let shortcut = GogShortcut {
                                    name: game.name,
                                    game_folder: folder_path,
                                    working_dir,
                                    game_id: game.game_id,
                                    path: full_path.to_string(),
                                };
                                shortcuts.push(shortcut);
                            }
                        }
                    }
                }
            }
        }

        Ok(shortcuts)
    }
}

fn get_install_locations(path: PathBuf) -> Result<Vec<String>, GogErrors> {
    let data_res =
        std::fs::read_to_string(&path).map_err(|e| GogErrors::ConfigFileCouldNotBeRead {
            path: path.clone(),
            error: format!("{}", e),
        })?;
    let config: GogConfig =
        serde_json::from_str(&data_res).map_err(|e| GogErrors::ConfigFileCouldNotBeRead {
            path,
            error: format!("{}", e),
        })?;
    Ok(config.installation_paths)
}

pub fn default_location() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let key = "PROGRAMDATA";
        let program_data = std::env::var(key).expect("Expected a APPDATA variable to be defined");
        Path::new(&program_data).join("GOG.com").join("Galaxy")
    }
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").expect("Expected a home variable to be defined");
        Path::new(&home).join("GOG.com").join("Galaxy")
    }
}

#[derive(Debug, Fail)]
pub enum GogErrors {
    #[fail(
        display = "Gog path: {:?} could not be found. Try to specify a different path for Gog.",
        path
    )]
    PathNotFound { path: PathBuf },

    #[fail(display = "Gog config file not found at path: {:?}", path)]
    ConfigFileNotFound { path: PathBuf },

    #[fail(
        display = "Gog config file at path: {:?} could not be red {}",
        path, error
    )]
    ConfigFileCouldNotBeRead { path: PathBuf, error: String },
}