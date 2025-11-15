use macroquad::prelude::*;
use serde::Deserialize;
use std::{
    fs, thread,
    collections::{HashMap, HashSet},
    sync::mpsc::{channel, Receiver, Sender},
    time::{Duration, Instant},
    io::{self, Read, Write},
    path::PathBuf,
};

use crate::{
    audio::SoundEffects,
    config::{Config, get_user_data_dir},
    FONT_SIZE, Screen, BackgroundState, render_background, get_current_font, text_with_config_color, InputState, wrap_text, DEV_MODE,
};

// --- CONSTANTS ---
const ITEMS_PER_PAGE: usize = 5;

// --- State Management & Structs ---

/// Represents the source of the runtime for categorization
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimeSource {
    Official,   // From kazetaos/kazeta
    Outcaster,  // From the-outcaster/kazeta-plus (kzr)
    ThirdParty, // From the-outcaster/kazeta-plus (zip)
}

#[derive(Debug, Clone)]
pub struct RemoteRuntime {
    pub name: String,         // Display name, e.g., "psx.kzr" or "pcengine.zip"
    pub file_name: String,    // The file to download, e.g., "psx.kzr" or "pcengine.zip"
    pub description: String,
    pub download_url: String,
    pub source: RuntimeSource,
    pub is_installed: bool,
    pub is_zip: bool,         // True if this is a zip archive that needs extraction
    pub size_mb: Option<f32>,
}

pub enum DownloaderState {
    Idle,
    FetchingList,
    DisplayingList,
    Downloading {
        name: String,
        progress: Option<f32>, // 0.0 to 1.0, None if size unknown
        received_mb: f32,
    },
    Success(String),
    Error(String),
    ConfirmDelete {
        runtime: RemoteRuntime, // Pass the whole object
        selection: usize,       // 0=Yes, 1=No
    },
    ConfirmRedownload {
        runtime: RemoteRuntime,
        selection: usize,       // 0=Yes, 1=No
    },
}

enum DownloaderMessage {
    RuntimeList(Result<Vec<RemoteRuntime>, String>),
    DownloadProgress {
        progress: Option<f32>,
        received_mb: f32,
    },
    InstallResult(Result<String, String>),
    DeleteResult(Result<String, String>),
}

pub struct RuntimeDownloaderState {
    pub screen_state: DownloaderState,
    pub runtimes: Vec<RemoteRuntime>,
    pub selected_index: usize,
    rx: Receiver<DownloaderMessage>,
    tx: Sender<DownloaderMessage>,
    pub current_page: usize,
}

// Structs for parsing GitHub API responses
#[derive(Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    assets: Vec<GithubReleaseAsset>,
}

// --- Implementation ---

impl RuntimeDownloaderState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            screen_state: DownloaderState::Idle,
            runtimes: Vec::new(),
            selected_index: 0,
            rx,
            tx,
            current_page: 0,
        }
    }

    fn start_fetch(&mut self) {
        fetch_runtime_list(self.tx.clone());
        self.screen_state = DownloaderState::FetchingList;
    }
}

/// Returns the correct runtime directory based on dev mode
fn get_runtime_dir() -> PathBuf {
    if DEV_MODE {
        // Dev path: ~/.local/share/kazeta-plus/runtimes/
        get_user_data_dir().unwrap_or_else(|| PathBuf::from(".")).join("runtimes")
    } else {
        // Prod path: /usr/share/kazeta/runtimes/
        PathBuf::from("/usr/share/kazeta/runtimes")
    }
}

/// Scans the runtime directory for installed files and extracted folders.
fn get_installed_runtime_files() -> HashSet<String> {
    let runtimes_dir = get_runtime_dir();
    if let Ok(entries) = fs::read_dir(runtimes_dir) {
        return entries.flatten()
        .filter_map(|entry| {
            let path = entry.path();
            // Check for both files (e.g., "psx.kzr") and directories (e.g., "pcengine")
            if path.is_file() || path.is_dir() {
                entry.file_name().into_string().ok()
            } else {
                None
            }
        })
        .collect();
    }
    HashSet::new()
}


