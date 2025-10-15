use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, error::Error};
use crate::MenuPosition;

/// Returns the path to the user's data directory for Kazeta+.
/// This is a public helper function for other modules to use.
pub fn get_user_data_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|path| path.join(".local/share/kazeta-plus"))
}

/// Gets the full path to the kazeta.toml configuration file.
fn get_config_path() -> Result<PathBuf, Box<dyn Error>> {
    let mut config_path = get_user_data_dir().ok_or("Could not find user's data directory.")?;
    fs::create_dir_all(&config_path)?; // Create the directory if it doesn't exist
    config_path.push("config.toml");
    Ok(config_path)
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub resolution: String,
    pub fullscreen: bool,
    pub show_splash_screen: bool,
    pub timezone: String,
    pub wifi: bool,
    pub bgm_volume: f32,
    pub sfx_volume: f32,
    pub audio_output: String,
    pub theme: String,
    pub menu_position: MenuPosition,
    pub font_color: String,
    pub cursor_color: String,
    pub background_scroll_speed: String,
    pub color_shift_speed: String,
    pub bgm_track: Option<String>,
    pub sfx_pack: String,
    pub logo_selection: String,
    pub background_selection: String,
    pub font_selection: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            resolution: "640x360".to_string(),
            fullscreen: false,
            show_splash_screen: true,
            timezone: "UTC".to_string(),
            wifi: true,
            bgm_volume: 0.7,
            sfx_volume: 0.7,
            audio_output: "Auto".to_string(),
            theme: "Default".to_string(),
            menu_position: MenuPosition::Center,
            font_color: "WHITE".to_string(),
            cursor_color: "WHITE".to_string(),
            background_scroll_speed: "NORMAL".to_string(),
            color_shift_speed: "NORMAL".to_string(),
            bgm_track: None,
            sfx_pack: "Default".to_string(),
            logo_selection: "Kazeta+ (Default)".to_string(),
            background_selection: "Default".to_string(),
            font_selection: "Default".to_string(),
        }
    }
}

impl Config {
    /// Loads the configuration from kazeta.toml, or returns a default if it fails.
    pub fn load() -> Self {
        if let Ok(config_path) = get_config_path() {
            if let Ok(content) = fs::read_to_string(config_path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// Saves the current configuration to kazeta.toml.
    pub fn save(&self) {
        if let Ok(config_path) = get_config_path() {
            if let Ok(toml_string) = toml::to_string_pretty(self) {
                let _ = fs::write(config_path, toml_string);
            }
        }
    }

    pub fn delete() -> std::io::Result<()> {
        if let Ok(config_path) = get_config_path() {
            if config_path.exists() {
                println!("[Info] Deleting config file at: {}", config_path.display());
                std::fs::remove_file(config_path)?;
            }
        }
        Ok(())
    }
}
