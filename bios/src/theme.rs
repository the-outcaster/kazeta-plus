// Make sure you have the right imports and make your structs public
use crate::audio::SoundEffects;
use crate::config::get_user_data_dir;
use macroquad::prelude::*; // for load_string
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

// This needs to be public so main.rs can see it
#[derive(Deserialize, Debug, Clone)]
pub struct ThemeConfigFile {
    pub menu_position: Option<String>,
    pub font_color: Option<String>,
    pub cursor_color: Option<String>,
    pub background_scroll_speed: Option<String>,
    pub color_shift_speed: Option<String>,
    pub sfx_pack: Option<String>,
    pub bgm_track: Option<String>,
    pub logo_selection: Option<String>,
    pub background_selection: Option<String>,
    pub font_selection: Option<String>,
}

// This also needs to be public
#[derive(Clone)]
pub struct Theme {
    pub name: String,
    pub sounds: SoundEffects,
    // Add other pre-loaded assets here if you want
    // pub background: Texture2D,
    pub config: ThemeConfigFile, // Store the parsed config
}

// LOAD CUSTOM THEMES
pub async fn load_all_themes() -> HashMap<String, Theme> {
    let mut themes = HashMap::new();

    // .await is needed here because SoundEffects::load is async
    let default_sfx = SoundEffects::load("Default").await;

    let themes_dir = match get_user_data_dir() {
        Some(dir) => dir.join("themes"),
        None => return themes,
    };

    // Use synchronous std::fs to list directories. It's simple and efficient here.
    if let Ok(entries) = fs::read_dir(themes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let theme_name = path.file_name().unwrap().to_string_lossy().into_owned();
                let toml_path = path.join("theme.toml");

                if toml_path.exists() {
                    // Use macroquad's async load_string to read file contents
                    if let Ok(content) = load_string(&toml_path.to_string_lossy()).await {
                        if let Ok(config) = toml::from_str::<ThemeConfigFile>(&content) {
                            let sounds = match &config.sfx_pack {
                                // .await is needed here too
                                Some(pack_name) => SoundEffects::load(pack_name).await,
                                None => default_sfx.clone(),
                            };

                            let loaded_theme = Theme {
                                name: theme_name.clone(),
                                sounds,
                                config,
                            };

                            println!("[INFO] Loaded theme '{}'", theme_name);
                            themes.insert(theme_name, loaded_theme);
                        }
                    }
                }
            }
        }
    }
    themes
}
