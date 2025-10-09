use macroquad::prelude::*;
use std::collections::HashMap;
use std::process::Command;
use macroquad::audio::{Sound, set_sound_volume};

// Import things from our new modules
use crate::audio::{SoundEffects, play_new_bgm};
//use crate::config::{Config, save_config};
use crate::config::Config;
use crate::system::{adjust_system_volume, get_system_volume, set_brightness, get_current_brightness};
use crate::utils::{apply_resolution, trim_extension};
use crate::{FONT_SIZE, MENU_PADDING};
use crate::theme;

// Import types/structs/constants that are still in main.rs
use crate::{
    AnimationState, AudioSink, BackgroundState, BatteryInfo, InputState, Screen,
    render_background, render_ui_overlay, get_current_font, measure_text,
    text_with_config_color,
};

const SETTINGS_START_Y: f32 = 80.0;
const SETTINGS_OPTION_HEIGHT: f32 = 30.0;

pub const VIDEO_SETTINGS: &[&str] = &[
    "RESET SETTINGS",
    "RESOLUTION",
    "USE FULLSCREEN",
    "SHOW SPLASH SCREEN",
    "TIME ZONE",
    "BRIGHTNESS",
    "AUDIO SETTINGS",
];

pub const AUDIO_SETTINGS: &[&str] = &[
    "MASTER VOLUME",
    "BGM VOLUME",
    "SFX VOLUME",
    "AUDIO OUTPUT",
    "VIDEO SETTINGS",
    "GUI CUSTOMIZATION",
];

pub const GUI_CUSTOMIZATION_SETTINGS: &[&str] = &[
    "THEME",
    "MAIN MENU POSITION",
    "FONT COLOR",
    "CURSOR COLOR",
    "BACKGROUND SCROLLING",
    "COLOR GRADIENT SHIFTING",
    "AUDIO SETTINGS",
    "CUSTOM ASSETS SETTINGS",
];

pub const CUSTOM_ASSET_SETTINGS: &[&str] = &[
    "BACKGROUND MUSIC",
    "SOUND PACK",
    "LOGO",
    "BACKGROUND",
    "FONT TYPE",
    "GUI CUSTOMIZATION SETTINGS",
];

pub const FONT_COLORS: &[&str] = &[
    "WHITE",
    "PINK",
    "RED",
    "ORANGE",
    "YELLOW",
    "GREEN",
    "BLUE",
    "PURPLE",
];

pub const RESOLUTIONS: &[&str] = &[
    "640x360",
    "1280x720",
    "1280x800", // Steam Deck
    "1920x1080",
    "1920x1200", // DeckHD
    "2560x1440",
    "3840x2160",
];

pub const SCROLL_SPEEDS: &[&str] = &["OFF", "SLOW", "NORMAL", "FAST"];
pub const COLOR_SHIFT_SPEEDS: &[&str] = &["OFF", "SLOW", "NORMAL", "FAST"];

pub const TIMEZONES: [&str; 25] = [
    "UTC-12", "UTC-11", "UTC-10", "UTC-9", "UTC-8", "UTC-7", "UTC-6",
    "UTC-5", "UTC-4", "UTC-3", "UTC-2", "UTC-1", "UTC", "UTC+1",
    "UTC+2", "UTC+3", "UTC+4", "UTC+5", "UTC+6", "UTC+7", "UTC+8",
    "UTC+9", "UTC+10", "UTC+11", "UTC+12",
];

