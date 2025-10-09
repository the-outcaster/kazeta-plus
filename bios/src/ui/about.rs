use macroquad::prelude::*;
use std::collections::HashMap;
use crate::audio::SoundEffects;

use crate::{
    config::Config, FONT_SIZE, SystemInfo, Screen, BackgroundState, BatteryInfo, render_background, render_ui_overlay, get_current_font, measure_text, text_with_config_color, InputState,
};

// New helper function to draw text with a drop shadow for readability
fn draw_text_with_shadow(
    font_cache: &HashMap<String, Font>,
    config: &Config,
    text: &str,
    x: f32,
    y: f32,
    font_size: u16,
    shadow_color: Color,
    offset: f32,
) {
    // Draw the shadow text first
    let shadow_params = TextParams {
        font: Some(get_current_font(font_cache, config)),
        font_size,
        color: shadow_color,
        ..Default::default()
    };
    draw_text_ex(text, x + offset, y + offset, shadow_params);

    // Draw the main text on top
    text_with_config_color(font_cache, config, text, x, y, font_size);
}

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

    // Define shadow properties for our new text function
    let shadow_offset = 1.0 * scale_factor;
    let shadow_color = BLACK;

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
        // --- MODIFIED: Use the new shadow text function ---
        draw_text_with_shadow(font_cache, config, label, start_x_labels, current_y, about_font_size, shadow_color, shadow_offset);
        draw_text_with_shadow(font_cache, config, value, start_x_values, current_y, about_font_size, shadow_color, shadow_offset);
        // ---
        current_y += line_height;
    }

    // --- Credits ---
    // --- MODIFIED: Increased the offset to fix clipping ---
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
        draw_text_with_shadow(
            font_cache, config, line,
            x_pos, current_y,
            about_font_size,
            shadow_color,
            shadow_offset
        );
        // ---
        current_y += line_height;
    }
}
