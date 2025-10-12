use macroquad::prelude::*;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::{io, fs, thread};
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    audio::SoundEffects,
    config::{Config, get_user_data_dir},
    FONT_SIZE, Screen, BackgroundState, render_background, get_current_font, text_with_config_color, InputState, wrap_text,
};

// --- State Management & Structs ---

pub enum DownloaderState {
    FetchingList,
    DisplayingList,
    Downloading(String),
    Success(String),
    Error(String),
    ConfirmDelete {
        theme_folder_name: String,
        theme_display_name: String,
        selection: usize, // 0 for Yes, 1 for No
    },
}

enum DownloaderMessage {
    ThemeList(Result<Vec<RemoteTheme>, String>),
    InstallResult(Result<String, String>),
}

#[derive(Deserialize, Debug, Clone)]
pub struct RemoteTheme {
    pub name: String,         // Display name, e.g., "Soul Calibur II"
    pub folder_name: String,  // Directory name, e.g., "soul_calibur_ii"
    pub author: String,
    pub description: String,
    pub download_url: String,
}

pub struct ThemeDownloaderState {
    pub screen_state: DownloaderState,
    pub themes: Vec<RemoteTheme>,
    pub selected_index: usize,
    rx: Receiver<DownloaderMessage>,
    tx: Sender<DownloaderMessage>,
}

#[derive(Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    name: String,
    body: String,
    assets: Vec<GithubReleaseAsset>,
}

// --- Implementation ---

impl ThemeDownloaderState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        fetch_theme_list(tx.clone());
        Self {
            screen_state: DownloaderState::FetchingList,
            themes: Vec::new(),
            selected_index: 0,
            rx,
            tx,
        }
    }
}

pub fn update(
    state: &mut ThemeDownloaderState,
    input_state: &InputState,
    current_screen: &mut Screen,
    sound_effects: &SoundEffects,
    config: &Config,
) {
    if input_state.back {
        sound_effects.play_back(config);
        match &state.screen_state {
            // If on the main list, exit to the main menu
            DownloaderState::DisplayingList => *current_screen = Screen::MainMenu,
            // If in a sub-menu (confirm, success, error), go back to the list
            _ => state.screen_state = DownloaderState::DisplayingList,
        }
        return; // Stop further processing after handling 'back'
    }

    if let Ok(msg) = state.rx.try_recv() {
        match msg {
            DownloaderMessage::ThemeList(Ok(themes)) => { state.themes = themes; state.screen_state = DownloaderState::DisplayingList; }
            DownloaderMessage::ThemeList(Err(e)) => { state.screen_state = DownloaderState::Error(e); }
            DownloaderMessage::InstallResult(Ok(theme_name)) => { state.screen_state = DownloaderState::Success(format!("'{}' installed!", theme_name)); *current_screen = Screen::ReloadingThemes; }
            DownloaderMessage::InstallResult(Err(e)) => { state.screen_state = DownloaderState::Error(e); }
        }
    }

    match &mut state.screen_state {
        DownloaderState::DisplayingList => {
            if !state.themes.is_empty() {
                if input_state.down && state.selected_index < state.themes.len() - 1 { state.selected_index += 1; sound_effects.play_cursor_move(&config); }
                if input_state.up && state.selected_index > 0 { state.selected_index -= 1; sound_effects.play_cursor_move(&config); }
                if input_state.select {
                    sound_effects.play_select(config);
                    let theme_to_download = state.themes[state.selected_index].clone();
                    state.screen_state = DownloaderState::Downloading(theme_to_download.name.clone());
                    download_and_extract_theme(theme_to_download, state.tx.clone());
                }
                if input_state.secondary {
                    let theme_to_delete = &state.themes[state.selected_index];
                    if theme_to_delete.name != "Default" {
                        sound_effects.play_select(config);
                        state.screen_state = DownloaderState::ConfirmDelete {
                            theme_folder_name: theme_to_delete.folder_name.clone(),
                            theme_display_name: theme_to_delete.name.clone(),
                            selection: 1,
                        };
                    } else {
                        sound_effects.play_reject(config);
                    }
                }
            }
        }
        DownloaderState::ConfirmDelete { theme_folder_name, theme_display_name, selection } => {
            if input_state.left || input_state.right { *selection = 1 - *selection; sound_effects.play_cursor_move(&config); }
            if input_state.select {
                sound_effects.play_select(config);
                if *selection == 0 {
                    let theme_path = get_user_data_dir().unwrap().join("themes").join(theme_folder_name);
                    match fs::remove_dir_all(&theme_path) {
                        Ok(_) => {
                            state.screen_state = DownloaderState::Success(format!("'{}' deleted.", theme_display_name));
                            *current_screen = Screen::ReloadingThemes;
                        }
                        Err(e) => { state.screen_state = DownloaderState::Error(format!("Failed to delete: {}", e)); }
                    }
                } else {
                    state.screen_state = DownloaderState::DisplayingList;
                }
            }
            if input_state.back {
                sound_effects.play_back(config);
                state.screen_state = DownloaderState::DisplayingList;
            }
        }
        DownloaderState::Success(_) | DownloaderState::Error(_) => {
            // --- THIS IS THE FIX ---
            if input_state.select || input_state.back {
                // After success/error, re-fetch the list to show the change
                fetch_theme_list(state.tx.clone());
                state.screen_state = DownloaderState::FetchingList;
                sound_effects.play_select(config);
            }
        }
        _ => {}
    }
}

