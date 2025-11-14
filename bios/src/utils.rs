use macroquad::prelude::*;
use rodio::{buffer::SamplesBuffer, Sink};
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;
use chrono::Local;
use crate::{save, Child, Arc, Mutex, thread, BufReader};
use crate::audio::play_new_bgm;
use crate::types::Screen;
//use macroquad::audio::Sound;

// wrap text in certain menus so it doesn't clip outside the screen
pub fn wrap_text(text: &str, font: Font, font_size: u16, max_width: f32) -> Vec<String> {
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

/// Scans a directory and returns a sorted list of paths for files with given extensions.
pub fn find_asset_files(dir_path: &str, extensions: &[&str]) -> Vec<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir_path) {
        let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|path| {
            path.is_file() &&
            path.extension()
            .and_then(|s| s.to_str())
            .map_or(false, |ext| extensions.contains(&ext))
        })
        .collect();
        files.sort();
        return files;
    }
    vec![]
}

// Helper to read the first line from a file containing a specific key
pub fn read_line_from_file(path: &str, key: &str) -> Option<String> {
    fs::read_to_string(path).ok()?.lines()
    .find(|line| line.starts_with(key))
    .map(|line| line.replace(key, "").trim().to_string())
}

/// Calls a privileged helper script to copy session logs to the SD card.
pub fn copy_session_logs_to_sd() -> Result<String, String> {
    let output = Command::new("sudo")
    .arg("/usr/bin/kazeta-copy-logs")
    .output()
    .map_err(|e| format!("Failed to execute helper script: {}", e))?;

    if output.status.success() {
        // The script prints the destination path on success, so we can capture it.
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Find the last line of output that contains the path
        let path_line = stdout.lines().last().unwrap_or("Log copy successful.");
        Ok(path_line.to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Log copy script failed: {}", stderr.trim()))
    }
}

// FOR ACTUAL HARDWARE USE
pub fn trigger_session_restart(
    //current_bgm: &mut Option<Sound>,
    //music_cache: &HashMap<String, Sound>,
    current_bgm: &mut Option<Sink>,
    music_cache: &HashMap<String, SamplesBuffer>,
) -> (Screen, Option<f64>) {
    // Stop the BGM
    play_new_bgm("OFF", 0.0, music_cache, current_bgm);

    // Create the sentinel file at the correct system path
    let sentinel_path = Path::new("/var/kazeta/state/.RESTART_SESSION_SENTINEL");
    if let Some(parent) = sentinel_path.parent() {
        // Ensure the directory exists
        if fs::create_dir_all(parent).is_ok() {
            let _ = fs::File::create(sentinel_path);
        }
    }

    // Return the state to begin the fade-out
    (Screen::FadingOut, Some(get_time()))
}

pub fn trigger_game_launch(
    _cart_info: &save::CartInfo,
    kzi_path: &Path,
    //current_bgm: &mut Option<Sound>,
    //music_cache: &HashMap<String, Sound>,
    current_bgm: &mut Option<Sink>,
    music_cache: &HashMap<String, SamplesBuffer>,
) -> (Screen, Option<f64>) {
    // Write the specific launch command for the selected game
    if let Err(e) = save::write_launch_command(kzi_path) {
        // If we fail, we should probably show an error on the debug screen
        // For now, we'll just print it for desktop debugging.
        println!("[ERROR] Failed to write launch command: {}", e);
    }

    // Now, trigger the standard session restart process,
    // which will find and execute our command file.
    trigger_session_restart(current_bgm, music_cache)
}

pub fn save_log_to_file(log_messages: &[String]) -> std::io::Result<String> {
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let filename = format!("kazeta_log_{}.log", timestamp);

    // In a real application, you'd save this to a logs directory.
    // For now, it will save in the same directory as the executable.
    fs::write(&filename, log_messages.join("\n"))?;

    println!("Log saved to {}", filename);
    Ok(filename)
}

pub fn start_log_reader(process: &mut Child, logs: Arc<Mutex<Vec<String>>>) {
    // Take ownership of the output pipes
    if let (Some(stdout), Some(stderr)) = (process.stdout.take(), process.stderr.take()) {
        let logs_clone_stdout = logs.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().filter_map(|l| l.ok()) {
                logs_clone_stdout.lock().unwrap().push(line);
            }
        });

        let logs_clone_stderr = logs.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().filter_map(|l| l.ok()) {
                logs_clone_stderr.lock().unwrap().push(line);
            }
        });
    }
}

/// Removes the file extension from a filename string slice.
pub fn trim_extension(filename: &str) -> &str {
    if let Some(dot_index) = filename.rfind('.') {
        &filename[..dot_index]
    } else {
        filename
    }
}

pub fn string_to_color(color_str: &str) -> Color {
    match color_str {
        "BLACK" => BLACK,
        "PINK" => PINK,
        "RED" => RED,
        "ORANGE" => ORANGE,
        "YELLOW" => YELLOW,
        "GREEN" => GREEN,
        "BLUE" => BLUE,
        "PURPLE" => VIOLET, // USING VIOLET AS A CLOSE APPROXIMATION
        _ => WHITE, // Default to WHITE
    }
}

/// Parses a resolution string and requests a window resize.
pub fn apply_resolution(resolution_str: &str) {
    if let Some((w_str, h_str)) = resolution_str.split_once('x') {
        // Parse to f32 for the resize function
        if let (Ok(w), Ok(h)) = (w_str.parse::<f32>(), h_str.parse::<f32>()) {
            // Use the correct function name
            request_new_screen_size(w, h);
        }
    }
}