pub fn update(
    state: &mut RuntimeDownloaderState,
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
            DownloaderState::Downloading { .. } => {
                // NOTE: This doesn't cancel the thread, but just returns to the list.
                // The thread will error when it tries to send its next message.
                // This is acceptable for this use case.
                state.screen_state = DownloaderState::Idle; // Go to Idle to force a list refresh
                *current_screen = Screen::Extras;
            }
            _ => { // For any sub-menu, go back to the list
                state.screen_state = DownloaderState::DisplayingList;
                // Restore current page based on selection
                state.current_page = state.selected_index / ITEMS_PER_PAGE;
            }
        }
        return;
    }

    if let Ok(msg) = state.rx.try_recv() {
        match msg {
            DownloaderMessage::RuntimeList(Ok(runtimes)) => {
                state.runtimes = runtimes;
                state.screen_state = DownloaderState::DisplayingList;
            }
            DownloaderMessage::RuntimeList(Err(e)) => {
                state.screen_state = DownloaderState::Error(e);
            }
            DownloaderMessage::DownloadProgress { progress, received_mb } => {
                // Only update if we are still in the Downloading state
                if let DownloaderState::Downloading { progress: p, received_mb: mb, .. } = &mut state.screen_state {
                    *p = progress;
                    *mb = received_mb;
                }
            }
            DownloaderMessage::InstallResult(Ok(runtime_name)) => {
                state.screen_state = DownloaderState::Success(format!("'{}' installed!", runtime_name));
                // Set to Idle to trigger a re-fetch, which will update the [INSTALLED] status
                state.screen_state = DownloaderState::Idle;
            }
            DownloaderMessage::InstallResult(Err(e)) => {
                state.screen_state = DownloaderState::Error(e);
            }
            DownloaderMessage::DeleteResult(Ok(name)) => {
                state.screen_state = DownloaderState::Success(format!("'{}' deleted.", name));
                // Set to Idle to trigger a re-fetch
                state.screen_state = DownloaderState::Idle;
            }
            DownloaderMessage::DeleteResult(Err(e)) => {
                state.screen_state = DownloaderState::Error(e);
            }
        }
    }

    // if the screen is idle, trigger a new fetch.
    if let DownloaderState::Idle = state.screen_state {
        state.start_fetch();
    }

    match &mut state.screen_state {
        DownloaderState::DisplayingList => {
            let total_options = state.runtimes.len();
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
                if state.selected_index < state.runtimes.len() {
                    let runtime = state.runtimes[state.selected_index].clone();

                    if runtime.is_installed {
                        // Runtime is already installed, show confirmation
                        state.screen_state = DownloaderState::ConfirmRedownload {
                            runtime,
                            selection: 1, // Default to "NO"
                        };
                    } else {
                        // Not installed, download immediately
                        state.screen_state = DownloaderState::Downloading {
                            name: runtime.name.clone(),
                            progress: Some(0.0), // Start at 0%
                            received_mb: 0.0,
                        };
                        download_and_install_runtime(runtime, state.tx.clone());
                    }
                }
            }
            // Handle delete
            if input_state.secondary && state.selected_index < state.runtimes.len() {
                let runtime_to_delete = &state.runtimes[state.selected_index];

                // Only allow deletion if the runtime is installed
                if runtime_to_delete.is_installed {
                    sound_effects.play_select(config); // Or a "delete" sound
                    state.screen_state = DownloaderState::ConfirmDelete {
                        runtime: runtime_to_delete.clone(),
                        selection: 1, // Default to "NO"
                    };
                } else {
                    // Play reject sound if runtime is not installed
                    sound_effects.play_reject(config);
                }
            }
        }
        DownloaderState::ConfirmDelete { runtime, selection } => {
            if input_state.left || input_state.right { *selection = 1 - *selection; sound_effects.play_cursor_move(config); }
            if input_state.select {
                sound_effects.play_select(config);
                if *selection == 0 { // YES
                    let runtime_to_delete = runtime.clone();
                    state.screen_state = DownloaderState::Downloading {
                        name: format!("Deleting {}...", runtime_to_delete.name),
                        progress: None,
                        received_mb: 0.0,
                    };
                    delete_runtime(runtime_to_delete, state.tx.clone());
                } else { // NO
                    state.screen_state = DownloaderState::DisplayingList;
                }
            }
        }
        DownloaderState::ConfirmRedownload { runtime, selection } => {
            if input_state.left || input_state.right {
                *selection = 1 - *selection; // Flips between 0 (Yes) and 1 (No)
                sound_effects.play_cursor_move(config);
            }
            if input_state.select {
                sound_effects.play_select(config);
                if *selection == 0 { // User selected YES
                    let runtime_to_download = runtime.clone();
                    state.screen_state = DownloaderState::Downloading {
                        name: runtime_to_download.name.clone(),
                        progress: Some(0.0),
                        received_mb: 0.0,
                    };
                    download_and_install_runtime(runtime_to_download, state.tx.clone());
                } else { // User selected NO
                    state.screen_state = DownloaderState::DisplayingList;
                }
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
        DownloaderState::Downloading { .. } => {}
        _ => {}
    }
}

