use crate::{
    audio::SoundEffects,
    config::{Config, get_user_data_dir},
    FONT_SIZE, Screen, BackgroundState, render_background, get_current_font, text_with_config_color, InputState, wrap_text, VideoPlayer,
};
use macroquad::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fs, io, thread,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{channel, Receiver, Sender},
};
use toml;

// --- CONSTANTS ---
const ITEMS_PER_PAGE: usize = 5;

// --- State Management & Structs ---

pub enum DownloaderState {
    Idle,
    FetchingList,
    DisplayingList,
    Downloading(String),
    Success(String),
    Error(String),
    ConfirmDelete {
        theme_folder_name: String,
        theme_display_name: String,
        selection: usize,
    },
    ConfirmRedownload {
        theme: RemoteTheme,
        selection: usize, // 0=Yes, 1=No
    },
    ConfirmConvertToWav { selection: usize }, // 0=Yes, 1=No
    ConfirmConvertToOgg { selection: usize }, // 0=Yes, 1=No
    ConfirmDeleteAllBGM { selection: usize },
    Converting(String), // Shows progress message, e.g., "Converting files..."
}

enum DownloaderMessage {
    ThemeList(Result<Vec<RemoteTheme>, String>),
    InstallResult(Result<String, String>),
    ConversionResult(Result<String, String>), // -- NEW -- For audio conversion success/error
}

#[derive(Deserialize, Debug, Clone)]
pub struct RemoteTheme {
    pub name: String,         // Display name, e.g., "Soul Calibur II"
    pub folder_name: String,  // Directory name, e.g., "soul_calibur_ii"
    pub author: String,
    pub description: String,
    pub download_url: String,
    #[serde(default)]
    pub is_installed: bool,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct ThemeToml {
    author: Option<String>,
    description: Option<String>,
    menu_position: Option<String>,
    font_color: Option<String>,
    cursor_color: Option<String>,
    background_scroll_speed: Option<String>,
    color_shift_speed: Option<String>,
    bgm_track: Option<String>,
    logo_selection: Option<String>,
    background_selection: Option<String>,
    font_selection: Option<String>,
    sfx_pack: Option<String>,
}

pub struct ThemeDownloaderState {
    pub screen_state: DownloaderState,
    pub themes: Vec<RemoteTheme>,
    pub selected_index: usize,
    rx: Receiver<DownloaderMessage>,
    tx: Sender<DownloaderMessage>,
    pub has_audio_tools_option: bool,
    pub current_page: usize,
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
        Self {
            screen_state: DownloaderState::Idle,
            themes: Vec::new(),
            selected_index: 0,
            rx,
            tx,
            has_audio_tools_option: true,
            current_page: 0,
        }
    }

