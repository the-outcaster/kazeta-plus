use macroquad::prelude::*;
use std::collections::HashMap;
use std::process::Command;
use macroquad::audio;

// Import things from our new modules
use crate::config::{Config, save_config};
use crate::system::{adjust_system_volume, get_system_volume, set_brightness, get_current_brightness};
use crate::utils::{apply_resolution, trim_extension};
use crate::{FONT_SIZE, MENU_PADDING, SETTINGS_START_Y, SETTINGS_OPTION_HEIGHT};
use crate::SystemInfo;

// Import types/structs/constants that are still in main.rs
use crate::{
    AnimationState, AudioSink, BackgroundState, BatteryInfo, InputState, Screen, SoundEffects,
    render_background, render_ui_overlay, get_current_font, measure_text,
    text_with_config_color, find_sound_packs, play_new_bgm
};

// ABOUT
pub fn render_about_screen(
    system_info: &SystemInfo,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    scale_factor: f32,
) {
    let current_font = get_current_font(font_cache, config);
    // Create one smaller font size for all text on this screen
    let about_font_size = (FONT_SIZE as f32 * scale_factor * 0.8) as u16; // 85% of base size
    let line_height = 25.0 * scale_factor; // Adjusted line height for smaller text

    let start_x_labels = 50.0 * scale_factor;
    let start_x_values = 120.0 * scale_factor; // Nudged this over slightly for long CPU/GPU names
    let mut current_y = 100.0 * scale_factor;

    // --- Hardware Info ---
    let info = vec![
        ("OS:", &system_info.os_name),
        ("KERNEL:", &system_info.kernel),
        ("CPU:", &system_info.cpu),
        ("GPU:", &system_info.gpu),
        ("MEMORY:", &system_info.ram_total),
    ];

    for (label, value) in info {
        // Use the new smaller font size here
        text_with_config_color(font_cache, config, label, start_x_labels, current_y, about_font_size);
        text_with_config_color(font_cache, config, value, start_x_values, current_y, about_font_size);
        current_y += line_height;
    }

    // --- Credits ---
    current_y = screen_height() - (100.0 * scale_factor);

    let credits1 = "Original Kazeta concept by Alkazar.";
    let credits2 = "Kazeta+ forked and developed by Linux Gaming Central.";
    let credits3 = "Kazeta website: kazeta.org";
    let credits4 = "Linux Gaming Central website: linuxgamingcentral.org";

    let credit_lines = vec![credits1, credits2, credits3, credits4];

    for line in credit_lines {
        let dims = measure_text(line, Some(current_font), about_font_size, 1.0);
        // And use the new smaller font size here as well
        text_with_config_color(
            font_cache, config, line,
            screen_width() / 2.0 - dims.width / 2.0,
                               current_y,
                               about_font_size
        );
        current_y += line_height;
    }
}
