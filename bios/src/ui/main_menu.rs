use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

// --- Corrected Imports ---

// Items from your new modules
use crate::audio::{SoundEffects, play_new_bgm};
use crate::config::Config;
use crate::save;
use crate::types::{AnimationState, BackgroundState, BatteryInfo, ShakeTarget};

// Items that are still in `main.rs` (the crate root)
use crate::{
    Screen, UIFocus, InputState,
    copy_session_logs_to_sd,
    trigger_session_restart,
    start_log_reader,
    render_background,
    render_ui_overlay,
    get_current_font,
    measure_text,
    text_with_config_color,
    text_disabled,
    DEBUG_GAME_LAUNCH,
    FLASH_MESSAGE_DURATION,
    FONT_SIZE,
    MENU_PADDING,
    MENU_START_Y,
    MENU_OPTION_HEIGHT
};

pub const MAIN_MENU_OPTIONS: [&str; 5] = ["DATA", "PLAY", "COPY SESSION LOGS", "SETTINGS", "ABOUT"];

pub fn update(
    current_screen: &mut Screen,
    main_menu_selection: &mut usize,
    play_option_enabled: &mut bool,
    copy_logs_option_enabled: &mut bool,
    cart_connected: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    input_state: &InputState,
    animation_state: &mut AnimationState,
    sound_effects: &SoundEffects,
    config: &Config,
    log_messages: &std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    fade_start_time: &mut Option<f64>,
    current_bgm: &mut Option<macroquad::audio::Sound>,
    music_cache: &HashMap<String, macroquad::audio::Sound>,
    game_icon_queue: &mut Vec<(String, PathBuf)>,
    available_games: &mut Vec<(save::CartInfo, PathBuf)>,
    game_selection: &mut usize,
    flash_message: &mut Option<(String, f32)>,
    game_process: &mut Option<std::process::Child>,
) {
    // Update play option enabled status based on cart connection
    *play_option_enabled = cart_connected.load(Ordering::Relaxed);

    // Update copy logs option enabled status based on cart connection
    *copy_logs_option_enabled = cart_connected.load(Ordering::Relaxed);

    // Handle main menu navigation
    if input_state.up {
        if *main_menu_selection == 0 {
            *main_menu_selection = MAIN_MENU_OPTIONS.len() - 1;
        } else {
            *main_menu_selection = (*main_menu_selection - 1) % MAIN_MENU_OPTIONS.len();
        }
        animation_state.trigger_transition();
        sound_effects.play_cursor_move(&config);
    }
    if input_state.down {
        *main_menu_selection = (*main_menu_selection + 1) % MAIN_MENU_OPTIONS.len();
        animation_state.trigger_transition();
        sound_effects.play_cursor_move(&config);
    }
    if input_state.select {
        match *main_menu_selection {
            0 => { // SAVE DATA
                *current_screen = Screen::SaveData;
                input_state.ui_focus = UIFocus::Grid;
                sound_effects.play_select(&config);
            },
            1 => { // PLAY option
                if *play_option_enabled {
                    sound_effects.play_select(&config);
                    log_messages.lock().unwrap().clear();

                    match save::find_all_kzi_files() {
                        Ok((kzi_paths, mut debug_log)) => {
                            log_messages.lock().unwrap().append(&mut debug_log);

                            let mut games: Vec<(save::CartInfo, PathBuf)> = Vec::new();
                            let parse_errors: Vec<String> = Vec::new();

                            for path in &kzi_paths {
                                if let Ok(info) = save::parse_kzi_file(path) {
                                    games.push((info, path.clone()));
                                }
                            }

                            match games.len() {
                                0 => { // Case: Found files, but none were valid
                                    let mut logs = log_messages.lock().unwrap();
                                    logs.push(format!("[Info] Found {} potential game file(s), but none could be parsed.", kzi_paths.len()));
                                    logs.push("--- ERRORS ---".to_string());
                                    logs.extend(parse_errors);
                                    *current_screen = Screen::Debug;
                                },
                                1 => {
                                    // Case: Exactly one game found, go to Debug screen and launch
                                    let (cart_info, kzi_path) = games.remove(0);
                                    sound_effects.play_select(&config);

                                    if DEBUG_GAME_LAUNCH {
                                        { // Scoped lock to add messages
                                            let mut logs = log_messages.lock().unwrap();
                                            logs.push("--- CARTRIDGE FOUND ---".to_string());
                                            logs.push(format!("Name: {}", cart_info.name.as_deref().unwrap_or("N/A")));
                                            logs.push(format!("ID: {}", cart_info.id));
                                            logs.push(format!("Exec: {}", cart_info.exec));
                                            logs.push(format!("Runtime: {}", cart_info.runtime.as_deref().unwrap_or("None")));
                                            logs.push(format!("KZI Path: {}", kzi_path.display()));
                                        }
                                        println!("[Debug] Single Cartridge Found! Preparing to launch...");
                                        println!("[Debug]   Name: {}", cart_info.name.as_deref().unwrap_or("N/A"));
                                        println!("[Debug]   ID: {}", cart_info.id);
                                        println!("[Debug]   Exec: {}", cart_info.exec);
                                        println!("[Debug]   Runtime: {}", cart_info.runtime.as_deref().unwrap_or("None"));
                                        println!("[Debug]   KZI Path: {}", kzi_path.display());

                                        match save::launch_game(&cart_info, &kzi_path) {
                                            Ok(mut child) => {
                                                log_messages.lock().unwrap().push("\n--- LAUNCHING GAME ---".to_string());
                                                start_log_reader(&mut child, log_messages.clone());
                                                *game_process = Some(child);
                                            }
                                            Err(e) => {
                                                log_messages.lock().unwrap().push(format!("\n--- LAUNCH FAILED ---\nError: {}", e));
                                            }
                                        }
                                        *current_screen = Screen::Debug;
                                    } else {
                                        // --- PRODUCTION MODE: Fade out and launch ---
                                        (*current_screen, *fade_start_time) = trigger_session_restart(&mut current_bgm, &music_cache);
                                    }
                                },
                                _ => { // multiple games found
                                    println!("[Debug] Found {} games. Switching to selection screen.", games.len());
                                    for (index, (_, path)) in games.iter().enumerate() {
                                        println!("[Debug]   Game {}: {}", index + 1, path.display());
                                    }

                                    // --- FIX 2: Use the cart_info.icon field ---
                                    // Queue up the icons for loading.
                                    game_icon_queue.clear();
                                    for (cart_info, kzi_path) in &games {
                                        let icon_path = kzi_path.parent().unwrap().join(&cart_info.icon);
                                        game_icon_queue.push((cart_info.id.clone(), icon_path));
                                    }

                                    *available_games = games;
                                    *game_selection = 0;
                                    *current_screen = Screen::GameSelection;
                                }
                            }
                        },
                        Err(e) => { // Handle the error case
                            let error_msg = format!("[Error] Error scanning for cartridges: {}", e);
                            println!("[Error] {}", &error_msg);
                            log_messages.lock().unwrap().push(error_msg);
                            *current_screen = Screen::Debug;
                        }
                    }
                } else {
                    sound_effects.play_reject(&config);
                    animation_state.trigger_play_option_shake();
                }
            },
            2 => { // SESSION LOG COPY
                if *copy_logs_option_enabled {
                    sound_effects.play_select(&config);

                    // Call our new function and handle the result
                    match copy_session_logs_to_sd() {
                        Ok(path) => {
                            *flash_message = Some((
                                format!("LOGS COPIED TO {}", path),
                                FLASH_MESSAGE_DURATION
                            ));
                        }
                        Err(e) => {
                            *flash_message = Some((
                                format!("ERROR: {}", e),
                                FLASH_MESSAGE_DURATION
                            ));
                        }
                    }
                } else {
                    sound_effects.play_reject(&config);
                    animation_state.trigger_copy_log_option_shake();
                }
            },
            3 => { // SETTINGS
                *current_screen = Screen::VideoSettings;
                sound_effects.play_select(&config);
            },
            4 => { // ABOUT
                *current_screen = Screen::About;
                sound_effects.play_select(&config);
            },
            _ => {}
        }
    }
}