pub fn draw(
    state: &ThemeDownloaderState,
    animation_state: &mut crate::AnimationState,
    background_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    background_state: &mut BackgroundState,
    scale_factor: f32,
) {
    render_background(&background_cache, &config, background_state);

    let font = get_current_font(font_cache, config);
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let line_height = font_size as f32 * 1.5;

    // Create a container for the UI
    let container_w = screen_width() * 0.9;
    let container_h = screen_height() * 0.8;
    let container_x = (screen_width() - container_w) / 2.0;
    let container_y = (screen_height() - container_h) / 2.0;
    draw_rectangle(container_x, container_y, container_w, container_h, Color::new(0.0, 0.0, 0.0, 0.75));

    let text_x = container_x + 30.0 * scale_factor;
    let text_y_start = container_y + 40.0 * scale_factor;

    match &state.screen_state {
        DownloaderState::FetchingList => {
            let text = "Fetching theme list from GitHub...";
            let text_dims = measure_text(text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        DownloaderState::DisplayingList => {
            if state.themes.is_empty() {
                text_with_config_color(font_cache, config, "No themes found.", text_x, text_y_start, font_size);
            } else {
                for (i, theme) in state.themes.iter().enumerate() {
                    let y_pos = text_y_start + (i as f32 * line_height);
                    if i == state.selected_index {
                        let cursor_color = animation_state.get_cursor_color(config);
                        draw_rectangle(container_x, y_pos - font_size as f32 - 5.0, container_w, line_height, Color::new(cursor_color.r, cursor_color.g, cursor_color.b, 0.3));
                    }
                    let display_text = format!("{} by {}", theme.name, theme.author);
                    text_with_config_color(font_cache, config, &display_text, text_x, y_pos, font_size);
                }
                // --- NEW: Draw the selected theme's description ---
                let separator_y = container_y + container_h / 2.0;
                draw_line(container_x, separator_y, container_x + container_w, separator_y, 2.0, Color::new(1.0, 1.0, 1.0, 0.2));

                if let Some(selected_theme) = state.themes.get(state.selected_index) {

                    // --- NEW: Filter out the "Author:" line from the description ---
                    let description_without_author = selected_theme.description
                    .lines()
                    .filter(|line| !line.trim().to_lowercase().starts_with("author:"))
                    .collect::<Vec<&str>>()
                    .join("\n");

                    let img_tag_regex = Regex::new(r"<img[^>]*>").unwrap();
                    let clean_description = img_tag_regex.replace_all(&description_without_author, "");

                    let wrap_width = container_w - 60.0 * scale_factor;
                    let wrapped_lines = wrap_text(clean_description.trim(), font.clone(), font_size, wrap_width);

                    for (i, line) in wrapped_lines.iter().enumerate() {
                        let y_pos = separator_y + 40.0 * scale_factor + (i as f32 * line_height);
                        text_with_config_color(font_cache, config, line, text_x, y_pos, font_size);
                    }
                }
                // Add a UI hint for the new delete button
                let hint_text = "Press [SELECT] to Download, [SECONDARY] to Delete";
                let hint_dims = measure_text(hint_text, Some(font), (font_size as f32 * 0.8) as u16, 1.0);
                text_with_config_color(font_cache, config, hint_text, screen_width() / 2.0 - hint_dims.width / 2.0, container_y + container_h - 20.0, (font_size as f32 * 0.8) as u16);
            }
        }
        DownloaderState::ConfirmDelete { theme_display_name, selection, .. } => {
            let dialog_w = 400.0 * scale_factor;
            let dialog_h = 150.0 * scale_factor;
            let dialog_x = screen_width() / 2.0 - dialog_w / 2.0;
            let dialog_y = screen_height() / 2.0 - dialog_h / 2.0;
            draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::new(0.1, 0.1, 0.1, 0.9));
            draw_rectangle_lines(dialog_x, dialog_y, dialog_w, dialog_h, 3.0, WHITE);

            let question = format!("Delete '{}'?", theme_display_name);
            let question_dims = measure_text(&question, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, &question, screen_width() / 2.0 - question_dims.width / 2.0, dialog_y + 40.0 * scale_factor, font_size);

            let yes_text = "YES";
            let no_text = "NO";
            let yes_dims = measure_text(yes_text, Some(font), font_size, 1.0);
            let no_dims = measure_text(no_text, Some(font), font_size, 1.0);
            let yes_x = screen_width() / 2.0 - yes_dims.width - 20.0 * scale_factor;
            let no_x = screen_width() / 2.0 + 20.0 * scale_factor;
            let options_y = dialog_y + dialog_h - 50.0 * scale_factor;
            text_with_config_color(font_cache, config, yes_text, yes_x, options_y, font_size);
            text_with_config_color(font_cache, config, no_text, no_x, options_y, font_size);

            let cursor_x = if *selection == 0 { yes_x } else { no_x };
            let cursor_w = if *selection == 0 { yes_dims.width } else { no_dims.width };
            let cursor_color = animation_state.get_cursor_color(config);
            draw_rectangle_lines(cursor_x - 5.0, options_y - font_size as f32, cursor_w + 10.0, line_height, 3.0, cursor_color);
        }
        DownloaderState::Downloading(name) => {
            let text = format!("Downloading {}...", name);
            let text_dims = measure_text(&text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, &text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        DownloaderState::Success(msg) | DownloaderState::Error(msg) => {
            let text_dims = measure_text(msg, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, msg, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);

            let continue_text = "Press A to continue";
            let continue_dims = measure_text(continue_text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, continue_text, screen_width() / 2.0 - continue_dims.width / 2.0, screen_height() / 2.0 + line_height * 2.0, font_size);
        }
    }
}

// --- Background Thread Functions ---

fn fetch_theme_list(tx: Sender<DownloaderMessage>) {
    thread::spawn(move || {
        let client = reqwest::blocking::Client::builder().user_agent("KazetaPlus-Theme-Downloader").build().unwrap();
        let response = client.get("https://api.github.com/repos/the-outcaster/kazeta-plus-themes/releases").send();
        let result = match response {
            Ok(resp) => match resp.json::<Vec<GithubRelease>>() {
                Ok(releases) => {
                    let themes: Vec<RemoteTheme> = releases.into_iter().filter_map(|release| {
                        release.assets.iter().find(|asset| asset.name.ends_with(".zip")).map(|asset| {
                            let author = release.body.lines().find(|line| line.to_lowercase().starts_with("author:")).map(|line| line.split(':').nth(1).unwrap_or("").trim().to_string()).unwrap_or_else(|| "Unknown".to_string());
                            let folder_name = asset.name.strip_suffix(".zip").unwrap_or(&asset.name).to_string();
                            RemoteTheme {
                                name: release.name,
                                folder_name,
                                author,
                                description: release.body,
                                download_url: asset.browser_download_url.clone(),
                            }
                        })
                    }).collect();
                    Ok(themes)
                }
                Err(_) => Err("Failed to parse theme list from GitHub.".to_string()),
            },
            Err(_) => Err("Failed to fetch theme list from GitHub.".to_string()),
        };
        tx.send(DownloaderMessage::ThemeList(result)).unwrap();
    });
}

fn download_and_extract_theme(theme: RemoteTheme, tx: Sender<DownloaderMessage>) {
    thread::spawn(move || {
        let result = (|| -> Result<String, String> {
            let themes_dir = get_user_data_dir().ok_or("Could not find user data directory.")?.join("themes");
            let response_bytes = reqwest::blocking::get(&theme.download_url).map_err(|e| format!("Download failed: {}", e))?.bytes().map_err(|e| format!("Failed to read download: {}", e))?;
            let reader = io::Cursor::new(response_bytes);
            let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("Invalid zip file: {}", e))?;
            archive.extract(&themes_dir).map_err(|e| format!("Failed to extract theme: {}", e))?;
            Ok(theme.name)
        })();
        tx.send(DownloaderMessage::InstallResult(result)).unwrap();
    });
}