// SETTINGS
pub fn render_settings_page(
    page_number: usize,
    options: &[&str],
    logo_cache: &HashMap<String, Texture2D>,
    background_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &mut Config,
    selection: usize,
    animation_state: &AnimationState,
    background_state: &mut BackgroundState,
    battery_info: &Option<BatteryInfo>,
    current_time_str: &str,
    scale_factor: f32,
    display_settings_changed: bool,
    system_volume: f32,
    brightness: f32,
) {
    // --- Create scaled layout values ---
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let menu_padding = MENU_PADDING * scale_factor;
    let settings_start_y = SETTINGS_START_Y * scale_factor;
    let settings_option_height = SETTINGS_OPTION_HEIGHT * scale_factor;
    let right_margin = 50.0 * scale_factor;
    let left_margin = 50.0 * scale_factor;

    // get currently selected font at start
    let current_font = get_current_font(font_cache, config);

    render_background(background_cache, config, background_state);

    // dim the background for easier legibility
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.5));

    render_ui_overlay(logo_cache, font_cache, config, battery_info, current_time_str, scale_factor);

    //render_debug_info(config);

    // Loop through and draw all settings options
    for (i, label_text) in options.iter().enumerate() {
        let y_pos_base = settings_start_y + (i as f32 * settings_option_height);

        let value_text = get_settings_value(page_number, i, config, system_volume, brightness);

        // --- UPDATED: Consistent and Dynamic Layout Calculations ---
        let value_dims = measure_text(&value_text.to_uppercase(), Some(current_font), font_size, 1.0);

        // --- Draw the highlight rectangle if this item is selected ---
        if i == selection {
            let cursor_color = animation_state.get_cursor_color(config);
            let cursor_scale = animation_state.get_cursor_scale();
            //let value_dims = measure_text(&value_text.to_uppercase(), Some(current_font), FONT_SIZE as u16, 1.0);

            let base_width = value_dims.width + (menu_padding * 2.0);
            let base_height = value_dims.height + (menu_padding * 2.0);
            let scaled_width = base_width * cursor_scale;
            let scaled_height = base_height * cursor_scale;
            let offset_x = (scaled_width - base_width) / 2.0;
            let offset_y = (scaled_height - base_height) / 2.0;

            let value_x = screen_width() - value_dims.width - right_margin;
            //let rect_x = value_x - MENU_PADDING - offset_x;
            let rect_x = value_x - menu_padding;
            //let rect_y = y_pos_base - 7.0 - offset_y;
            let rect_y = y_pos_base + (settings_option_height / 2.0) - (base_height / 2.0);
            //draw_rectangle_lines(rect_x, rect_y, scaled_width, scaled_height / 1.5, 4.0, cursor_color);
            draw_rectangle_lines(rect_x - offset_x, rect_y - offset_y, scaled_width, scaled_height, 4.0 * scale_factor, cursor_color);
        }

        // --- Draw the text ---
        let value_x = screen_width() - value_dims.width - right_margin;
        let text_y = y_pos_base + (settings_option_height / 2.0) + (value_dims.offset_y * 0.5);

        text_with_config_color(font_cache, config, label_text, left_margin, text_y, font_size);
        text_with_config_color(font_cache, config, &value_text, value_x, text_y, font_size);

        if display_settings_changed {
            let message = "RESTART REQUIRED TO APPLY CHANGES";
            let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
            let current_font = get_current_font(font_cache, config);
            let dims = measure_text(message, Some(current_font), font_size, 1.0);

            let x = screen_width() / 2.0 - dims.width / 2.0;
            let y = screen_height() - (40.0 * scale_factor); // Position near the bottom

            // Draw a semi-transparent background for the message
            draw_rectangle(x - (5.0 * scale_factor), y - dims.height, dims.width + (10.0 * scale_factor), dims.height + (5.0 * scale_factor), Color::new(0.0, 0.0, 0.0, 0.7));

            text_with_config_color(font_cache, config, message, x, y, font_size);
        }
    }
}