pub fn draw(
    menu_options: &[&str],
    selected_option: usize,
    play_option_enabled: bool,
    copy_logs_option_enabled: bool,
    animation_state: &AnimationState,
    logo_cache: &HashMap<String, Texture2D>,
    background_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    background_state: &mut BackgroundState,
    battery_info: &Option<BatteryInfo>,
    current_time_str: &str,
    scale_factor: f32,
    flash_message: Option<&str>,
) {
    render_background(background_cache, config, background_state);
    render_ui_overlay(logo_cache, font_cache, config, battery_info, current_time_str, scale_factor);

    let current_font = get_current_font(font_cache, config);
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let menu_padding = MENU_PADDING * scale_factor;

    // --- Draw menu options (centered) ---
    for (i, option) in menu_options.iter().enumerate() {
        let text_dims = measure_text(&option.to_uppercase(), Some(current_font), font_size, 1.0);
        let y_pos = MENU_START_Y + (i as f32 * MENU_OPTION_HEIGHT); // Use the imported constants
        let mut x_pos = (screen_width() - text_dims.width) / 2.0;

        if i == 1 && !play_option_enabled && i == selected_option {
            x_pos += animation_state.calculate_shake_offset(ShakeTarget::PlayOption) * scale_factor;
        }
        if i == 2 && !copy_logs_option_enabled && i == selected_option {
            x_pos += animation_state.calculate_shake_offset(ShakeTarget::CopyLogOption) * scale_factor;
        }

        if i == selected_option {
            let cursor_color = animation_state.get_cursor_color(config);
            let cursor_scale = animation_state.get_cursor_scale();
            let highlight_padding = MENU_PADDING * 1.5 * scale_factor;
            let base_width = text_dims.width + (highlight_padding * 2.0);
            let base_height = text_dims.height + (highlight_padding * 2.0);
            let scaled_width = base_width * cursor_scale;
            let scaled_height = base_height * cursor_scale;
            let offset_x = (scaled_width - base_width) / 2.0;
            let offset_y = (scaled_height - base_height) / 2.0;
            let rect_x = (screen_width() - base_width) / 2.0;
            let rect_y = y_pos - (base_height / 2.0) + (text_dims.height / 2.0) - (menu_padding / 2.0);
            draw_rectangle_lines(rect_x - offset_x, rect_y - offset_y, scaled_width, scaled_height, 4.0 * scale_factor, cursor_color);
        }

        let text_y_pos = y_pos + menu_padding;

        if (i == 1 && !play_option_enabled) || (i == 2 && !copy_logs_option_enabled) {
            text_disabled(font_cache, config, option, x_pos, text_y_pos, font_size);
        } else {
            text_with_config_color(font_cache, config, option, x_pos, text_y_pos, font_size);
        }
    }
    // --- Draw the flash message if it exists ---
    if let Some(message) = flash_message {
        let font_size = (14.0 * scale_factor) as u16;
        let dims = measure_text(message, Some(current_font), font_size, 1.0);

        let x = screen_width() / 2.0 - dims.width / 2.0; // Center horizontally
        let y = screen_height() - (20.0 * scale_factor); // Position near the bottom

        // Draw a semi-transparent background for better readability
        draw_rectangle(
            x - (5.0 * scale_factor),
                       y - dims.height,
                       dims.width + (10.0 * scale_factor),
                       dims.height + (5.0 * scale_factor),
                       Color::new(0.0, 0.0, 0.0, 0.7)
        );

        // Draw the message text
        text_with_config_color(font_cache, config, message, x, y, font_size);
    }
}