pub fn draw(
    state: &RuntimeDownloaderState,
    animation_state: &mut crate::AnimationState,
    background_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    background_state: &mut BackgroundState,
    scale_factor: f32,
) {
    render_background(background_cache, config, background_state);

    let font = get_current_font(font_cache, config);
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let line_height = font_size as f32 * 1.5;

    // Create a container for the UI
    let container_w = screen_width() * 0.9;
    let container_h = screen_height() * 0.8;
    let container_x = (screen_width() - container_w) / 2.0;
    let container_y = (screen_height() - container_h) / 2.0;

    // Don't draw container if downloading, it's a full-screen message
    if !matches!(state.screen_state, DownloaderState::Downloading { .. }) {
        draw_rectangle(container_x, container_y, container_w, container_h, Color::new(0.0, 0.0, 0.0, 0.75));
    }

    let text_x = container_x + 30.0 * scale_factor;
    let text_y_start = container_y + 40.0 * scale_factor;

    let center_x = screen_width() / 2.0;
    let center_y = screen_height() / 2.0;

    match &state.screen_state {
        DownloaderState::Idle => {
            let text = "Connecting to runtime repositories...";
            let text_dims = measure_text(text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        DownloaderState::FetchingList => {
            let text = "Fetching runtime list from GitHub...";
            let text_dims = measure_text(text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        DownloaderState::DisplayingList => {
            let total_options = state.runtimes.len();
            if total_options == 0 {
                text_with_config_color(font_cache, config, "No runtimes found.", text_x, text_y_start, font_size);
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

                let runtime = &state.runtimes[i];
                let installed_flag = if runtime.is_installed { " [INSTALLED]" } else { "" };

                // Add a prefix based on the source
                let source_prefix = match runtime.source {
                    RuntimeSource::Official => "[Official]",
                    RuntimeSource::Outcaster => "[Kazeta+]",
                    RuntimeSource::ThirdParty => "[Third-Party]",
                };

                // Format the size string
                let size_str = runtime.size_mb
                .map(|mb| format!(" ({:.1} MB)", mb)) // e.g., " (4.2 MB)"
                .unwrap_or_else(|| "".to_string()); // Show nothing if size is unknown

                let display_text = format!("{} {}{}{}", source_prefix, runtime.name, installed_flag, size_str);
                text_with_config_color(font_cache, config, &display_text, text_x, y_pos, font_size);
            }

            // Draw description panel
            let separator_y = text_y_start + (ITEMS_PER_PAGE as f32 * line_height) + (line_height / 2.0);
            draw_line(container_x, separator_y, container_x + container_w, separator_y, 2.0, Color::new(1.0, 1.0, 1.0, 0.2));

            let description_text = if state.selected_index < state.runtimes.len() {
                state.runtimes[state.selected_index].description.clone()
            } else {
                "".to_string() // Should be unreachable
            };

            let description_font_size = (font_size as f32 * 0.8) as u16;
            let description_line_height = description_font_size as f32 * 1.5;
            let wrap_width = container_w - 60.0 * scale_factor;

            let wrapped_lines = wrap_text(description_text.trim(), font.clone(), description_font_size, wrap_width);
            for (i, line) in wrapped_lines.iter().enumerate() {
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
        DownloaderState::ConfirmDelete { runtime, selection } => {
            let dialog_w = 400.0 * scale_factor;
            let dialog_h = 150.0 * scale_factor;
            let dialog_x = screen_width() / 2.0 - dialog_w / 2.0;
            let dialog_y = screen_height() / 2.0 - dialog_h / 2.0;
            draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::new(0.1, 0.1, 0.1, 0.9));
            draw_rectangle_lines(dialog_x, dialog_y, dialog_w, dialog_h, 3.0, WHITE);

            let question = format!("Delete '{}'?", runtime.name);
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
        DownloaderState::ConfirmRedownload { runtime, selection } => {
            let dialog_w = 500.0 * scale_factor;
            let dialog_h = 170.0 * scale_factor;
            let dialog_x = screen_width() / 2.0 - dialog_w / 2.0;
            let dialog_y = screen_height() / 2.0 - dialog_h / 2.0;
            draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::new(0.1, 0.1, 0.1, 0.9));
            draw_rectangle_lines(dialog_x, dialog_y, dialog_w, dialog_h, 3.0, WHITE);

            let question = format!("'{}' is already installed.", runtime.name);
            let question_dims = measure_text(&question, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, &question, screen_width() / 2.0 - question_dims.width / 2.0, dialog_y + 40.0 * scale_factor, font_size);

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
        DownloaderState::Downloading { name, progress, received_mb } => {
            // 1. Draw title text
            let text = format!("Downloading {}...", name);
            let text_dims = measure_text(&text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, &text, center_x - text_dims.width / 2.0, center_y - 60.0 * scale_factor, font_size);

            // 2. Define bar dimensions
            let bar_w = screen_width() * 0.6;
            let bar_h = 30.0 * scale_factor;
            let bar_x = center_x - bar_w / 2.0;
            let bar_y = center_y;

            // 3. Draw bar background (empty)
            draw_rectangle(bar_x, bar_y, bar_w, bar_h, BLACK);
            draw_rectangle_lines(bar_x, bar_y, bar_w, bar_h, 3.0, WHITE);

            // 4. Draw bar fill and progress text
            let progress_text: String;
            if let Some(p) = progress {
                // We have a percentage, draw the fill
                let fill_w = bar_w * p.clamp(0.0, 1.0); // Ensure fill is between 0% and 100%
                draw_rectangle(bar_x, bar_y, fill_w, bar_h, WHITE);
                progress_text = format!("{:.0}% ({:.1} MB)", p * 100.0, received_mb);
            } else {
                // No percentage (content-length unknown), just show MB
                // We can add a simple "cylon" scanner for visual feedback
                let scan_width = bar_w * 0.1_f32;
                let scan_pos = (get_time() as f32 * (bar_w * 0.5_f32)) % (bar_w - scan_width);
                draw_rectangle(bar_x + scan_pos as f32, bar_y, scan_width, bar_h, WHITE);
                progress_text = format!("Downloading... ({:.1} MB)", received_mb);
            }

            // Draw the progress text below the bar
            let text_dims = measure_text(&progress_text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, &progress_text, center_x - text_dims.width / 2.0, bar_y + bar_h + 40.0 * scale_factor, font_size);
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

// --- Background Thread Functions ---

/// Performs a HEAD request to get the file size in MB without downloading the body.
fn get_remote_file_size(client: &reqwest::blocking::Client, url: &str) -> Option<f32> {
    match client.head(url).send() {
        Ok(response) => {
            if response.status().is_success() {
                response.headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|val| val.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(|bytes| bytes as f32 / 1024.0 / 1024.0) // Convert bytes to MB
            } else {
                eprintln!("[FileSize] HEAD request failed for {}: {}", url, response.status());
                None
            }
        },
        Err(e) => {
            eprintln!("[FileSize] HEAD request error for {}: {}", url, e);
            None
        }
    }
}

fn get_zip_extracted_files(zip_name: &str) -> Vec<&str> {
    match zip_name {
        "pcengine-1.0.zip"    => vec!["pcengine-1.0.kzr", "pcengine-info.txt"],
        "playstation-1.01.zip" => vec!["playstation-1.01.kzr", "playstation-info.txt"],
        "saturn-1.0.zip"      => vec!["saturn-1.0.kzr", "saturn-info.txt"],
        "segacd-1.0.zip"      => vec!["segacd-1.0.kzr", "segacd-info.txt"],
        _ => vec![], // Unknown zip
    }
}

fn fetch_runtime_list(tx: Sender<DownloaderMessage>) {
    thread::spawn(move || {
        let mut all_runtimes: Vec<RemoteRuntime> = Vec::new();
        let client = reqwest::blocking::Client::builder()
        .user_agent("KazetaPlus-Runtime-Downloader")
        .build()
        .unwrap();

        // --- 1. Fetch Official Runtimes ---
        // From: https://github.com/kazetaos/kazeta/wiki/Runtimes
        // These are on the "latest" release page of the main repo.
        let official_base_url = "https://runtimes.kazeta.org/";
        let official_files = ["linux-1.0.kzr", "windows-1.0.kzr", "megadrive-1.1.kzr", "snes-1.0.kzr", "nes-1.0.kzr", "nintendo64-1.0.kzr"];

        for filename in official_files {
            let download_url = format!("{}{}", official_base_url, filename);
            // Get the file size
            let size_mb = get_remote_file_size(&client, &download_url);

            all_runtimes.push(RemoteRuntime {
                name: filename.to_string(),
                file_name: filename.to_string(),
                description: "Official Kazeta runtime.".to_string(),
                download_url: format!("{}{}", official_base_url, filename),
                source: RuntimeSource::Official,
                is_installed: false,
                is_zip: false,
                size_mb,
            });
        }

        // --- 2. Fetch Outcaster & Third-Party Runtimes ---
        // From: https://github.com/the-outcaster/kazeta-plus/releases/tag/runtimes
        let outcaster_files = ["dolphin-1.0.kzr", "linux-1.1.kzr", "windows-1.1.kzr"];
        let third_party_files = ["pcengine-1.0.zip", "playstation-1.01.zip", "saturn-1.0.zip", "segacd-1.0.zip"];
        let response_plus = client
        .get("https://api.github.com/repos/the-outcaster/kazeta-plus/releases/tags/runtimes")
        .send();

        if let Ok(resp) = response_plus {
            if let Ok(release) = resp.json::<GithubRelease>() {
                for asset in release.assets {
                    // Get the file size
                    let size_mb = get_remote_file_size(&client, &asset.browser_download_url);

                    if outcaster_files.contains(&asset.name.as_str()) {
                        all_runtimes.push(RemoteRuntime {
                            name: asset.name.clone(),
                            file_name: asset.name.clone(),
                            description: "Kazeta+ specific runtime.".to_string(),
                            download_url: asset.browser_download_url,
                            source: RuntimeSource::Outcaster,
                            is_installed: false,
                            is_zip: false,
                            size_mb,
                        });
                    } else if third_party_files.contains(&asset.name.as_str()) {
                        all_runtimes.push(RemoteRuntime {
                            name: asset.name.clone(),
                            file_name: asset.name.clone(),
                            description: "Third-party runtime pack. This will be extracted.".to_string(),
                            download_url: asset.browser_download_url,
                            source: RuntimeSource::ThirdParty,
                            is_installed: false,
                            is_zip: true,
                            size_mb,
                        });
                    }
                }
            } else {
                eprintln!("[Runtime] Failed to parse kazeta-plus releases JSON");
            }
        } else {
            eprintln!("[Runtime] Failed to fetch kazeta-plus releases");
        }

        // --- 3. Sort and Check Installation Status ---
        all_runtimes.sort_by_key(|r| (r.source.clone(), r.name.clone()));

        let installed_files = get_installed_runtime_files();
        for runtime in all_runtimes.iter_mut() {
            if runtime.is_zip {
                // Get the list of files this zip extracts
                let extracted_files = get_zip_extracted_files(&runtime.file_name);

                // We'll say it's "installed" if the *first* file (the .kzr) exists.
                if let Some(main_file) = extracted_files.get(0) {
                    runtime.is_installed = installed_files.contains(*main_file);
                }
            } else {
                // For .kzr files (e.g., "psx.kzr"), check for the file itself
                runtime.is_installed = installed_files.contains(&runtime.file_name);
            }
        }

        let result = if all_runtimes.is_empty() {
            Err("Failed to fetch any runtimes. Check internet connection.".to_string())
        } else {
            Ok(all_runtimes)
        };

        tx.send(DownloaderMessage::RuntimeList(result)).unwrap();
    });
}

fn download_and_install_runtime(runtime: RemoteRuntime, tx: Sender<DownloaderMessage>) {
    thread::spawn(move || {
        let result = (|| -> Result<String, String> {
            let runtimes_dir = get_runtime_dir();
            fs::create_dir_all(&runtimes_dir)
            .map_err(|e| format!("Failed to create runtime dir: {}", e))?;

            // Use a client that can handle streaming
            let client = reqwest::blocking::Client::new();
            let mut response = client.get(&runtime.download_url)
            .send()
            .map_err(|e| format!("Download failed: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("Download failed: Server returned {}", response.status()));
            }

            // Get total size for progress bar, if available
            let total_size = response.content_length(); // This is Option<u64>

            let mut received_bytes: u64 = 0;
            let mut response_bytes = Vec::new(); // Store the downloaded data here
            let mut buffer = [0; 8192]; // 8KB read buffer

            // Throttling for UI updates. Sending a message for every 8KB chunk
            // would flood the UI thread.
            let mut last_update = Instant::now();
            let update_interval = Duration::from_millis(50); // ~20 updates per second

            loop {
                // Read a chunk from the download stream
                let bytes_read = response.read(&mut buffer)
                .map_err(|e| format!("Failed to read download stream: {}", e))?;

                if bytes_read == 0 {
                    break; // Download complete
                }

                // Save the chunk to our main byte vector
                response_bytes.write_all(&buffer[..bytes_read])
                .map_err(|e| format!("Failed to write to in-memory buffer: {}", e))?;

                received_bytes += bytes_read as u64;

                // Send progress update (if throttled time has passed)
                if last_update.elapsed() >= update_interval {
                    let progress = total_size.map(|total| received_bytes as f32 / total as f32);
                    let received_mb = received_bytes as f32 / 1024.0 / 1024.0;

                    // Send the progress update. If this fails, the UI closed, so we abort.
                    if tx.send(DownloaderMessage::DownloadProgress { progress, received_mb }).is_err() {
                        return Err("Download cancelled: UI closed".to_string());
                    }
                    last_update = Instant::now();
                }
            }

            // Send one final, 100% update to make sure it looks complete
            let received_mb = received_bytes as f32 / 1024.0 / 1024.0;
            tx.send(DownloaderMessage::DownloadProgress { progress: Some(1.0), received_mb }).ok();

            // We have all the bytes. Now proceed as before.
            if runtime.is_zip {
                let reader = io::Cursor::new(response_bytes); // Use our Vec<u8>
                let mut archive = zip::ZipArchive::new(reader)
                .map_err(|e| format!("Invalid zip file: {}", e))?;
                archive.extract(&runtimes_dir)
                .map_err(|e| format!("Failed to extract zip: {}", e))?;
            } else {
                let target_path = runtimes_dir.join(&runtime.file_name);
                fs::write(target_path, response_bytes) // Use our Vec<u8>
                .map_err(|e| format!("Failed to save file: {}", e))?;
            }

            Ok(runtime.name)
        })();

        // Send the final result (Ok or Err)
        tx.send(DownloaderMessage::InstallResult(result)).unwrap_or_default();
    });
}

fn delete_runtime(runtime: RemoteRuntime, tx: Sender<DownloaderMessage>) {
    thread::spawn(move || {
        let result = (|| -> Result<String, String> {
            let runtimes_dir = get_runtime_dir();

            if runtime.is_zip {
                // It's a zip, so we delete all associated extracted files
                let files_to_delete = get_zip_extracted_files(&runtime.file_name);

                if files_to_delete.is_empty() {
                    return Err(format!("Unknown zip file: {}", runtime.name));
                }

                let mut deleted_count = 0;
                let mut last_error = Ok(());

                for file_name in files_to_delete {
                    let target_path = runtimes_dir.join(file_name);
                    if target_path.is_file() {
                        if let Err(e) = fs::remove_file(target_path) {
                            eprintln!("[Delete] Failed to delete file: {}", file_name);
                            last_error = Err(format!("Failed to delete {}: {}", file_name, e));
                            // We'll continue anyway, to try and delete the others
                        } else {
                            deleted_count += 1;
                        }
                    }
                }

                if deleted_count == 0 {
                    // If no files were deleted, it's an error.
                    // Return the specific error if one occurred,
                    // otherwise return a "not found" error.
                    match last_error {
                        Ok(_) => {
                            // No errors, but no files deleted.
                            return Err(format!("Extracted files for {} not found.", runtime.name));
                        }
                        Err(e) => {
                            // An error occurred during the loop. Return that error.
                            return Err(e);
                        }
                    }
                }
            } else {
                // It's a .kzr, so we delete the *file*
                let target_path = runtimes_dir.join(&runtime.file_name);

                if !target_path.is_file() {
                    return Err(format!("File {} not found.", runtime.file_name));
                }
                fs::remove_file(target_path)
                .map_err(|e| format!("Failed to delete file: {}", e))?;
            }

            Ok(runtime.name)
        })();
        tx.send(DownloaderMessage::DeleteResult(result)).unwrap();
    });
}