// SETTINGS VALUE
// Text for the settings on the RIGHT side
pub fn get_settings_value(page: usize, index: usize, config: &Config, system_volume: f32, brightness: f32) -> String {
    match page {
        // VIDEO SETTINGS
        1 => match index {
            0 => "CONFIRM".to_string(), // RESET SETTINGS
            1 => config.resolution.clone(), // RESOLUTION
            2 => if config.fullscreen { "ON" } else { "OFF" }.to_string(), // FULLSCREEN TOGGLE
            3 => if config.show_splash_screen { "ON" } else { "OFF" }.to_string(), // SPLASH SCREEN TOGGLE
            4 => config.timezone.clone().to_uppercase(), // TIME ZONE
            5 => format!("{:.0}%", brightness * 100.0), // BRIGHTNESS
            6 => "->".to_string(),
            _ => "".to_string(),
        },
        // AUDIO SETTINGS
        2 => match index {
            0 => format!("{:.0}%", system_volume * 100.0), // MASTER VOLUME
            1 => format!("{:.0}%", config.bgm_volume * 100.0), // BGM VOLUME
            2 => format!("{:.0}%", config.sfx_volume * 100.0), // SFX VOLUME
            3 => config.audio_output.clone().to_uppercase(), // AUDIO OUTPUT
            4 => "<-".to_string(),
            5 => "->".to_string(),
            _ => "".to_string(),
        },
        // GUI CUSTOMIZATION
        3 => match index {
            0 => config.theme.clone().replace('_', " ").to_uppercase(), // THEME SELECTION
            1 => format!("{:?}", config.menu_position).to_uppercase(), // MENU POSITION
            2 => config.font_color.clone(), // FONT COLOR
            3 => config.cursor_color.clone(), // CURSOR COLOR
            4 => config.background_scroll_speed.clone(), // BACKGROUND SCROLL SPEED
            5 => config.color_shift_speed.clone(), // COLOR SHIFTING GRADIENT SPEED
            6 => "<-".to_string(),
            7 => "->".to_string(),
            _ => "".to_string(),
        },
        // CUSTOM ASSETS
        4 => match index {
            0 => { // BGM SELECTION
                // Always show the current track or "OFF"
                let track = config.bgm_track.clone().unwrap_or("OFF".to_string());
                trim_extension(&track).replace('_', " ").to_uppercase()
            },
            1 => { // SOUND PACK
                // Always show the currently selected sound pack
                config.sfx_pack.clone().replace('_', " ").to_uppercase()
            },
            2 => { // LOGO
                // Always show the currently selected logo
                trim_extension(&config.logo_selection).replace('_', " ").to_uppercase()
            },
            3 => { // BACKGROUND
                // Always show the currently selected background
                trim_extension(&config.background_selection).replace('_', " ").to_uppercase()
            },
            4 => { // FONT TYPE
                // Always show the currently selected font
                trim_extension(&config.font_selection).replace('_', " ").to_uppercase()
            },
            5 => "<-".to_string(),
            _ => "".to_string(),
        },
        _ => "".to_string(), // Default case for unknown pages
    }
}

