use macroquad::prelude::*;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, exit};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use crate::{
    audio::SoundEffects,
    config::Config,
    FONT_SIZE, VERSION_NUMBER, Screen, BackgroundState, render_background, get_current_font, text_with_config_color, InputState, wrap_text,
};

// --- State Management & Structs ---

pub enum UpdateCheckerScreenState {
    Idle,
    Checking,
    UpToDate,
    UpdateAvailable(GithubRelease),
    InProgress(String), // carries status message
    UpdateComplete, // final screen before shutdown
    Error(String),
}

enum CheckerMessage {
    CheckComplete(Result<UpdateCheckResult, String>),
}

// A new message type for the update thread to send progress back to the UI.
enum UpdateProgressMessage {
    Status(String),
    Complete,
    Error(String),
}


enum UpdateCheckResult {
    UpToDate,
    UpdateAvailable(GithubRelease),
}

pub struct UpdateCheckerState {
    pub screen_state: UpdateCheckerScreenState,
    rx_check: Receiver<CheckerMessage>,
    rx_progress: Receiver<UpdateProgressMessage>,
    pub description_scroll_offset: usize,
    pub max_description_scroll: usize,
}

#[derive(Deserialize, Clone, Debug)]
pub struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct GithubRelease {
    pub tag_name: String,
    pub body: String,
    pub assets: Vec<GithubAsset>,
}

// --- Implementation ---

impl UpdateCheckerState {
    pub fn new() -> Self {
        let (_tx_check, rx_check) = channel(); // Use specific names
        let (_tx_progress, rx_progress) = channel(); // Create a dummy channel for now
        Self {
            screen_state: UpdateCheckerScreenState::Idle,
            rx_check,
            rx_progress,
            description_scroll_offset: 0,
            max_description_scroll: 0,
        }
    }

    fn start_check(&mut self) {
        let (tx, rx) = channel();
        check_for_updates(tx);
        self.screen_state = UpdateCheckerScreenState::Checking;
        self.rx_check = rx; // Overwrite the old receiver
        self.description_scroll_offset = 0; // Reset scroll on new check
        self.max_description_scroll = 0;
    }
}

pub fn update(
    state: &mut UpdateCheckerState,
    input_state: &InputState,
    current_screen: &mut Screen,
    sound_effects: &SoundEffects,
    config: &Config,
) {
    if input_state.back {
        *current_screen = Screen::Extras;
        state.screen_state = UpdateCheckerScreenState::Idle; // <-- RESET STATE
        sound_effects.play_back(config);
        return;
    }

    if let Ok(msg) = state.rx_check.try_recv() {
        match msg {
            CheckerMessage::CheckComplete(Ok(result)) => match result {
                UpdateCheckResult::UpToDate => state.screen_state = UpdateCheckerScreenState::UpToDate,
                UpdateCheckResult::UpdateAvailable(release) => state.screen_state = UpdateCheckerScreenState::UpdateAvailable(release),
            },
            CheckerMessage::CheckComplete(Err(e)) => state.screen_state = UpdateCheckerScreenState::Error(e),
        }
    }

    // Receive messages from the update progress thread
    if let Ok(msg) = state.rx_progress.try_recv() {
        match msg {
            UpdateProgressMessage::Status(text) => {
                state.screen_state = UpdateCheckerScreenState::InProgress(text);
            }
            UpdateProgressMessage::Complete => {
                state.screen_state = UpdateCheckerScreenState::UpdateComplete;
            }
            UpdateProgressMessage::Error(e) => {
                state.screen_state = UpdateCheckerScreenState::Error(e);
            }
        }
    }

    // If we're idle, start a check. This triggers on entering the screen.
    if let UpdateCheckerScreenState::Idle = state.screen_state {
        state.start_check();
    }

    let mut release_to_install: Option<GithubRelease> = None;
    match &state.screen_state {
        UpdateCheckerScreenState::UpdateAvailable(release) => {
            if input_state.select {
                sound_effects.play_select(config);
                release_to_install = Some(release.clone());
            }

            // Handle up/down for scrolling the description text
            if input_state.down {
                // Check against the max value calculated in the previous frame
                if state.description_scroll_offset < state.max_description_scroll {
                    state.description_scroll_offset += 1;
                    sound_effects.play_cursor_move(config);
                }
            }
            if input_state.up {
                if state.description_scroll_offset > 0 {
                    state.description_scroll_offset -= 1;
                    sound_effects.play_cursor_move(config);
                }
            }
        }
        UpdateCheckerScreenState::UpdateComplete => {
            // SOUTH button for shutdown
            if input_state.select {
                sound_effects.play_select(config);
                Command::new("sudo").arg("shutdown").arg("now").status().ok();
                exit(0); // Fallback in case shutdown command fails
            }
            // WEST button for reboot
            if input_state.secondary {
                sound_effects.play_select(config);
                Command::new("sudo").arg("reboot").status().ok();
                exit(0); // Fallback in case reboot command fails
            }
        }
        UpdateCheckerScreenState::UpToDate | UpdateCheckerScreenState::Error(_) => {
            if input_state.select {
                *current_screen = Screen::MainMenu;
                state.screen_state = UpdateCheckerScreenState::Idle; // <-- RESET STATE
                sound_effects.play_select(config);
            }
        }
        _ => {}
    }

    if let Some(release) = release_to_install {
        // Create a new channel and pass the sender to the thread
        let (tx_progress, rx_progress) = channel();
        state.rx_progress = rx_progress; // Hook up the new receiver

        // Start in the InProgress state
        state.screen_state = UpdateCheckerScreenState::InProgress("Starting update...".to_string());

        thread::spawn(move || {
            // We now check the result of the update logic.
            // If it fails, we send the error string back to the UI.
            if let Err(e) = perform_update_logic(release, tx_progress.clone()) {
                // Use unwrap_or_default() in case the UI is already closed
                tx_progress.send(UpdateProgressMessage::Error(e)).unwrap_or_default();
            }
        });
    }
}

