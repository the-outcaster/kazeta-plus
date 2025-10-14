// In bios/src/ui/update_checker.rs

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
    Checking,
    UpToDate,
    UpdateAvailable(GithubRelease),
    Downloading,
    Error(String),
}

enum CheckerMessage {
    CheckComplete(Result<UpdateCheckResult, String>),
}

enum UpdateCheckResult {
    UpToDate,
    UpdateAvailable(GithubRelease),
}

pub struct UpdateCheckerState {
    pub screen_state: UpdateCheckerScreenState,
    rx: Receiver<CheckerMessage>,
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
        let (tx, rx) = channel();
        check_for_updates(tx);
        Self {
            screen_state: UpdateCheckerScreenState::Checking,
            rx,
        }
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
        *current_screen = Screen::MainMenu;
        sound_effects.play_back(config);
        return;
    }

    if let Ok(msg) = state.rx.try_recv() {
        match msg {
            CheckerMessage::CheckComplete(Ok(result)) => match result {
                UpdateCheckResult::UpToDate => state.screen_state = UpdateCheckerScreenState::UpToDate,
                UpdateCheckResult::UpdateAvailable(release) => state.screen_state = UpdateCheckerScreenState::UpdateAvailable(release),
            },
            CheckerMessage::CheckComplete(Err(e)) => state.screen_state = UpdateCheckerScreenState::Error(e),
        }
    }

    let mut release_to_install: Option<GithubRelease> = None;
    match &state.screen_state {
        UpdateCheckerScreenState::UpdateAvailable(release) => {
            if input_state.select {
                sound_effects.play_select(config);
                release_to_install = Some(release.clone());
            }
        }
        UpdateCheckerScreenState::UpToDate | UpdateCheckerScreenState::Error(_) => {
            if input_state.select {
                *current_screen = Screen::MainMenu;
                sound_effects.play_select(config);
            }
        }
        _ => {}
    }

    if let Some(release) = release_to_install {
        state.screen_state = UpdateCheckerScreenState::Downloading;
        thread::spawn(move || {
            perform_update(release);
        });
    }
}

pub fn draw(
    state: &UpdateCheckerState,
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
        UpdateCheckerScreenState::Checking => {
            let text = "Checking for updates...";
            let text_dims = measure_text(text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        UpdateCheckerScreenState::UpToDate => {
            text_with_config_color(font_cache, config, "You are running the latest version.", text_x, text_y_start, font_size);
            text_with_config_color(font_cache, config, &format!("Current version: {}", VERSION_NUMBER), text_x, text_y_start + line_height, font_size);
            text_with_config_color(font_cache, config, "Press Select or Back to return.", text_x, text_y_start + line_height * 3.0, font_size);
        }
        UpdateCheckerScreenState::UpdateAvailable(release) => {
            // --- FIX 1: Use release.tag_name for the version number ---
            text_with_config_color(font_cache, config, &format!("New version available: {}", release.tag_name), text_x, text_y_start, font_size);
            text_with_config_color(font_cache, config, &format!("Current version: {}", VERSION_NUMBER), text_x, text_y_start + line_height, font_size);

            let separator_y = text_y_start + line_height * 2.5;
            draw_line(container_x, separator_y, container_x + container_w, separator_y, 2.0, Color::new(1.0, 1.0, 1.0, 0.2));

            // --- FIX 2: Add a regex to strip Markdown links ---
            let img_tag_regex = Regex::new(r"<img[^>]*>").unwrap();
            let md_link_regex = Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap(); // Matches [text](url)

            let no_images = img_tag_regex.replace_all(&release.body, "");
            let clean_body = md_link_regex.replace_all(&no_images, "$1"); // Replaces the link with just the text part

            let wrap_width = container_w - 60.0 * scale_factor;
            let wrapped_lines = wrap_text(clean_body.trim(), font.clone(), font_size, wrap_width);
            for (i, line) in wrapped_lines.iter().enumerate() {
                text_with_config_color(font_cache, config, line, text_x, separator_y + 40.0 * scale_factor + (i as f32 * line_height), font_size);
            }

            let continue_text = "Press A to Install Update";
            let continue_dims = measure_text(continue_text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, continue_text, screen_width() / 2.0 - continue_dims.width / 2.0, container_y + container_h - 40.0 * scale_factor, font_size);
        }
        UpdateCheckerScreenState::Downloading => {
            let text = "Downloading update... The app will close automatically.";
            let text_dims = measure_text(text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        UpdateCheckerScreenState::Error(msg) => {
            text_with_config_color(font_cache, config, "An error occurred:", text_x, text_y_start, font_size);
            text_with_config_color(font_cache, config, msg, text_x, text_y_start + line_height, font_size);
            text_with_config_color(font_cache, config, "Press Select or Back to return.", text_x, text_y_start + line_height * 3.0, font_size);
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

fn perform_update(release_info: GithubRelease) {
    let update_asset = match release_info.assets.iter().find(|asset| asset.name.ends_with(".zip")) {
        Some(asset) => asset,
        None => { eprintln!("Error: No .zip asset found in the latest release."); return; }
    };
    let tmp_zip_path = Path::new("/tmp/kazeta-update.zip");

    let response_bytes = reqwest::blocking::get(&update_asset.browser_download_url).expect("Failed to download update file.").bytes().expect("Failed to read response bytes.");
    let mut tmp_file = fs::File::create(&tmp_zip_path).expect("Failed to create temp file.");
    tmp_file.write_all(&response_bytes).expect("Failed to save update file.");

    // 1. Change the extraction directory to /tmp/
    let tmp_extract_dir = Path::new("/tmp/");

    // 2. Get the kit's directory name *before* extraction
    let root_dir_name = update_asset.name.strip_suffix(".zip").unwrap_or(&update_asset.name);

    // 3. Define the *full path* to the kit directory
    let kit_path = tmp_extract_dir.join(root_dir_name);

    // 4. SAFELY remove the *specific* kit directory if it already exists
    if kit_path.exists() {
        fs::remove_dir_all(&kit_path).unwrap_or_else(|e| {
            eprintln!("Failed to remove old kit directory: {}", e);
        });
    }

    // 5. We no longer create_dir_all, since /tmp/ exists
    //    and the extractor will create the nested folder.
    extract_archive(&tmp_zip_path, &tmp_extract_dir);

    // 6. This line now correctly resolves to the new path:
    //    e.g., /tmp/kazeta-plus-upgrade-kit-1.11/upgrade-to-plus.sh
    let script_path = tmp_extract_dir.join(root_dir_name).join("upgrade-to-plus.sh");

    if !script_path.exists() { eprintln!("Error: upgrade-to-plus.sh not found in the archive."); return; }

    let mut perms = fs::metadata(&script_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("Failed to set script permissions.");

    Command::new("sudo")
        .arg(script_path)
        .spawn()
        .expect("Failed to start upgrade script.");
    exit(0);
}

fn extract_archive(archive_path: &Path, destination: &Path) {
    let file = fs::File::open(archive_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = destination.join(file.enclosed_name().unwrap());
        if (*file.name()).ends_with('/') { fs::create_dir_all(&outpath).unwrap(); }
        else {
            if let Some(p) = outpath.parent() { if !p.exists() { fs::create_dir_all(&p).unwrap(); } }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }
}