// --- UPDATE FUNCTION ---
pub fn update(
    current_screen: &mut Screen,
    input_state: &InputState,
    config: &mut Config,
    themes: &Vec<String>,
    sound_pack_choices: &Vec<String>,
    loaded_themes: &HashMap<String, theme::Theme>,
    settings_menu_selection: &mut usize,
    sound_effects: &mut SoundEffects,
    confirm_selection: &mut usize,
    display_settings_changed: &mut bool,
    brightness: &mut f32,
    system_volume: &mut f32,
    available_sinks: &Vec<AudioSink>,
    current_bgm: &mut Option<Sound>,
    bgm_choices: &Vec<String>,
    music_cache: &HashMap<String, Sound>,
    sfx_pack_to_reload: &mut Option<String>,
    logo_choices: &Vec<String>,
    background_choices: &Vec<String>,
    font_choices: &Vec<String>,
) {
    // --- Determine current page info ---
    let (page_number, options): (usize, &[&str]) = match *current_screen {
        Screen::VideoSettings => (1, &VIDEO_SETTINGS),
        Screen::AudioSettings => (2, &AUDIO_SETTINGS),
        Screen::GuiSettings => (3, &GUI_CUSTOMIZATION_SETTINGS),
        Screen::AssetSettings => (4, &CUSTOM_ASSET_SETTINGS),
        _ => unreachable!(),
    };

    // INPUT HANDLING
    if input_state.up {
        *settings_menu_selection = if *settings_menu_selection == 0 { options.len() - 1 } else { *settings_menu_selection - 1 };
        sound_effects.play_cursor_move(&config);
    }
    if input_state.down {
        *settings_menu_selection = (*settings_menu_selection + 1) % options.len();
        sound_effects.play_cursor_move(&config);
    }
    if input_state.back {
        *current_screen = Screen::MainMenu;
        sound_effects.play_back(&config);
    }
    if input_state.next {
        sound_effects.play_select(&config);
        *settings_menu_selection = 0; // Reset selection for the new page
        match current_screen {
            Screen::VideoSettings => *current_screen = Screen::AudioSettings,
            Screen::AudioSettings => *current_screen = Screen::GuiSettings,
            Screen::GuiSettings => *current_screen = Screen::AssetSettings,
            Screen::AssetSettings => *current_screen = Screen::VideoSettings,
            _ => {} // This case won't be reached
        }
    }
    if input_state.prev {
        sound_effects.play_select(&config);
        *settings_menu_selection = 0; // Reset selection for the new page
        match current_screen {
            Screen::VideoSettings => *current_screen = Screen::AssetSettings,
            Screen::AudioSettings => *current_screen = Screen::VideoSettings,
            Screen::GuiSettings => *current_screen = Screen::AudioSettings,
            Screen::AssetSettings => *current_screen = Screen::GuiSettings,
            _ => {} // This case won't be reached
        }
    }

    match page_number {
        // VIDEO OPTIONS
        1 => match settings_menu_selection {
            0 => { // RESET SETTINGS
                if input_state.select {
                    sound_effects.play_select(&config);
                    *confirm_selection = 1; // Default to "NO"
                    *current_screen = Screen::ConfirmReset;
                }
            },
            1 => { // RESOLUTION
                if input_state.left || input_state.right {
                    let current_index = RESOLUTIONS.iter().position(|&r| r == config.resolution).unwrap_or(0);
                    let new_index = if input_state.right {
                        (current_index + 1) % RESOLUTIONS.len()
                    } else {
                        (current_index + RESOLUTIONS.len() - 1) % RESOLUTIONS.len()
                    };

                    config.resolution = RESOLUTIONS[new_index].to_string();
                    config.save();
                    apply_resolution(&config.resolution); // Apply the change immediately
                    sound_effects.play_cursor_move(&config);
                }
            },
            2 => { // FULLSCREEN
                if input_state.left || input_state.right {
                    config.fullscreen = !config.fullscreen;
                    //set_fullscreen(config.fullscreen); // Apply the change immediately
                    config.save();
                    sound_effects.play_cursor_move(&config);
                    *display_settings_changed = true;
                }
            },
            3 => { // SPLASH SCREEN
                if input_state.left || input_state.right {
                    config.show_splash_screen = !config.show_splash_screen;
                    config.save();
                    sound_effects.play_cursor_move(&config);
                }
            },
            4 => { // TIME ZONE
                let mut change_occurred = false;

                // Find the current index of the timezone in our array
                if let Some(current_index) = TIMEZONES.iter().position(|&tz| tz == config.timezone) {
                    if input_state.left {
                        // Decrement and wrap around if we go below zero
                        let new_index = (current_index + TIMEZONES.len() - 1) % TIMEZONES.len();
                        config.timezone = TIMEZONES[new_index].to_string();
                        change_occurred = true;
                    }
                    if input_state.right {
                        // Increment and wrap around using the modulo operator
                        let new_index = (current_index + 1) % TIMEZONES.len();
                        config.timezone = TIMEZONES[new_index].to_string();
                        change_occurred = true;
                    }
                }

                if change_occurred {
                    sound_effects.play_cursor_move(&config);
                    config.save();
                }
            },
            5 => { // MASTER VOLUME
                if input_state.left {
                    set_brightness(*brightness - 0.1); // Decrease by 10%
                    *brightness = get_current_brightness().unwrap_or(*brightness); // Refresh the value
                    sound_effects.play_cursor_move(&config);
                }
                if input_state.right {
                    set_brightness(*brightness + 0.1); // Increase by 10%
                    *brightness = get_current_brightness().unwrap_or(*brightness); // Refresh the value
                    sound_effects.play_cursor_move(&config);
                }
            },
            6 => { // GO TO AUDIO SETTINGS
                if input_state.select {
                    *current_screen = Screen::AudioSettings;
                    *settings_menu_selection = 0;
                    sound_effects.play_select(&config);
                }
            },
            _ => {}
        },

        // AUDIO SETTINGS
        2 => match settings_menu_selection {
            0 => { // MASTER VOLUME
                if input_state.left {
                    adjust_system_volume("10%-"); // Decrease by 10%
                    *system_volume = get_system_volume().unwrap_or(*system_volume); // Refresh the value
                    sound_effects.play_cursor_move(&config);
                }
                if input_state.right {
                    adjust_system_volume("10%+"); // Increase by 10%
                    *system_volume = get_system_volume().unwrap_or(*system_volume); // Refresh the value
                    sound_effects.play_cursor_move(&config);
                }
            },
            1 => { // BGM VOLUME
                if input_state.left || input_state.right {
                    if input_state.left {
                        config.bgm_volume = (config.bgm_volume - 0.1).max(0.0);
                    }
                    if input_state.right {
                        config.bgm_volume = (config.bgm_volume + 0.1).min(1.0);
                    }

                    // Change the volume of the currently playing sound
                    if let Some(sound) = &current_bgm {
                        set_sound_volume(sound, config.bgm_volume);
                    }

                    config.save();
                    sound_effects.play_cursor_move(&config);
                }
            },
            2 => { // SFX Volume
                if input_state.left || input_state.right {
                    if input_state.left {
                        config.sfx_volume = (config.sfx_volume - 0.1).max(0.0);
                    }
                    if input_state.right {
                        config.sfx_volume = (config.sfx_volume + 0.1).min(1.0);
                    }
                    config.save();
                    sound_effects.play_cursor_move(&config); // Test the new volume
                }
            },
            3 => { // AUDIO OUTPUT
                // Only run this logic if we actually found sinks
                if !available_sinks.is_empty() {
                    // Find the index of the current sink in our discovered list
                    let current_index = available_sinks.iter().position(|s| s.name == config.audio_output).unwrap_or(0);

                    let mut new_index = current_index;
                    if input_state.left {
                        new_index = (current_index + available_sinks.len() - 1) % available_sinks.len();
                    }
                    if input_state.right {
                        new_index = (current_index + 1) % available_sinks.len();
                    }

                    if new_index != current_index {
                        let new_sink = &available_sinks[new_index];
                        config.audio_output = new_sink.name.clone();

                        // Apply the change immediately
                        let _ = Command::new("wpctl").arg("set-default").arg(new_sink.id.to_string()).status();

                        // Create a sentinel file to make the choice persistent
                        let state_dir = std::path::Path::new("/var/kazeta/state");
                        if std::fs::create_dir_all(state_dir).is_ok() {
                            let _ = std::fs::File::create(state_dir.join(".AUDIO_PREFERENCE_SET"));
                        }

                        sound_effects.play_cursor_move(&config);
                    }
                }
            },
            4 => { // GO TO VIDEO SETTINGS
                if input_state.select {
                    *current_screen = Screen::VideoSettings;
                    *settings_menu_selection = 0;
                    sound_effects.play_select(&config);
                }
            },
            5 => { // GO TO GUI CUSTOMIZATION
                if input_state.select {
                    *current_screen = Screen::GuiSettings;
                    *settings_menu_selection = 0;
                    sound_effects.play_select(&config);
                }
            },
            _ => {}
        },

        // GUI CUSTOMIZATION OPTIONS
        3 => match settings_menu_selection {
            0 => { // THEME SELECTION
                if input_state.left || input_state.right {
                    if themes.is_empty() { return; } // Prevent panic if no themes are loaded

                    let current_index = themes.iter().position(|t| *t == config.theme).unwrap_or(0);
                    let new_index = if input_state.right {
                        (current_index + 1) % themes.len()
                    } else {
                        (current_index + themes.len() - 1) % themes.len()
                    };

                    // Clone the name here to work around borrowing rules
                    let new_theme_name = themes[new_index].clone();

                    // --- REVISED THEME SWITCHING LOGIC ---

                    // Only run this logic if the theme has actually changed
                    if config.theme != new_theme_name {
                        config.theme = new_theme_name.clone();

                        // Special case for the "Default" theme
                        if new_theme_name == "Default" {
                            println!("[INFO] Switched to Default theme.");
                            let defaults = Config::default(); // Get a fresh set of default values

                            // Apply the default settings to the live config
                            config.sfx_pack = defaults.sfx_pack;
                            config.bgm_track = defaults.bgm_track;
                            config.logo_selection = defaults.logo_selection;
                            config.background_selection = defaults.background_selection;
                            config.font_selection = defaults.font_selection;
                            config.menu_position = defaults.menu_position;
                            config.font_color = defaults.font_color;
                            config.cursor_color = defaults.cursor_color;
                            config.background_scroll_speed = defaults.background_scroll_speed;
                            config.color_shift_speed = defaults.color_shift_speed;

                            // Apply the default sound effects from the pre-loaded "Default" theme
                            if let Some(default_theme) = loaded_themes.get("Default") {
                                *sound_effects = default_theme.sounds.clone();
                            }

                        } else {
                            // For any other theme, apply its settings from the .toml file
                            if let Some(theme) = loaded_themes.get(&new_theme_name) {
                                println!("[INFO] Switched to '{}' theme.", new_theme_name);

                                // This is the most efficient way to apply the new SFX
                                *sound_effects = theme.sounds.clone();

                                // Apply settings from the theme's config, falling back to defaults if a key is missing
                                config.sfx_pack = theme.config.sfx_pack.clone().unwrap_or_else(|| "Default".to_string());
                                config.bgm_track = theme.config.bgm_track.clone();
                                config.logo_selection = theme.config.logo_selection.clone().unwrap_or_else(|| "Kazeta+ (Default)".to_string());
                                config.background_selection = theme.config.background_selection.clone().unwrap_or_else(|| "Default".to_string());
                                config.font_selection = theme.config.font_selection.clone().unwrap_or_else(|| "Default".to_string());

                                // For these, we only update the config if the value exists in the theme.toml
                                if let Some(val) = &theme.config.menu_position {
                                    // .parse() is needed to convert the String "BottomLeft" into the MenuPosition::BottomLeft enum
                                    config.menu_position = val.parse().unwrap_or_default();
                                }
                                if let Some(val) = &theme.config.font_color {
                                    config.font_color = val.clone();
                                }
                                if let Some(val) = &theme.config.cursor_color {
                                    config.cursor_color = val.clone();
                                }
                                if let Some(val) = &theme.config.background_scroll_speed {
                                    config.background_scroll_speed = val.clone();
                                }
                                if let Some(val) = &theme.config.color_shift_speed {
                                    config.color_shift_speed = val.clone();
                                }
                            }
                        }

                        // --- APPLY BGM CHANGE IMMEDIATELY ---
                        // This runs after the config has been updated by the logic above.
                        play_new_bgm(
                            &config.bgm_track.clone().unwrap_or_else(|| "OFF".to_string()),
                            config.bgm_volume,
                            music_cache,
                            current_bgm,
                        );

                        // Play a confirmation sound with the (potentially new) SFX pack
                        sound_effects.play_cursor_move(config);
                        config.save();
                    }
                }
            },
            1 => { // MENU POSITION
                if input_state.left {
                    config.menu_position = config.menu_position.prev();
                    config.save();
                    sound_effects.play_cursor_move(config);
                }
                if input_state.right {
                    config.menu_position = config.menu_position.next();
                    config.save();
                    sound_effects.play_cursor_move(config);
                }
            },
            2 => { // FONT COLOR
                if input_state.left || input_state.right {
                    // Find current color's index in our list
                    let current_index = FONT_COLORS.iter().position(|&c| c == config.font_color).unwrap_or(0);
                    let new_index = if input_state.right {
                        (current_index + 1) % FONT_COLORS.len()
                    } else {
                        (current_index + FONT_COLORS.len() - 1) % FONT_COLORS.len()
                    };
                    config.font_color = FONT_COLORS[new_index].to_string();
                    config.save();
                    sound_effects.play_cursor_move(&config);
                }
            }
            3 => { // CURSOR COLOR
                if input_state.left || input_state.right {
                    // We can reuse the FONT_COLORS constant for this
                    let current_index = FONT_COLORS.iter().position(|&c| c == config.cursor_color).unwrap_or(0);
                    let new_index = if input_state.right {
                        (current_index + 1) % FONT_COLORS.len()
                    } else {
                        (current_index + FONT_COLORS.len() - 1) % FONT_COLORS.len()
                    };

                    config.cursor_color = FONT_COLORS[new_index].to_string();
                    config.save();
                    sound_effects.play_cursor_move(&config);
                }
            },
            4 => { // BACKGROUND SCROLLING
                if input_state.left || input_state.right {
                    let current_index = SCROLL_SPEEDS.iter().position(|&s| s == config.background_scroll_speed).unwrap_or(0);
                    let new_index = if input_state.right {
                        (current_index + 1) % SCROLL_SPEEDS.len()
                    } else {
                        (current_index + SCROLL_SPEEDS.len() - 1) % SCROLL_SPEEDS.len()
                    };

                    config.background_scroll_speed = SCROLL_SPEEDS[new_index].to_string();
                    config.save();
                    sound_effects.play_cursor_move(&config);
                }
            },
            5 => { // COLOR GRADIENT SHIFTING
                if input_state.left || input_state.right {
                    let current_index = COLOR_SHIFT_SPEEDS.iter().position(|&s| s == config.color_shift_speed).unwrap_or(0);
                    let new_index = if input_state.right {
                        (current_index + 1) % COLOR_SHIFT_SPEEDS.len()
                    } else {
                        (current_index + COLOR_SHIFT_SPEEDS.len() - 1) % COLOR_SHIFT_SPEEDS.len()
                    };

                    config.color_shift_speed = COLOR_SHIFT_SPEEDS[new_index].to_string();
                    config.save();
                    sound_effects.play_cursor_move(&config);
                }
            },
            6 => { // GO TO AUDIO SETTINGS
                if input_state.select {
                    *current_screen = Screen::AudioSettings;
                    *settings_menu_selection = 0;
                    sound_effects.play_select(&config);
                }
            },
            7 => { // GO TO CUSTOM ASSETS
                if input_state.select {
                    *current_screen = Screen::AssetSettings;
                    *settings_menu_selection = 0;
                    sound_effects.play_select(&config);
                }
            },
            _ => {}
        },
        // CUSTOM ASSETS
        4 => match settings_menu_selection {
            0 => { // BGM SELECTION
                if input_state.left || input_state.right {
                    // Find the current track's position in our list of choices
                    let current_index = bgm_choices.iter().position(|t| *t == config.bgm_track.clone().unwrap_or("OFF".to_string())).unwrap_or(0);
                    let mut new_index = current_index;

                    if input_state.left {
                        new_index = if current_index == 0 { bgm_choices.len() - 1 } else { current_index - 1 };
                    }
                    if input_state.right {
                        new_index = (current_index + 1) % bgm_choices.len();
                    }

                    let new_track = &bgm_choices[new_index];
                    play_new_bgm(new_track, config.bgm_volume, &music_cache, current_bgm);

                    // Update the config with the new choice
                    if new_track == "OFF" {
                        config.bgm_track = None;
                    } else {
                        config.bgm_track = Some(new_track.clone());
                    }

                    config.save();
                    sound_effects.play_cursor_move(&config);
                }
            },
            1 => { // SOUND PACK
                if input_state.left || input_state.right {
                    // `sound_pack_choices` is the Vec<String> of available packs
                    let current_index = sound_pack_choices.iter().position(|p| *p == config.sfx_pack).unwrap_or(0);

                    let new_index = if input_state.right {
                        (current_index + 1) % sound_pack_choices.len()
                    } else {
                        (current_index + sound_pack_choices.len() - 1) % sound_pack_choices.len()
                    };

                    let new_pack_name = &sound_pack_choices[new_index];

                    if &config.sfx_pack != new_pack_name {
                        // 1. Update the config value
                        config.sfx_pack = new_pack_name.clone();

                        // 2. Set the request for the main loop to handle
                        *sfx_pack_to_reload = Some(new_pack_name.clone());

                        // 3. Save the config
                        config.save();
                    }
                }
            },
            2 => { // LOGO selection
                if input_state.left || input_state.right {
                    // Find the current logo's position in our list of choices
                    let current_index = logo_choices.iter().position(|l| *l == config.logo_selection).unwrap_or(0);
                    let mut new_index = current_index;

                    if input_state.left {
                        new_index = if current_index == 0 { logo_choices.len() - 1 } else { current_index - 1 };
                    }
                    if input_state.right {
                        new_index = (current_index + 1) % logo_choices.len();
                    }

                    // Update the config with the new choice
                    config.logo_selection = logo_choices[new_index].clone();

                    config.save();
                    sound_effects.play_cursor_move(&config);
                }
            },
            3 => { // BACKGROUND SELECTION
                if input_state.left || input_state.right {
                    // Find the current background's position in our list of choices
                    let current_index = background_choices.iter().position(|b| *b == config.background_selection).unwrap_or(0);
                    let mut new_index = current_index;

                    if input_state.left {
                        new_index = if current_index == 0 { background_choices.len() - 1 } else { current_index - 1 };
                    }
                    if input_state.right {
                        new_index = (current_index + 1) % background_choices.len();
                    }

                    // Update the config with the new choice
                    config.background_selection = background_choices[new_index].clone();

                    config.save();
                    sound_effects.play_cursor_move(&config);
                }
            },
            4 => { // FONT TYPE
                if input_state.left || input_state.right {
                    let current_index = font_choices.iter().position(|name| name == &config.font_selection).unwrap_or(0);
                    let new_index = if input_state.right {
                        (current_index + 1) % font_choices.len()
                    } else {
                        (current_index + font_choices.len() - 1) % font_choices.len()
                    };

                    config.font_selection = font_choices[new_index].clone();
                    config.save();
                    sound_effects.play_cursor_move(&config);
                }
            },
            5 => { // GO TO GUI CUSTOMIZATION SETTINGS
                if input_state.select {
                    *current_screen = Screen::GuiSettings;
                    *settings_menu_selection = 0;
                    sound_effects.play_select(&config);
                }
            },
            _ => {}
        },
        _ => {}
    }
}
