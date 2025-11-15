use macroquad::prelude::*;
use std::collections::HashMap;

use crate::{
    audio::SoundEffects,
    config::Config,
    types::{AnimationState, BackgroundState, BatteryInfo, Screen},
    render_background, render_ui_overlay, get_current_font, measure_text, text_with_config_color,
    FONT_SIZE, MENU_PADDING, MENU_OPTION_HEIGHT, InputState,
};

pub const EXTRAS_MENU_OPTIONS: &[&str] = &[
    "CONNECT TO WI-FI",
    "PAIR BLUETOOTH CONTROLLER",
    "GET NEW THEMES",
    "DOWNLOAD RUNTIMES",
    "CD PLAYER",
    "CHECK FOR UPDATES",
];

/// Handles input and state logic for the Extras menu.
pub fn update(
    current_screen: &mut Screen,
    extras_menu_selection: &mut usize,
    input_state: &InputState,
    animation_state: &mut AnimationState,
    sound_effects: &SoundEffects,
    config: &Config,
) {
    if input_state.up {
        *extras_menu_selection = if *extras_menu_selection == 0 { EXTRAS_MENU_OPTIONS.len() - 1 } else { *extras_menu_selection - 1 };
        animation_state.trigger_transition();
        sound_effects.play_cursor_move(config);
    }
    if input_state.down {
        *extras_menu_selection = (*extras_menu_selection + 1) % EXTRAS_MENU_OPTIONS.len();
        animation_state.trigger_transition();
        sound_effects.play_cursor_move(config);
    }
    if input_state.back {
        *current_screen = Screen::MainMenu;
        sound_effects.play_back(config);
    }
    if input_state.select {
        sound_effects.play_select(config);
        match *extras_menu_selection {
            0 => *current_screen = Screen::Wifi,
            1 => *current_screen = Screen::Bluetooth,
            2 => *current_screen = Screen::ThemeDownloader,
            3 => *current_screen = Screen::RuntimeDownloader,
            4 => *current_screen = Screen::CdPlayer,
            5 => *current_screen = Screen::UpdateChecker,
            _ => {}
        }
    }
}

/// Draws the Extras menu UI.
pub fn draw(
    selected_option: usize,
    animation_state: &AnimationState,
    logo_cache: &HashMap<String, Texture2D>,
    background_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    background_state: &mut BackgroundState,
    battery_info: &Option<BatteryInfo>,
    current_time_str: &str,
    gcc_adapter_poll_rate: &Option<u32>,
    scale_factor: f32,
) {
    render_background(background_cache, config, background_state);

    // dim the background for easier legibility
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.5));

    render_ui_overlay(logo_cache, font_cache, config, battery_info, current_time_str, gcc_adapter_poll_rate, scale_factor);

    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let menu_padding = MENU_PADDING * scale_factor;
    let menu_option_height = MENU_OPTION_HEIGHT * scale_factor;
    let current_font = get_current_font(font_cache, config);

    // Center the menu
    let start_x = screen_width() / 2.0;
    //let start_y = screen_height() * 0.35;
    let start_y = screen_height() * 0.25;

    // Draw menu options
    for (i, &option) in EXTRAS_MENU_OPTIONS.iter().enumerate() {
        let y_pos = start_y + (i as f32 * menu_option_height);
        let text_dims = measure_text(option, Some(current_font), font_size, 1.0);
        let x_pos = start_x - (text_dims.width / 2.0);

        // Draw selected option highlight
        if i == selected_option {
            let cursor_color = animation_state.get_cursor_color(config);
            let cursor_scale = animation_state.get_cursor_scale();
            let base_width = text_dims.width + (menu_padding * 2.0);
            let base_height = text_dims.height + (menu_padding * 2.0);
            let scaled_width = base_width * cursor_scale;
            let scaled_height = base_height * cursor_scale;
            let offset_x = (scaled_width - base_width) / 2.0;
            let offset_y = (scaled_height - base_height) / 2.0;
            let rect_x = x_pos - menu_padding;
            let slot_center_y = y_pos + (menu_option_height / 2.0);
            let rect_y = slot_center_y - (base_height / 2.0);

            draw_rectangle_lines(
                rect_x - offset_x,
                rect_y - offset_y,
                scaled_width,
                scaled_height,
                4.0 * scale_factor,
                cursor_color,
            );
        }

        // Draw text
        let slot_center_y = y_pos + (menu_option_height / 2.0);
        let y_pos_text = slot_center_y + (text_dims.offset_y / 2.0);
        text_with_config_color(font_cache, config, option, x_pos, y_pos_text, font_size);
    }
}