pub fn draw(
    state: &mut UpdateCheckerState,
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
    let container_w = screen_width() * 0.9;
    let container_h = screen_height() * 0.8;
    let container_x = (screen_width() - container_w) / 2.0;
    let container_y = (screen_height() - container_h) / 2.0;
    draw_rectangle(container_x, container_y, container_w, container_h, Color::new(0.0, 0.0, 0.0, 0.75));
    let text_x = container_x + 30.0 * scale_factor;
    let text_y_start = container_y + 40.0 * scale_factor;

    match &state.screen_state {
        UpdateCheckerScreenState::Idle => {
            let text = "Connecting to update server...";
            let text_dims = measure_text(text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        UpdateCheckerScreenState::Checking => {
            let text = "Checking for updates...";
            let text_dims = measure_text(text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        UpdateCheckerScreenState::UpToDate => {
            text_with_config_color(font_cache, config, "You are running the latest version.", text_x, text_y_start, font_size);
            text_with_config_color(font_cache, config, &format!("Current version: {}", VERSION_NUMBER), text_x, text_y_start + line_height, font_size);
            text_with_config_color(font_cache, config, "Press [SOUTH] or [EAST] to return.", text_x, text_y_start + line_height * 3.0, font_size);
        }
        UpdateCheckerScreenState::UpdateAvailable(release) => {
            text_with_config_color(font_cache, config, &format!("New version available: {}", release.tag_name), text_x, text_y_start, font_size);
            text_with_config_color(font_cache, config, &format!("Current version: {}", VERSION_NUMBER), text_x, text_y_start + line_height, font_size);

            let separator_y = text_y_start + line_height * 2.5;
            draw_line(container_x, separator_y, container_x + container_w, separator_y, 2.0, Color::new(1.0, 1.0, 1.0, 0.2));

            // -- CHANGED -- Implemented scrolling logic
            let img_tag_regex = Regex::new(r"<img[^>]*>").unwrap();
            let md_link_regex = Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap();

            let no_images = img_tag_regex.replace_all(&release.body, "");
            let clean_body = md_link_regex.replace_all(&no_images, "$1");

            let wrap_width = container_w - 60.0 * scale_factor;
            let wrapped_lines = wrap_text(clean_body.trim(), font.clone(), font_size, wrap_width);

            let description_area_top = separator_y + 30.0 * scale_factor;
            let description_area_bottom = container_y + container_h - 30.0 * scale_factor;
            let visible_lines = ((description_area_bottom - description_area_top) / line_height).floor() as usize;

            let max_scroll_offset = if wrapped_lines.len() > visible_lines { wrapped_lines.len() - visible_lines } else { 0 };

            // Clamp the scroll offset to prevent scrolling past the end
            state.description_scroll_offset = state.description_scroll_offset.min(max_scroll_offset);

            state.max_description_scroll = max_scroll_offset;

            // Draw the visible lines of text
            for (i, line) in wrapped_lines.iter().skip(state.description_scroll_offset).take(visible_lines).enumerate() {
                text_with_config_color(font_cache, config, line, text_x, description_area_top + (i as f32 * line_height), font_size);
            }

            // Draw scroll indicators if needed
            if max_scroll_offset > 0 {
                let indicator_x = container_x + container_w - 20.0 * scale_factor;
                let arrow_size = 4.0 * scale_factor;

                // Calculate the vertical center of the first and last lines
                let first_line_center_y = description_area_top + (line_height / 2.0) - 40.0;
                let last_line_center_y = description_area_bottom - (line_height / 2.0) - 40.0;

                // Up arrow - Aligned with the first line of text
                if state.description_scroll_offset > 0 {
                    draw_triangle(
                        vec2(indicator_x, first_line_center_y - arrow_size),
                        vec2(indicator_x - arrow_size, first_line_center_y + arrow_size),
                        vec2(indicator_x + arrow_size, first_line_center_y + arrow_size),
                        WHITE
                    );
                }
                // Down arrow - Aligned with the last line of text
                if state.description_scroll_offset < max_scroll_offset {
                    draw_triangle(
                        vec2(indicator_x, last_line_center_y + arrow_size),
                        vec2(indicator_x - arrow_size, last_line_center_y - arrow_size),
                        vec2(indicator_x + arrow_size, last_line_center_y - arrow_size),
                        WHITE
                    );
                }
            }

            let continue_text = "Press [SOUTH] to Install Update";
            let continue_dims = measure_text(continue_text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, continue_text, screen_width() / 2.0 - continue_dims.width / 2.0, container_y + container_h - 20.0 * scale_factor, font_size);
        }
        UpdateCheckerScreenState::InProgress(message) => {
            let text_dims = measure_text(message, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, message, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        UpdateCheckerScreenState::UpdateComplete => {
            let line1 = "Update Complete!";
            let line2 = "Press [SOUTH] to shut down, or [WEST] to reboot.";

            let dims1 = measure_text(line1, Some(font), font_size, 1.0);
            let dims2 = measure_text(line2, Some(font), font_size, 1.0);

            text_with_config_color(font_cache, config, line1, screen_width() / 2.0 - dims1.width / 2.0, screen_height() / 2.0 - line_height, font_size);
            text_with_config_color(font_cache, config, line2, screen_width() / 2.0 - dims2.width / 2.0, screen_height() / 2.0, font_size);
        }
        UpdateCheckerScreenState::Error(msg) => {
            text_with_config_color(font_cache, config, "An error occurred:", text_x, text_y_start, font_size);
            text_with_config_color(font_cache, config, msg, text_x, text_y_start + line_height, font_size);
            text_with_config_color(font_cache, config, "Press [SOUTH] or [EAST] to return.", text_x, text_y_start + line_height * 3.0, font_size);
        }
    }
}

// --- Background Thread Functions ---

fn check_for_updates(tx: Sender<CheckerMessage>) {
    thread::spawn(move || {
        let client = match reqwest::blocking::Client::builder().user_agent("KazetaPlus-Updater").build() {
            Ok(c) => c,
                  Err(e) => { tx.send(CheckerMessage::CheckComplete(Err(e.to_string()))).unwrap(); return; }
        };

        let response = client.get("https://api.github.com/repos/the-outcaster/kazeta-plus/releases").send();

        let result = match response {
            Ok(resp) => if resp.status().is_success() {
                match resp.json::<Vec<GithubRelease>>() {
                    Ok(releases) => {
                        if let Some(latest_release) = releases.get(0) { // No need for mut here
                            if latest_release.tag_name != VERSION_NUMBER {
                                Ok(UpdateCheckResult::UpdateAvailable(latest_release.clone()))
                            } else {
                                Ok(UpdateCheckResult::UpToDate)
                            }
                        } else {
                            Ok(UpdateCheckResult::UpToDate)
                        }
                    }
                    Err(e) => Err(format!("Failed to parse response: {}", e)),
                }
            } else {
                Err(format!("GitHub API Error: {}", resp.status()))
            },
            Err(e) => Err(format!("Failed to fetch from GitHub: {}", e)),
        };
        tx.send(CheckerMessage::CheckComplete(result)).unwrap();
    });
}

// This function now returns a Result, so we can catch all errors
fn perform_update_logic(release_info: GithubRelease, tx: Sender<UpdateProgressMessage>) -> Result<(), String> {
    let update_asset = match release_info.assets.iter().find(|asset| asset.name.ends_with(".zip")) {
        Some(asset) => asset,
        None => return Err("No .zip asset found in the release.".to_string()),
    };

    tx.send(UpdateProgressMessage::Status("Downloading update...".to_string())).map_err(|e| e.to_string())?;

    // download
    let tmp_zip_path = Path::new("/tmp/kazeta-update.zip");

    let response = reqwest::blocking::get(&update_asset.browser_download_url)
    .map_err(|e| format!("Download failed: {}", e))?;
    let response_bytes = response.bytes().map_err(|e| format!("Failed to read bytes: {}", e))?;

    let mut tmp_file = fs::File::create(&tmp_zip_path)
    .map_err(|e| format!("Failed to create temp file: {}", e))?;
    tmp_file.write_all(&response_bytes)
    .map_err(|e| format!("Failed to save update file: {}", e))?;

    // extraction
    tx.send(UpdateProgressMessage::Status("Extracting archive...".to_string())).map_err(|e| e.to_string())?;

    let tmp_extract_dir = Path::new("/tmp/");

    // Call the new, safer extract_archive function
    extract_archive(&tmp_zip_path, &tmp_extract_dir)?;

    let script_path = tmp_extract_dir.join("upgrade-to-plus.sh");

    // Add a log to see the exact path being checked
    println!("[UPDATE_AGENT] Checking for script at: {}", script_path.display());

    // Send an error instead of silently returning
    if !script_path.exists() {
        let error_msg = format!("Script not found at: {}", script_path.display());
        eprintln!("[UPDATE_AGENT] {}", error_msg);
        return Err(error_msg);
    }

    // Manually set the executable permission for the script.
    println!("[UPDATE_AGENT] Setting executable permission on upgrade script...");

    // Check metadata and set permissions safely
    let metadata = fs::metadata(&script_path)
    .map_err(|e| format!("Failed to read script metadata: {}", e))?;
    let mut perms = metadata.permissions();

    // Set permissions to rwxr-xr-x (0755)
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms)
    .map_err(|e| format!("Failed to set permissions: {}", e))?;

    println!("[UPDATE_AGENT] Permissions set. Executing script...");
    tx.send(UpdateProgressMessage::Status("Applying update... Do not turn off.".to_string())).map_err(|e| e.to_string())?;

    let status = Command::new("sudo")
    .arg(script_path)
    .status()
    .map_err(|e| format!("Failed to run upgrade script: {}", e))?;

    if !status.success() {
        return Err(format!("Upgrade script failed with status: {}", status));
    }

    // Send "Complete" message and let the thread finish
    tx.send(UpdateProgressMessage::Complete).map_err(|e| e.to_string())?;

    Ok(())
}

fn extract_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = fs::File::open(archive_path)
    .map_err(|e| format!("Failed to open zip: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
    .map_err(|e| format!("Failed to read zip: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
        .map_err(|e| format!("Failed to read zip entry {}: {}", i, e))?;

        let outpath = match file.enclosed_name() {
            Some(path) => destination.join(path),
            None => return Err(format!("Invalid file path in zip entry {}", i)),
        };

        if (*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath)
            .map_err(|e| format!("Failed to create dir: {}", e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)
                    .map_err(|e| format!("Failed to create parent dir: {}", e))?;
                }
            }
            let mut outfile = fs::File::create(&outpath)
            .map_err(|e| format!("Failed to create file: {}", e))?;
            io::copy(&mut file, &mut outfile)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        }
    }
    Ok(())
}
