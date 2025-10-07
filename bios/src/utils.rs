use macroquad::prelude::*;
use std::collections::HashMap;
use std::process::Command; // For the Command in AUDIO OUTPUT logic

// Import types and functions from your other modules
use crate::Regex;
use crate::config::{Config, save_config};
use crate::system::{adjust_system_volume, get_system_volume, set_brightness, get_current_brightness};
use crate::{AnimationState, AudioSink, BackgroundState, BatteryInfo, InputState, Screen, SoundEffects, FONT_SIZE, MENU_PADDING, SETTINGS_START_Y, SETTINGS_OPTION_HEIGHT};

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
