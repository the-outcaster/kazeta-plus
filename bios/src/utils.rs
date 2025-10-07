use macroquad::prelude::*;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;
use chrono::Local;
use crate::{save, Child, Arc, Mutex, thread, BufReader};
use crate::audio::play_new_bgm;
use crate::types::Screen;
use macroquad::audio::Sound;

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
// put log files in "logs" and backup existing files
pub fn copy_session_logs_to_sd() -> Result<String, String> {
    // 1. Find the SD card path
    let sd_card_path = match save::find_all_kzi_files() {
        Ok((paths, _)) => paths.get(0).and_then(|p| p.parent()).map(PathBuf::from),
        Err(e) => return Err(format!("SD card scan error: {}", e)),
    };
    let Some(base_path) = sd_card_path else {
        return Err("Could not locate SD card (no .kzi files found?).".to_string());
    };

    // 2. Define the 'logs' subdirectory and create it
    let dest_dir = base_path.join("logs");
    fs::create_dir_all(&dest_dir)
    .map_err(|e| format!("Failed to create logs dir: {}", e))?;

    // Force a filesystem sync to flush log buffers to disk
    Command::new("sync").status().map_err(|e| format!("Failed to run sync: {}", e))?;

    let source_files = ["session.log", "session.log.old"];
    let mut files_copied_count = 0;

    for filename in source_files {
        let source_file = Path::new("/var/kazeta/").join(filename);
        if source_file.exists() {
            let dest_file = dest_dir.join(filename);

            // 3. Check for an existing file at the destination to back it up
            if dest_file.exists() {
                let backup_file = dest_dir.join(format!("{}.bak", filename));
                // Use sudo to rename the existing log to a .bak file
                let mv_output = Command::new("sudo")
                .arg("mv")
                .arg(&dest_file)
                .arg(&backup_file)
                .output()
                .map_err(|e| format!("Failed to run sudo mv: {}", e))?;

                if !mv_output.status.success() {
                    let error_message = String::from_utf8_lossy(&mv_output.stderr);
                    return Err(format!("Failed to back up {}: {}", filename, error_message.trim()));
                }
            }

            // 4. Copy the new file using sudo
            let cp_output = Command::new("sudo")
            .arg("cp")
            .arg(&source_file)
            .arg(&dest_dir)
            .output()
            .map_err(|e| format!("Failed to run sudo cp: {}", e))?;

            if !cp_output.status.success() {
                let error_message = String::from_utf8_lossy(&cp_output.stderr);
                return Err(format!("Failed to copy {}: {}", filename, error_message.trim()));
            }
            files_copied_count += 1;
        }
    }

    if files_copied_count == 0 {
        return Err(format!("No log files found in /var/kazeta/"));
    }

    Ok(dest_dir.to_string_lossy().to_string())
}

// FOR ACTUAL HARDWARE USE
pub fn trigger_session_restart(
    current_bgm: &mut Option<Sound>,
    music_cache: &HashMap<String, Sound>,
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
    current_bgm: &mut Option<Sound>,
    music_cache: &HashMap<String, Sound>,
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
