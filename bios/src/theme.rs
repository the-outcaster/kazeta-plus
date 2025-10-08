// Make sure you have the right imports and make your structs public
use crate::audio::SoundEffects;
use crate::config::get_user_data_dir;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::fs; // Use tokio's fs module!

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
    // It's efficient to load the default sounds once and clone them
    // for any theme that doesn't specify a custom pack.
    let default_sfx = SoundEffects::load("Default").await;

    // Get the path to the user's themes directory
    let themes_dir = match get_user_data_dir() {
        Some(dir) => dir.join("themes"),
        None => return themes, // Or handle error appropriately
    };

    if let Ok(mut entries) = fs::read_dir(themes_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                let theme_name = path.file_name().unwrap().to_string_lossy().into_owned();
                let toml_path = path.join("theme.toml");

                if toml_path.exists() {
                    // Read and parse the theme.toml file
                    if let Ok(content) = fs::read_to_string(toml_path).await {
                        if let Ok(config) = toml::from_str::<ThemeConfigFile>(&content) {

                            // Load the specified sound pack, or clone the default
                            let sounds = match &config.sfx_pack {
                                Some(pack_name) => SoundEffects::load(pack_name).await,
                                None => default_sfx.clone(),
                            };

                            // Here, you would also load your other assets like fonts and images
                            // let background = load_texture(...).await.unwrap();

                            let loaded_theme = Theme {
                                name: theme_name.clone(),
                                sounds,
                                config,
                                // background,
                                // ...etc
                            };

                            println!("Loaded theme '{}'", theme_name);
                            themes.insert(theme_name, loaded_theme);
                        }
                    }
                }
            }
        }
    }
    themes
}
