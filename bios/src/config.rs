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
    config_path.push("kazeta.toml");
    Ok(config_path)
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub resolution: String,
    pub fullscreen: bool,
    pub show_splash_screen: bool,
    pub timezone: String,
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

/* OLD STRUCTURE
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, error::Error};
use crate::MenuPosition;

pub fn get_config_path() -> Result<PathBuf, Box<dyn Error>> {
    let mut config_path = dirs::config_dir()
    .ok_or("Could not find user's config directory.")?;
    config_path.push("kazeta-plus");

    // Create the directory if it doesn't exist
    fs::create_dir_all(&config_path)?;

    config_path.push("kazeta.toml");
    Ok(config_path)
}

// This struct defines the structure of your config.json file
#[derive(Serialize, Deserialize)]
pub struct Config {
    // video options
    pub resolution: String,
    pub fullscreen: bool,
    pub show_splash_screen: bool, // SPLASH SCREEN
    pub timezone: String,

    // audio options
    pub bgm_volume: f32,
    pub sfx_volume: f32,
    pub audio_output: String,

    // GUI customization options
    pub theme: String,
    pub menu_position: MenuPosition, // MENU POSITION
    pub font_color: String,
    pub cursor_color: String,
    pub background_scroll_speed: String,
    pub color_shift_speed: String,

    // custom asset options
    pub bgm_track: Option<String>,
    pub sfx_pack: String,
    pub logo_selection: String,
    pub background_selection: String,
    pub font_selection: String,
}

// subset of main config
#[derive(Serialize, Deserialize, Debug)]
pub struct ThemeConfig {
    pub menu_position: MenuPosition,
    pub font_color: String,
    pub cursor_color: String,
    pub background_scroll_speed: String,
    pub color_shift_speed: String,
    pub bgm_track: String,
    pub logo_selection: String,
    pub background_selection: String,
    pub font_selection: String,
}

// This provides a default state for the config
impl Default for Config {
    fn default() -> Self {
        Self {
            // video settings
            resolution: "640x360".to_string(),
            fullscreen: false,
            show_splash_screen: true, // Splash screen is ON by default
            timezone: "UTC".to_string(),

            // audio settings
            bgm_volume: 0.7,
            sfx_volume: 0.7,
            audio_output: "Auto".to_string(),

            // GUI settings
            theme: "Default".to_string(),
            menu_position: MenuPosition::Center, // MENU POSITION
            font_color: "WHITE".to_string(),
            cursor_color: "WHITE".to_string(),
            background_scroll_speed: "NORMAL".to_string(),
            color_shift_speed: "NORMAL".to_string(),

            // custom assets
            bgm_track: None,
            sfx_pack: "Default".to_string(),
            logo_selection: "Kazeta+ (Default)".to_string(),
            background_selection: "Default".to_string(),
            font_selection: "Default".to_string(),
        }
    }
}

impl Config {
    /// Loads the configuration from the user's config directory.
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = get_config_path()?;
        let content = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

// CONFIG.JSON SETTINGS
pub fn load_config() -> Config {
    if let Some(path) = get_user_data_dir() {
        let config_path = path.join("config.json");
        if let Ok(file_contents) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str(&file_contents) {
                return config;
            }
        }
    }
    // If anything fails, create and save a default config
    let default_config = Config::default();
    save_config(&default_config);
    default_config
}

pub fn save_config(config: &Config) {
    if let Some(path) = get_user_data_dir() {
        // Create the directory if it doesn't exist
        if fs::create_dir_all(&path).is_ok() {
            let config_path = path.join("config.json");
            if let Ok(json) = serde_json::to_string_pretty(config) {
                let _ = fs::write(&config_path, json);
            }
        }
    }
}

pub fn delete_config_file() -> std::io::Result<()> {
    if let Some(mut path) = home::home_dir() {
        path.push(".local/share/kazeta-plus/config.json");
        if path.exists() {
            println!("[Info] Deleting config file at: {}", path.display());
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

/// Returns the path to the user's data directory for Kazeta+.
pub fn get_user_data_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|path| path.join(".local/share/kazeta-plus"))
}
*/