    fn start_fetch(&mut self) {
        fetch_theme_list(self.tx.clone());
        self.screen_state = DownloaderState::FetchingList;
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
            DownloaderState::DisplayingList => {
                *current_screen = Screen::Extras;
                state.screen_state = DownloaderState::Idle; // Reset for next time
            }
            _ => { // For any sub-menu, go back to the list and reset page
                state.screen_state = DownloaderState::DisplayingList;
                state.current_page = state.selected_index / ITEMS_PER_PAGE;
            }
        }
        return;
    }

    if let Ok(msg) = state.rx.try_recv() {
        match msg {
            DownloaderMessage::ThemeList(Ok(mut themes)) => { // Make themes mutable
                // Get a list of folders currently installed
                let installed_themes = get_installed_theme_folders();

                // Check each remote theme against the installed list
                for theme in themes.iter_mut() {
                    if installed_themes.contains(&theme.folder_name) {
                        theme.is_installed = true;
                    }
                }

                state.themes = themes;
                state.screen_state = DownloaderState::DisplayingList;
            }
            DownloaderMessage::ThemeList(Err(e)) => { state.screen_state = DownloaderState::Error(e); }
            DownloaderMessage::InstallResult(Ok(theme_name)) => { state.screen_state = DownloaderState::Success(format!("'{}' installed!", theme_name)); *current_screen = Screen::ReloadingThemes; }
            DownloaderMessage::InstallResult(Err(e)) => { state.screen_state = DownloaderState::Error(e); }
            DownloaderMessage::ConversionResult(Ok(msg)) => {
                state.screen_state = DownloaderState::Success(msg);
                *current_screen = Screen::ReloadingThemes; // reload assets whenever we delete or convert BGM tracks
            }
            DownloaderMessage::ConversionResult(Err(e)) => { state.screen_state = DownloaderState::Error(e); }
        }
    }

    // if the screen is idle, trigger a new fetch.
    if let DownloaderState::Idle = state.screen_state {
        state.start_fetch();
    }

    match &mut state.screen_state {
        DownloaderState::DisplayingList => {
            let total_options = state.themes.len() + if state.has_audio_tools_option { 3 } else { 0 };
            if total_options == 0 { return; }

            let total_pages = (total_options + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE;

            if input_state.down {
                if state.selected_index < total_options - 1 {
                    state.selected_index += 1;
                    sound_effects.play_cursor_move(config);
                }
            }
            if input_state.up {
                if state.selected_index > 0 {
                    state.selected_index -= 1;
                    sound_effects.play_cursor_move(config);
                }
            }
            if input_state.right {
                if state.current_page < total_pages - 1 {
                    state.current_page += 1;
                    state.selected_index = state.current_page * ITEMS_PER_PAGE;
                    sound_effects.play_cursor_move(config);
                }
            }
            if input_state.left {
                if state.current_page > 0 {
                    state.current_page -= 1;
                    state.selected_index = state.current_page * ITEMS_PER_PAGE;
                    sound_effects.play_cursor_move(config);
                }
            }

            // Auto-update current page based on selection
            state.current_page = state.selected_index / ITEMS_PER_PAGE;

            // Handle selection
            if input_state.select {
                sound_effects.play_select(config);
                if state.selected_index < state.themes.len() {
                    let theme = state.themes[state.selected_index].clone();

                    if theme.is_installed {
                        // Theme is already installed, show confirmation
                        state.screen_state = DownloaderState::ConfirmRedownload {
                            theme: theme,
                            selection: 1, // Default to "NO"
                        };
                    } else {
                        // Not installed, download immediately
                        state.screen_state = DownloaderState::Downloading(theme.name.clone());
                        download_and_extract_theme(theme, state.tx.clone());
                    }
                } else {
                    // This is the existing logic for audio tools
                    let tool_index = state.selected_index - state.themes.len();
                    if tool_index == 0 {
                        state.screen_state = DownloaderState::ConfirmConvertToWav { selection: 1 };
                    } else if tool_index == 1 {
                        state.screen_state = DownloaderState::ConfirmConvertToOgg { selection: 1 };
                    } else if tool_index == 2 { // New option
                        state.screen_state = DownloaderState::ConfirmDeleteAllBGM { selection: 1 };
                    }
                }
            }
            // Handle delete
            if input_state.secondary && state.selected_index < state.themes.len() {
                let theme_to_delete = &state.themes[state.selected_index];

                // Only allow deletion if the theme is installed AND it's not the "Default" theme
                if theme_to_delete.is_installed && theme_to_delete.name != "Default" {
                    sound_effects.play_select(config); // Or a "delete" sound
                    state.screen_state = DownloaderState::ConfirmDelete {
                        theme_folder_name: theme_to_delete.folder_name.clone(),
                        theme_display_name: theme_to_delete.name.clone(),
                        selection: 1, // Default to "NO"
                    };
                } else {
                    // Play reject sound if theme is not installed or is "Default"
                    sound_effects.play_reject(config);
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
        DownloaderState::ConfirmRedownload { theme, selection } => {
            if input_state.left || input_state.right {
                *selection = 1 - *selection; // Flips between 0 (Yes) and 1 (No)
                sound_effects.play_cursor_move(config);
            }
            if input_state.select {
                sound_effects.play_select(config);
                if *selection == 0 { // User selected YES
                    // Clone the theme *before* changing the state,
                    // so we are not using the borrowed `theme` variable after the state change.
                    let theme_to_download = theme.clone();

                    state.screen_state = DownloaderState::Downloading(theme_to_download.name.clone());
                    download_and_extract_theme(theme_to_download, state.tx.clone());
                } else { // User selected NO
                    state.screen_state = DownloaderState::DisplayingList;
                }
            }
            // Back button also cancels
            if input_state.back {
                sound_effects.play_back(config);
                state.screen_state = DownloaderState::DisplayingList;
            }
        }
        DownloaderState::ConfirmConvertToWav { selection } => {
            if input_state.left || input_state.right { *selection = 1 - *selection; sound_effects.play_cursor_move(&config); }
            if input_state.select {
                sound_effects.play_select(config);
                if *selection == 0 { // YES
                    state.screen_state = DownloaderState::Converting("Searching for .ogg files...".to_string());
                    convert_files_to_wav(state.tx.clone());
                } else { // NO
                    state.screen_state = DownloaderState::DisplayingList;
                }
            }
        }
        DownloaderState::ConfirmConvertToOgg { selection } => {
            if input_state.left || input_state.right { *selection = 1 - *selection; sound_effects.play_cursor_move(&config); }
            if input_state.select {
                sound_effects.play_select(config);
                if *selection == 0 { // YES
                    state.screen_state = DownloaderState::Converting("Searching for .wav files...".to_string());
                    convert_files_to_ogg(state.tx.clone());
                } else { // NO
                    state.screen_state = DownloaderState::DisplayingList;
                }
            }
        }
        DownloaderState::ConfirmDeleteAllBGM { selection } => {
            if input_state.left || input_state.right { *selection = 1 - *selection; sound_effects.play_cursor_move(config); }
            if input_state.select {
                sound_effects.play_select(config);
                if *selection == 0 { // YES
                    state.screen_state = DownloaderState::Converting("Deleting all BGM files...".to_string());
                    delete_all_bgm_files(state.tx.clone()); // Call the new function
                } else { // NO
                    state.screen_state = DownloaderState::DisplayingList;
                }
            }
            if input_state.back {
                sound_effects.play_back(config);
                state.screen_state = DownloaderState::DisplayingList;
            }
        }
        DownloaderState::Success(_) | DownloaderState::Error(_) => {
            if input_state.select || input_state.back {
                // After success/error, go back to the list.
                // Setting to Idle will trigger a re-fetch of the remote list.
                state.screen_state = DownloaderState::Idle;
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
    video_cache: &mut HashMap<String, VideoPlayer>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    background_state: &mut BackgroundState,
    scale_factor: f32,
) {
    render_background(&background_cache, video_cache, &config, background_state);

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
        DownloaderState::Idle => {
            let text = "Connecting to theme repository...";
            let text_dims = measure_text(text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        DownloaderState::FetchingList => {
            let text = "Fetching theme list from GitHub...";
            let text_dims = measure_text(text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        DownloaderState::DisplayingList => {
            let total_options = state.themes.len() + if state.has_audio_tools_option { 3 } else { 0 };
            if total_options == 0 {
                text_with_config_color(font_cache, config, "No themes or tools available.", text_x, text_y_start, font_size);
                return;
            }
            let total_pages = (total_options + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE;
            let start_index = state.current_page * ITEMS_PER_PAGE;
            let end_index = (start_index + ITEMS_PER_PAGE).min(total_options);

            // Draw items for the current page
            for i in start_index..end_index {
                let item_on_page = i - start_index;
                let y_pos = text_y_start + (item_on_page as f32 * line_height) + 20.0;

                if i == state.selected_index {
                    let cursor_color = animation_state.get_cursor_color(config);
                    draw_rectangle(container_x, y_pos - font_size as f32 - 5.0, container_w, line_height, Color::new(cursor_color.r, cursor_color.g, cursor_color.b, 0.3));
                }

                let display_text = if i < state.themes.len() {
                    let theme = &state.themes[i];
                    let installed_flag = if theme.is_installed { " [INSTALLED]" } else { "" };
                    format!("{} by {}{}", theme.name, theme.author, installed_flag)
                } else {
                    let tool_index = i - state.themes.len();
                    if tool_index == 0 { "Audio Tools: Convert .OGG to .WAV".to_string() }
                    else if tool_index == 1 { "Audio Tools: Convert .WAV to .OGG".to_string() }
                    else { "Audio Tools: Delete All BGM Tracks".to_string() } // New option
                };
                text_with_config_color(font_cache, config, &display_text, text_x, y_pos, font_size);
            }

            // Draw description panel
            let separator_y = text_y_start + (ITEMS_PER_PAGE as f32 * line_height) + (line_height / 2.0);
            draw_line(container_x, separator_y, container_x + container_w, separator_y, 2.0, Color::new(1.0, 1.0, 1.0, 0.2));

            let description_text = if state.selected_index < state.themes.len() {
                let selected_theme = &state.themes[state.selected_index];
                let description_without_author = selected_theme.description
                .lines()
                .filter(|line| !line.trim().to_lowercase().starts_with("author:"))
                .collect::<Vec<&str>>()
                .join("\n");
                let img_tag_regex = Regex::new(r"<img[^>]*>").unwrap();
                img_tag_regex.replace_all(&description_without_author, "").to_string()
            } else {
                let tool_index = state.selected_index - state.themes.len();
                if tool_index == 0 {
                    "Converts space-saving .ogg files into faster-loading .wav files.\n\nThis uses more disk space.".to_string()
                } else if tool_index == 1 {
                    "Converts large .wav files into space-saving .ogg files.\n\nThis may increase theme loading times.".to_string()
                } else {
                    // New description
                    "Deletes all .wav and .ogg BGM files from all theme and bgm folders.\n\nThis will NOT delete sound effects (SFX) packs.".to_string()
                }
            };

            // -- NEW -- Define a smaller font size and line height for the description
            let description_font_size = (font_size as f32 * 0.8) as u16;
            let description_line_height = description_font_size as f32 * 1.5;

            let wrap_width = container_w - 60.0 * scale_factor;
            // -- CHANGED -- Use the new, smaller font size for text wrapping
            let wrapped_lines = wrap_text(description_text.trim(), font.clone(), description_font_size, wrap_width);
            for (i, line) in wrapped_lines.iter().enumerate() {
                // -- CHANGED -- Use the new line height and font size for drawing
                let y_pos = separator_y + 40.0 * scale_factor + (i as f32 * description_line_height);
                text_with_config_color(font_cache, config, line, text_x, y_pos, description_font_size);
            }

            // Draw pagination controls and hint text
            let hint_y = container_y + container_h - 20.0;
            let hint_text = "Press [SOUTH] to Download, [WEST] to Delete";
            let hint_dims = measure_text(hint_text, Some(font), (font_size as f32 * 0.8) as u16, 1.0);
            text_with_config_color(font_cache, config, hint_text, screen_width() / 2.0 - hint_dims.width / 2.0, hint_y, (font_size as f32 * 0.8) as u16);

            if total_pages > 1 {
                let page_text = format!("Page {} / {}", state.current_page + 1, total_pages);
                let page_dims = measure_text(&page_text, Some(font), (font_size as f32 * 0.8) as u16, 1.0);
                text_with_config_color(font_cache, config, &page_text, screen_width() / 2.0 - page_dims.width / 2.0, text_y_start - (line_height * 0.8), (font_size as f32 * 0.8) as u16);
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
        DownloaderState::ConfirmRedownload { theme, selection } => {
            let dialog_w = 500.0 * scale_factor; // Made dialog wider for new text
            let dialog_h = 170.0 * scale_factor; // Made dialog taller
            let dialog_x = screen_width() / 2.0 - dialog_w / 2.0;
            let dialog_y = screen_height() / 2.0 - dialog_h / 2.0;
            draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::new(0.1, 0.1, 0.1, 0.9));
            draw_rectangle_lines(dialog_x, dialog_y, dialog_w, dialog_h, 3.0, WHITE);

            // Line 1
            let question = format!("'{}' is already installed.", theme.name);
            let question_dims = measure_text(&question, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, &question, screen_width() / 2.0 - question_dims.width / 2.0, dialog_y + 40.0 * scale_factor, font_size);

            // Line 2
            let question2 = "Re-download and overwrite?";
            let question_dims2 = measure_text(question2, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, question2, screen_width() / 2.0 - question_dims2.width / 2.0, dialog_y + 40.0 * scale_factor + line_height, font_size);

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
        DownloaderState::ConfirmConvertToWav { selection } => {
            // -- FIX -- Pass `font` directly without cloning
            draw_conversion_dialog(
                font_cache, config, font, font_size, line_height, scale_factor, animation_state,
                "Convert Audio to .WAV?",
                &[
                    "This will convert all .ogg BGM files to .wav format.",
                    "Benefits: Faster theme loading times.",
                    "Drawbacks: Uses significantly more disk space.",
                ],
                *selection
            );
        }
        DownloaderState::ConfirmConvertToOgg { selection } => {
            // -- FIX -- Pass `font` directly without cloning
            draw_conversion_dialog(
                font_cache, config, font, font_size, line_height, scale_factor, animation_state,
                "Convert Audio to .OGG?",
                &[
                    "This will convert all .wav BGM files to .ogg format.",
                    "Benefits: Frees up a lot of disk space.",
                    "Drawbacks: Slower theme loading times.",
                ],
                *selection
            );
        }
        DownloaderState::ConfirmDeleteAllBGM { selection } => {
            draw_conversion_dialog(
                font_cache, config, font, font_size, line_height, scale_factor, animation_state,
                "Delete All BGM Tracks?",
                &[
                    "This will delete all .wav and .ogg files from:",
                    "  - /themes/...",
                    "  - /bgm/...",
                    "\nSound effect packs (SFX) will NOT be touched.",
                    "This cannot be undone.",
                ],
                *selection
            );
        }
        DownloaderState::Converting(msg) => {
            let text_dims = measure_text(msg, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, msg, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        DownloaderState::Downloading(name) => {
            let text = format!("Downloading {}...", name);
            let text_dims = measure_text(&text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, &text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        DownloaderState::Success(msg) | DownloaderState::Error(msg) => {
            let text_dims = measure_text(msg, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, msg, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);

            let continue_text = "Press [SOUTH] to continue";
            let continue_dims = measure_text(continue_text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, continue_text, screen_width() / 2.0 - continue_dims.width / 2.0, screen_height() / 2.0 + line_height * 2.0, font_size);
        }
    }
}

// -- NEW -- Helper function to draw the dialog box for conversions
fn draw_conversion_dialog(
    font_cache: &HashMap<String, Font>, config: &Config, font: &Font, font_size: u16, line_height: f32, scale_factor: f32, animation_state: &mut crate::AnimationState,
    title: &str, body_lines: &[&str], selection: usize
) {
    let dialog_w = 600.0 * scale_factor;
    let dialog_h = 300.0 * scale_factor;
    let dialog_x = screen_width() / 2.0 - dialog_w / 2.0;
    let dialog_y = screen_height() / 2.0 - dialog_h / 2.0;
    draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::new(0.1, 0.1, 0.1, 0.9));
    draw_rectangle_lines(dialog_x, dialog_y, dialog_w, dialog_h, 3.0, WHITE);

    let title_dims = measure_text(title, Some(font), font_size, 1.0);
    text_with_config_color(font_cache, config, title, screen_width() / 2.0 - title_dims.width / 2.0, dialog_y + 40.0 * scale_factor, font_size);

    for (i, line) in body_lines.iter().enumerate() {
        text_with_config_color(font_cache, config, line, dialog_x + 20.0 * scale_factor, dialog_y + 80.0 * scale_factor + (i as f32 * line_height), font_size);
    }

    let yes_text = "YES";
    let no_text = "NO";
    let yes_dims = measure_text(yes_text, Some(font), font_size, 1.0);
    let no_dims = measure_text(no_text, Some(font), font_size, 1.0);
    let yes_x = screen_width() / 2.0 - yes_dims.width - 40.0 * scale_factor;
    let no_x = screen_width() / 2.0 + 40.0 * scale_factor;
    let options_y = dialog_y + dialog_h - 50.0 * scale_factor;
    text_with_config_color(font_cache, config, yes_text, yes_x, options_y, font_size);
    text_with_config_color(font_cache, config, no_text, no_x, options_y, font_size);

    let cursor_x = if selection == 0 { yes_x } else { no_x };
    let cursor_w = if selection == 0 { yes_dims.width } else { no_dims.width };
    let cursor_color = animation_state.get_cursor_color(config);
    draw_rectangle_lines(cursor_x - 10.0, options_y - font_size as f32, cursor_w + 20.0, line_height, 3.0, cursor_color);
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
                                is_installed: false,
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

fn convert_files_to_wav(tx: Sender<DownloaderMessage>) {
    thread::spawn(move || {
        let result = (|| -> Result<String, String> {
            let files_to_convert = find_files_by_extension(".ogg")?;
            if files_to_convert.is_empty() {
                return Ok("No .ogg files found to convert.".to_string());
            }

            for path in &files_to_convert {
                let mut wav_path = path.clone();
                wav_path.set_extension("wav");

                let status = Command::new("ffmpeg")
                .arg("-i").arg(path)
                .arg("-y") // Overwrite output file if it exists
                .arg(&wav_path)
                .status()
                .map_err(|e| format!("Is ffmpeg installed? Command failed: {}", e))?;

                if !status.success() {
                    return Err(format!("ffmpeg failed for {}", path.display()));
                }

                update_theme_toml(path, ".wav")?;
                fs::remove_file(path).map_err(|e| format!("Failed to delete old file: {}", e))?;
            }
            Ok(format!("Successfully converted {} file(s) to .wav!", files_to_convert.len()))
        })();
        tx.send(DownloaderMessage::ConversionResult(result)).unwrap();
    });
}

// -- REVERTED -- Back to using the simpler ffmpeg command-line tool.
fn convert_files_to_ogg(tx: Sender<DownloaderMessage>) {
    thread::spawn(move || {
        let result = (|| -> Result<String, String> {
            let files_to_convert = find_files_by_extension(".wav")?;
            if files_to_convert.is_empty() {
                return Ok("No .wav files found to convert.".to_string());
            }

            for path in &files_to_convert {
                let mut ogg_path = path.clone();
                ogg_path.set_extension("ogg");

                let status = Command::new("ffmpeg")
                .arg("-i").arg(path)
                .arg("-y")
                .arg("-acodec").arg("libvorbis") // Specify ogg codec
                .arg(&ogg_path)
                .status()
                .map_err(|e| format!("Is ffmpeg installed? Command failed: {}", e))?;

                if !status.success() {
                    return Err(format!("ffmpeg failed for {}", path.display()));
                }

                update_theme_toml(path, ".ogg")?;
                fs::remove_file(path).map_err(|e| format!("Failed to delete old file: {}", e))?;
            }
            Ok(format!("Successfully converted {} file(s) to .ogg!", files_to_convert.len()))
        })();
        tx.send(DownloaderMessage::ConversionResult(result)).unwrap();
    });
}

// -- CHANGED -- Now ignores files in directories containing `_sfx`.
fn find_files_by_extension(ext: &str) -> Result<Vec<PathBuf>, String> {
    let mut found_files = Vec::new();
    let base_dir = get_user_data_dir().ok_or("Could not find user data directory.")?;
    let dirs_to_search = [base_dir.join("bgm"), base_dir.join("themes")];

    for dir in dirs_to_search.iter() {
        if !dir.exists() { continue; }
        for entry in walkdir::WalkDir::new(dir) {
            let entry = entry.map_err(|e| format!("Error walking directory: {}", e))?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some(&ext[1..]) {
                // Check if any part of the path indicates it's an SFX pack.
                let is_sfx_file = path.components().any(|c| {
                    c.as_os_str().to_string_lossy().contains("_sfx")
                });

                if !is_sfx_file {
                    found_files.push(path.to_path_buf());
                }
            }
        }
    }
    Ok(found_files)
}

// -- NEW -- Helper to update the theme.toml file after a conversion
fn update_theme_toml(audio_path: &Path, new_ext: &str) -> Result<(), String> {
    // Find a theme.toml in the parent directory of the audio file
    if let Some(parent_dir) = audio_path.parent() {
        let toml_path = parent_dir.join("theme.toml");
        if toml_path.exists() {
            let content = fs::read_to_string(&toml_path).map_err(|e| format!("Failed to read theme.toml: {}", e))?;
            let mut theme_data: ThemeToml = toml::from_str(&content).map_err(|e| format!("Failed to parse theme.toml: {}", e))?;

            if let Some(bgm_track) = theme_data.bgm_track.as_mut() {
                let mut new_track_path = PathBuf::from(bgm_track.as_str());
                new_track_path.set_extension(&new_ext[1..]);
                *bgm_track = new_track_path.to_string_lossy().to_string();
            }

            let new_content = toml::to_string(&theme_data).map_err(|e| format!("Failed to serialize theme.toml: {}", e))?;
            fs::write(toml_path, new_content).map_err(|e| format!("Failed to write theme.toml: {}", e))?;
        }
    }
    Ok(())
}

/// Scans the user's themes directory and returns a HashSet of installed theme folder names.
fn get_installed_theme_folders() -> HashSet<String> {
    if let Some(themes_dir) = get_user_data_dir().map(|d| d.join("themes")) {
        if let Ok(entries) = fs::read_dir(themes_dir) {
            // Use flatten() to filter out any read errors on individual entries
            return entries.flatten()
            .filter_map(|entry| {
                // Check if it's a directory
                if entry.path().is_dir() {
                    // Try to convert the file/folder name to a String
                    entry.file_name().into_string().ok()
                } else {
                    None
                }
            })
            .collect();
        }
    }
    // Return an empty set if any step failed
    HashSet::new()
}

fn delete_all_bgm_files(tx: Sender<DownloaderMessage>) {
    thread::spawn(move || {
        let result = (|| -> Result<String, String> {
            let wav_files = find_files_by_extension(".wav")?;
            let ogg_files = find_files_by_extension(".ogg")?;

            if wav_files.is_empty() && ogg_files.is_empty() {
                return Ok("No BGM files found to delete.".to_string());
            }

            let mut delete_count = 0;
            let mut toml_paths = HashSet::new();

            // Iterate over all files, delete them, and collect their parent toml paths
            for path in wav_files.iter().chain(ogg_files.iter()) {
                // Find the theme.toml file *before* deleting the file
                if let Some(parent) = path.parent() {
                    let toml_path = parent.join("theme.toml");
                    if toml_path.exists() {
                        toml_paths.insert(toml_path);
                    }
                }

                // Delete the file
                if fs::remove_file(path).is_ok() {
                    delete_count += 1;
                } else {
                    eprintln!("[WARN] Failed to delete file: {}", path.display());
                }
            }

            // Now, update all collected theme.toml files
            for toml_path in toml_paths {
                if let Ok(content) = fs::read_to_string(&toml_path) {
                    if let Ok(mut theme_data) = toml::from_str::<ThemeToml>(&content) {
                        // Set bgm_track to None (which serializes as it being removed or null)
                        theme_data.bgm_track = None;

                        // Reserialize and write back
                        if let Ok(new_content) = toml::to_string(&theme_data) {
                            let _ = fs::write(toml_path, new_content);
                        }
                    }
                }
            }

            Ok(format!("Successfully deleted {} BGM file(s)!", delete_count))
        })();

        // Send the result back, whether Ok or Err
        tx.send(DownloaderMessage::ConversionResult(result)).unwrap_or_default();
    });
}
