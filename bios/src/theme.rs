use std::fs;
//use std::path::PathBuf;
use crate::config::{Config, ThemeConfig, get_user_data_dir};

/// Scans for theme folders and returns a list of their names.
pub fn find_themes() -> Vec<String> {
    let mut themes = vec!["Default".to_string()];

    if let Some(mut theme_dir) = get_user_data_dir() {
        theme_dir.push("themes");
        if let Ok(entries) = fs::read_dir(theme_dir) {
            let mut found_themes: Vec<String> = entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
            themes.append(&mut found_themes);
        }
    }
    themes
}

/// Loads a theme by name and applies its settings to the main config.
pub fn load_theme(config: &mut Config, theme_name: &str) -> Result<(), String> {
    // Handle the special "Default" case
    if theme_name == "Default" {
        *config = Config::default();
        return Ok(());
    }

    // Get the path to the theme directory
    let mut theme_dir = get_user_data_dir().ok_or("Could not find user data directory.")?;
    theme_dir.push("themes");
    theme_dir.push(theme_name);

    // Read and parse the theme.toml file
    let toml_path = theme_dir.join("theme.toml");
    let toml_content = fs::read_to_string(&toml_path)
    .map_err(|e| format!("Could not read theme.toml: {}", e))?;
    let theme_config: ThemeConfig = toml::from_str(&toml_content)
    .map_err(|e| format!("Could not parse theme.toml: {}", e))?;

    // Apply settings from the theme to the main config
    config.menu_position = theme_config.menu_position;
    config.font_color = theme_config.font_color;
    config.cursor_color = theme_config.cursor_color;
    config.background_scroll_speed = theme_config.background_scroll_speed;
    config.color_shift_speed = theme_config.color_shift_speed;

    // IMPORTANT: For assets, construct the full path
    config.bgm_track = Some(theme_dir.join(theme_config.bgm_track).to_string_lossy().into_owned());
    config.logo_selection = theme_dir.join(theme_config.logo_selection).to_string_lossy().into_owned();
    config.background_selection = theme_dir.join(theme_config.background_selection).to_string_lossy().into_owned();
    config.font_selection = theme_dir.join(theme_config.font_selection).to_string_lossy().into_owned();

    Ok(())
}
