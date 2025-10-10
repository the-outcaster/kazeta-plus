use macroquad::prelude::*;
use regex::Regex; // get rid of the ugly text that shows the img description
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use crate::{
    audio::SoundEffects,
    config::{Config, get_user_data_dir},
    FONT_SIZE, Screen, BackgroundState, render_background, get_current_font, text_with_config_color, InputState,
};

// --- Structs for State Management ---

/// Represents the different views or states of the downloader UI.
pub enum DownloaderState {
    FetchingList,
    DisplayingList,
    Downloading(String),
    Success(String),
    Error(String),
}

/// A message passed from a background thread to the main UI thread.
enum DownloaderMessage {
    ThemeList(Result<Vec<RemoteTheme>, String>),
    InstallResult(Result<String, String>),
}

/// Holds all the information needed to manage the theme downloader UI.
pub struct ThemeDownloaderState {
    pub screen_state: DownloaderState,
    pub themes: Vec<RemoteTheme>,
    pub selected_index: usize,
    // The receiver for messages from our background threads
    rx: Receiver<DownloaderMessage>,
    // We hold on to the sender so we can spawn new threads
    tx: Sender<DownloaderMessage>,
}

// --- Structs for Deserializing GitHub API Response ---

#[derive(Deserialize, Debug, Clone)]
pub struct RemoteTheme {
    pub name: String,
    pub author: String,
    pub description: String,
    pub download_url: String,
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
    /// Creates a new state and immediately spawns a thread to fetch the theme list.
    pub fn new() -> Self {
        let (tx, rx) = channel();

        // Spawn a thread to fetch the theme list so the UI doesn't freeze.
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

fn wrap_text(text: &str, font: Font, font_size: u16, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let space_width = measure_text(" ", Some(&font), font_size, 1.0).width;

    for paragraph in text.lines() {
        if paragraph.is_empty() {
            lines.push("".to_string());
            continue;
        }

        let mut current_line = String::new();
        let mut current_line_width = 0.0;

        for word in paragraph.split_whitespace() {
            let word_width = measure_text(word, Some(&font), font_size, 1.0).width;

            if !current_line.is_empty() && current_line_width + space_width + word_width > max_width {
                lines.push(current_line);
                current_line = String::new();
                current_line_width = 0.0;
            }

            if !current_line.is_empty() {
                current_line.push(' ');
                current_line_width += space_width;
            }

            current_line.push_str(word);
            current_line_width += word_width;
        }
        lines.push(current_line);
    }

    lines
}

pub fn update(
    state: &mut ThemeDownloaderState,
    input_state: &InputState,
    current_screen: &mut Screen,
    sound_effects: &SoundEffects,
    config: &Config,
) {
    // Always allow going back to the main menu
    if input_state.back {
        *current_screen = Screen::MainMenu;
        sound_effects.play_back(config);
        return;
    }

    // Check for any messages from background threads
    if let Ok(msg) = state.rx.try_recv() {
        match msg {
            DownloaderMessage::ThemeList(Ok(themes)) => {
                state.themes = themes;
                state.screen_state = DownloaderState::DisplayingList;
            }
            DownloaderMessage::ThemeList(Err(e)) => {
                state.screen_state = DownloaderState::Error(e);
            }
            DownloaderMessage::InstallResult(Ok(theme_name)) => {
                state.screen_state = DownloaderState::Success(format!("'{}' installed successfully!", theme_name));

                // After success, tell the main app to reload all themes.
                *current_screen = Screen::ReloadingThemes;
            }
            DownloaderMessage::InstallResult(Err(e)) => {
                state.screen_state = DownloaderState::Error(e);
            }
        }
    }

    // Handle input based on the current state
    match &mut state.screen_state {
        DownloaderState::DisplayingList => {
            if !state.themes.is_empty() {
                if input_state.down && state.selected_index < state.themes.len() - 1 {
                    state.selected_index += 1;
                    sound_effects.play_cursor_move(&config);
                }
                if input_state.up && state.selected_index > 0 {
                    state.selected_index -= 1;
                    sound_effects.play_cursor_move(&config);
                }
                if input_state.select {
                    sound_effects.play_select(config);
                    let theme_to_download = state.themes[state.selected_index].clone();
                    state.screen_state = DownloaderState::Downloading(theme_to_download.name.clone());

                    // Spawn a thread to download and extract the theme
                    download_and_extract_theme(theme_to_download, state.tx.clone());
                }
            }
        }
        DownloaderState::Success(_) | DownloaderState::Error(_) => {
            // After seeing a success/error message, any key press returns to the list
            if input_state.select {
                state.screen_state = DownloaderState::DisplayingList;
                sound_effects.play_select(config);
            }
        }
        _ => { /* No input handled for Fetching, Downloading, etc. */ }
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
            }
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

/// Fetches the list of releases from the GitHub API in a separate thread.
fn fetch_theme_list(tx: Sender<DownloaderMessage>) {
    thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
        .user_agent("KazetaPlus-Theme-Downloader")
        .build().unwrap();

        let response = client.get("https://api.github.com/repos/the-outcaster/kazeta-plus-themes/releases").send();

        let result = match response {
            Ok(resp) => {
                match resp.json::<Vec<GithubRelease>>() {
                    Ok(releases) => {
                        let themes: Vec<RemoteTheme> = releases.into_iter().filter_map(|release| {
                            // Find the .zip asset in the release
                            release.assets.iter().find(|asset| asset.name.ends_with(".zip")).map(|asset| {
                                // A simple way to parse author from the body text
                                let author = release.body.lines()
                                .find(|line| line.to_lowercase().starts_with("author:"))
                                .map(|line| line.split(':').nth(1).unwrap_or("").trim().to_string())
                                .unwrap_or_else(|| "Unknown".to_string());

                                RemoteTheme {
                                    name: release.name,
                                    author,
                                    description: release.body,
                                    download_url: asset.browser_download_url.clone(),
                                }
                            })
                        }).collect();
                        Ok(themes)
                    }
                    Err(_) => Err("Failed to parse theme list from GitHub.".to_string()),
                }
            }
            Err(_) => Err("Failed to fetch theme list from GitHub.".to_string()),
        };
        tx.send(DownloaderMessage::ThemeList(result)).unwrap();
    });
}

/// Downloads and extracts a single theme .zip file in a separate thread.
fn download_and_extract_theme(theme: RemoteTheme, tx: Sender<DownloaderMessage>) {
    thread::spawn(move || {
        let result = (|| -> Result<String, String> {
            let themes_dir = get_user_data_dir().ok_or("Could not find user data directory.")?.join("themes");

            // Download the file to a temporary path
            let response = reqwest::blocking::get(&theme.download_url).map_err(|e| format!("Download failed: {}", e))?;
            let bytes = response.bytes().map_err(|e| format!("Failed to read download: {}", e))?;

            // Extract the zip archive
            let reader = std::io::Cursor::new(bytes);
            let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("Invalid zip file: {}", e))?;

            archive.extract(&themes_dir).map_err(|e| format!("Failed to extract theme: {}", e))?;

            Ok(theme.name)
        })();

        tx.send(DownloaderMessage::InstallResult(result)).unwrap();
    });
}
