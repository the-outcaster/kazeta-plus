use macroquad::prelude::*;
use std::collections::HashMap;

use crate::{
    audio::SoundEffects,
    config::Config, FONT_SIZE, SystemInfo, Screen, BackgroundState, BatteryInfo, render_background, render_ui_overlay, get_current_font, measure_text, text_with_config_color, InputState,
};

pub fn update(
    input_state: &InputState,
    current_screen: &mut Screen,
    sound_effects: &SoundEffects,
    config: &Config,
) {
    if input_state.back {
        *current_screen = Screen::MainMenu;
        sound_effects.play_back(config);
    }
}

pub fn draw(
    system_info: &SystemInfo,
    logo_cache: &HashMap<String, Texture2D>,
    background_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    background_state: &mut BackgroundState,
    battery_info: &Option<BatteryInfo>,
    current_time_str: &str,
    scale_factor: f32,
) {
    render_background(&background_cache, &config, background_state);

    // --- NEW: Dim the background to improve text readability ---
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.5));
    // ---

    render_ui_overlay(&logo_cache, &font_cache, &config, &battery_info, &current_time_str, scale_factor);

    let current_font = get_current_font(font_cache, config);
    let about_font_size = (FONT_SIZE as f32 * scale_factor * 0.8) as u16;
    let line_height = 25.0 * scale_factor;

    let start_x_labels = 50.0 * scale_factor;
    let start_x_values = 120.0 * scale_factor;
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
        text_with_config_color(font_cache, config, label, start_x_labels, current_y, about_font_size);
        text_with_config_color(font_cache, config, value, start_x_values, current_y, about_font_size);
        current_y += line_height;
    }

    // --- Credits ---
    current_y = screen_height() - (130.0 * scale_factor);

    let credit_lines = vec![
        "Original Kazeta concept by Alkazar.",
        "Kazeta+ forked and developed by Linux Gaming Central.",
        "Kazeta website: kazeta.org",
        "Linux Gaming Central website: linuxgamingcentral.org",
    ];

    for line in credit_lines {
        let dims = measure_text(line, Some(current_font), about_font_size, 1.0);
        let x_pos = screen_width() / 2.0 - dims.width / 2.0;

        // --- MODIFIED: Use the new shadow text function here as well ---
        text_with_config_color(
            font_cache, config, line,
            x_pos, current_y,
            about_font_size
        );
        // ---
        current_y += line_height;
    }
}
