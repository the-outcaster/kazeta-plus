// Add necessary imports for the shared functions
use crate::config::Config;
use crate::memory::{get_game_playtime, get_game_size};
use crate::{string_to_color, FONT_SIZE, BatteryInfo, MenuPosition, VERSION_NUMBER, BackgroundState, COLOR_TARGETS, UI_BG_COLOR,
    save, PathBuf, AnimationState, RECT_COLOR, Memory, Arc, Mutex, PlaytimeCache, SizeCache, TILE_SIZE,
    PADDING, GRID_OFFSET, GRID_WIDTH, ShakeTarget, Dialog, CopyOperationState, UI_BG_COLOR_DIALOG,
};
use macroquad::prelude::*;
use std::collections::HashMap;

pub mod about;
pub mod bluetooth;
pub mod cd_player;
pub mod data;
pub mod dialog;
pub mod extras_menu;
pub mod main_menu;
pub mod settings;
pub mod theme_downloader;
pub mod update_checker;
pub mod wifi;

// ===================================
// SCREEN RENDERING
// ===================================

// BACKGROUND
pub fn render_background(
    background_cache: &HashMap<String, Texture2D>,
    config: &Config,
    state: &mut BackgroundState,
) {
    if let Some(background_texture) = background_cache.get(&config.background_selection) {
        let tint_color = if config.color_shift_speed == "OFF" {
            WHITE
        } else {
            state.bg_color
        };

        if config.background_scroll_speed == "OFF" {
            // --- Static Logic (Stretch to fill) ---
            draw_texture_ex(
                background_texture,
                0.0,
                0.0,
                tint_color,
                DrawTextureParams {
                    dest_size: Some(vec2(screen_width(), screen_height())),
                            ..Default::default()
                },
            );
        } else {
            // --- Scrolling Logic (Scale with aspect ratio) ---
            let speed = match config.background_scroll_speed.as_str() {
                "SLOW" => 0.05, "NORMAL" => 0.1, "FAST" => 0.2, _ => 0.0
            };

            // Calculate new width and height while preserving aspect ratio
            let aspect_ratio = background_texture.width() / background_texture.height();
            let scaled_height = screen_height();
            let scaled_width = scaled_height * aspect_ratio;

            let params = DrawTextureParams {
                dest_size: Some(vec2(scaled_width, scaled_height)),
                ..Default::default()
            };

            // Update scrolling position based on the new scaled width
            state.bgx = (state.bgx + speed) % scaled_width;

            // Draw the two textures for a seamless loop
            draw_texture_ex(background_texture, state.bgx - scaled_width, 0.0, tint_color, params.clone());
            draw_texture_ex(background_texture, state.bgx, 0.0, tint_color, params);
        }

        // --- COLOR SHIFTING LOGIC ---
        let transition_speed = match config.color_shift_speed.as_str() {
            "SLOW" => 0.05,
            "NORMAL" => 0.1,
            "FAST" => 0.2,
            _ => 0.0, // This covers "OFF"
        };

        // Only run the color update logic if shifting is enabled
        if transition_speed > 0.0 {
            let frame_time = get_frame_time();

            state.bg_color.r += (state.tg_color.r - state.bg_color.r) * transition_speed * frame_time;
            state.bg_color.g += (state.tg_color.g - state.bg_color.g) * transition_speed * frame_time;
            state.bg_color.b += (state.tg_color.b - state.bg_color.b) * transition_speed * frame_time;

            // --- CORRECTED LOGIC ---
            // Check if all three channels are close to the target
            let red_done = (state.bg_color.r - state.tg_color.r).abs() < 0.01;
            let green_done = (state.bg_color.g - state.tg_color.g).abs() < 0.01;
            let blue_done = (state.bg_color.b - state.tg_color.b).abs() < 0.01;

            if red_done && green_done && blue_done {
                state.target = (state.target + 1) % COLOR_TARGETS.len();
                state.tg_color = COLOR_TARGETS[state.target];
            }
        }
    } else {
        clear_background(UI_BG_COLOR);
    }
}


// UI
pub fn render_ui_overlay(
    logo_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    battery_info: &Option<BatteryInfo>,
    current_time_str: &str,
    gcc_adapter_poll_rate: &Option<u32>,
    scale_factor: f32,
) {
    const BASE_LOGO_WIDTH: f32 = 200.0;

    let current_font = get_current_font(font_cache, config);
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let padding = 20.0 * scale_factor;

    // --- UPDATED: Dynamic Logo Drawing ---
    if config.logo_selection != "None" {
        if let Some(logo_to_draw) = logo_cache.get(&config.logo_selection) {
            // Calculate the scaled width and height while preserving aspect ratio
            let aspect_ratio = logo_to_draw.height() / logo_to_draw.width();
            let scaled_logo_width = BASE_LOGO_WIDTH * scale_factor;
            let scaled_logo_height = scaled_logo_width * aspect_ratio;

            // Center the logo horizontally
            let x_pos = (screen_width() - scaled_logo_width) / 2.0;
            let y_pos = 30.0 * scale_factor; // Scale the vertical position as well

            draw_texture_ex(
                logo_to_draw,
                x_pos,
                y_pos,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(scaled_logo_width, scaled_logo_height)),
                    source: Some(Rect::new(0.0, 0.0, logo_to_draw.width(), logo_to_draw.height())),
                    ..Default::default()
                },
            );
        }
    }

    // Clock
    let time_dims = measure_text(current_time_str, Some(current_font), font_size, 1.0);

    // If the menu is in the top-right, move the clock to the top-left.
    let time_x = if config.menu_position == MenuPosition::TopRight {
        20.0 * scale_factor
    } else {
        screen_width() - time_dims.width - (20.0 * scale_factor)
    };
    text_with_config_color(
        font_cache,
        config,
        current_time_str,
        time_x,
        20.0 * scale_factor,
        font_size,
    );

    // Battery
    if let Some(info) = battery_info {
        let status_symbol = match info.status.as_str() {
            "Charging" => "+",
            "Discharging" => "-",
            "Full" => "✓",
            _ => " ", // For "Unknown" or other statuses
        };

        // print battery
        let battery_text = format!("BATTERY: {}% {}", info.percentage, status_symbol);
        let batt_dims = measure_text(&battery_text, Some(current_font), font_size, 1.0);

        // If the menu is in the top-right, move the clock to the top-left.
        let batt_x = if config.menu_position == MenuPosition::TopRight {
            20.0 * scale_factor
        } else {
            screen_width() - batt_dims.width - (20.0 * scale_factor)
        };
        text_with_config_color(
            font_cache,
            config,
            &battery_text,
            batt_x,
            40.0 * scale_factor,
            font_size,
        );
    }

    // GCC Adapter Poll Rate
    if let Some(rate) = gcc_adapter_poll_rate {
        let gcc_text = format!("GCC: {}Hz", rate);
        let gcc_dims = measure_text(&gcc_text, Some(current_font), font_size, 1.0);

        // Position it in the same corner as the battery/clock
        let gcc_x = if config.menu_position == MenuPosition::TopRight {
            20.0 * scale_factor
        } else {
            screen_width() - gcc_dims.width - (20.0 * scale_factor)
        };

        // Draw it below the battery line
        text_with_config_color(
            font_cache,
            config,
            &gcc_text,
            gcc_x,
            60.0 * scale_factor, // Below the battery's 40.0
            font_size,
        );
    }

    // --- Version Number Drawing (now fully dynamic) ---
    let version_dims = measure_text(VERSION_NUMBER, Some(current_font), font_size, 1.0);

    // If the menu is in the bottom-right, move the version to the bottom-left.
    let version_x = if config.menu_position == MenuPosition::BottomRight {
        20.0 * scale_factor
    } else {
        screen_width() - version_dims.width - (20.0 * scale_factor)
    };
    text_with_config_color(
        font_cache,
        config,
        VERSION_NUMBER,
        version_x, // Position from right edge
        screen_height() - padding, // Position from bottom edge
        font_size,
    );
}

// GAME SELECTION
pub fn render_game_selection_menu(
    games: &[(save::CartInfo, PathBuf)],
    game_icon_cache: &HashMap<String, Texture2D>,
    placeholder: &Texture2D,
    selected_game: usize,
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
    render_ui_overlay(logo_cache, font_cache, config, battery_info, current_time_str, gcc_adapter_poll_rate, scale_factor);

    const TILE_SIZE: f32 = 60.0;
    const PADDING: f32 = 10.0;

    let scaled_tile_size = TILE_SIZE * scale_factor;
    let scaled_padding = PADDING * scale_factor;

    // --- 1. Define the Content Area ---
    // The logo's Y position is `30.0 * scale_factor`. Let's give it some space.
    let content_area_start_y = 100.0 * scale_factor;
    let content_area_height = screen_height() - content_area_start_y - (80.0 * scale_factor); // Leave space at bottom for text

    // --- 2. Calculate Grid Dimensions ---
    let grid_width_items = 5;
    let grid_height_items = (games.len() as f32 / grid_width_items as f32).ceil() as usize;

    let total_grid_width = (grid_width_items as f32 * scaled_tile_size) + ((grid_width_items - 1) as f32 * scaled_padding);
    let total_grid_height = (grid_height_items as f32 * scaled_tile_size) + ((grid_height_items - 1) as f32 * scaled_padding);

    // --- 3. Calculate Centered Starting Position (within the content area) ---
    let start_x = (screen_width() - total_grid_width) / 2.0;
    let start_y = content_area_start_y + (content_area_height - total_grid_height) / 2.0;

    // --- 4. Draw the Grid of Icons (this loop is unchanged) ---
    for (i, (cart_info, _)) in games.iter().enumerate() {
        let x = i % grid_width_items;
        let y = i / grid_width_items;

        let pos_x = start_x + (x as f32 * (scaled_tile_size + scaled_padding));
        let pos_y = start_y + (y as f32 * (scaled_tile_size + scaled_padding));

        let icon = game_icon_cache.get(&cart_info.id).unwrap_or(placeholder);

        // Draw background box for the icon
        draw_rectangle(pos_x, pos_y, scaled_tile_size, scaled_tile_size, RECT_COLOR);

        // Draw the icon
        draw_texture_ex(icon, pos_x, pos_y, WHITE, DrawTextureParams {
            dest_size: Some(vec2(scaled_tile_size, scaled_tile_size)),
                        ..Default::default()
        });

        // Draw selection highlight
        if i == selected_game {
            let cursor_color = animation_state.get_cursor_color(config);
            let cursor_scale = animation_state.get_cursor_scale();

            // The base size of the highlight is the tile size plus a small border
            let base_size = scaled_tile_size + (6.0 * scale_factor);
            let scaled_size = base_size * cursor_scale;
            let offset = (scaled_size - base_size) / 2.0;

            draw_rectangle_lines(
                pos_x - (3.0 * scale_factor) - offset,
                                 pos_y - (3.0 * scale_factor) - offset,
                                 scaled_size,
                                 scaled_size,
                                 6.0 * scale_factor, // Line thickness
                                 cursor_color
            );
        }
    }

    // --- Draw Selected Game Name (Subtitle) ---
    if let Some((cart_info, _)) = games.get(selected_game) {
        let name = cart_info.name.as_deref().unwrap_or(&cart_info.id);
        let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
        let text_dims = measure_text(name, None, font_size, 1.0);

        let text_x = screen_width() / 2.0 - text_dims.width / 2.0;
        let text_y = screen_height() - (40.0 * scale_factor);

        text_with_config_color(font_cache, config, name, text_x, text_y, font_size);
    }
}

// DEBUG
pub fn render_debug_screen(
    log_messages: &[String], // Takes a slice of strings
    scroll_offset: usize,
    flash_message: Option<&str>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    scale_factor: f32,
    background_cache: &HashMap<String, Texture2D>,
    background_state: &mut BackgroundState,
) {
    // --- Render the screen ---
    render_background(background_cache, config, background_state);

    let font_size = (12.0 * scale_factor) as u16;
    let line_height = font_size as f32 + (4.0 * scale_factor);
    let x_pos = 20.0 * scale_factor;
    //let top_margin = 20.0 * scale_factor;

    // Calculate how many lines can fit on the screen
    //let max_lines = ((screen_height() - (top_margin * 2.0)) / line_height).floor() as usize;

    // Determine which part of the log to show
    let start_index = scroll_offset;

    // Draw only the visible lines, starting from the scroll offset
    for (i, message) in log_messages.iter().skip(start_index).enumerate() {
        let y_pos = (20.0 * scale_factor) + (i as f32 * line_height);
        // Stop drawing if we go off the bottom of the screen
        if y_pos > screen_height() - (20.0 * scale_factor) {
            break;
        }
        text_with_config_color(font_cache, config, message, x_pos, y_pos, font_size);
    }

    // --- Draw the instruction or flash message ---
    let instruction_text = flash_message.unwrap_or("PRESS A TO SAVE LOG (OR B TO EXIT)");
    let instruction_font_size = (14.0 * scale_factor) as u16;
    let instruction_text_width = measure_text(instruction_text, None, instruction_font_size, 1.0).width;
    let instruction_x = (screen_width() - instruction_text_width) / 2.0; // Center it
    let instruction_y = screen_height() - (5.0 * scale_factor); // Position near the bottom

    draw_text(instruction_text, instruction_x, instruction_y, instruction_font_size as f32, WHITE);
}

// DIALOG BOX
pub fn render_dialog_box(
    message: &str,
    options: Option<(&str, &str)>,
    selection: usize,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    scale_factor: f32,
    animation_state: &AnimationState,
) {
    let current_font = get_current_font(font_cache, config);
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;

    // --- Box Dimensions ---
    let box_width = 400.0 * scale_factor;
    let box_height = 150.0 * scale_factor;
    let box_x = screen_width() / 2.0 - box_width / 2.0;
    let box_y = screen_height() / 2.0 - box_height / 2.0;

    // --- Draw Background and Border ---
    draw_rectangle(box_x, box_y, box_width, box_height, Color::new(0.0, 0.0, 0.0, 0.8));
    draw_rectangle_lines(box_x, box_y, box_width, box_height, 2.0, WHITE);

    // --- Draw Message Text (handles multiple lines) ---
    let mut line_y = box_y + 30.0 * scale_factor;
    for line in message.lines() {
        let text_dims = measure_text(line, Some(current_font), font_size, 1.0);
        let text_x = screen_width() / 2.0 - text_dims.width / 2.0;
        text_with_config_color(font_cache, config, line, text_x, line_y, font_size);
        line_y += text_dims.height + 5.0 * scale_factor;
    }

    // --- Draw Options (YES/NO or just OK) ---
    if let Some((opt1, opt2)) = options {
        let option_y = box_y + box_height - 40.0 * scale_factor;
        let yes_dims = measure_text(opt1, Some(current_font), font_size, 1.0);
        let no_dims = measure_text(opt2, Some(current_font), font_size, 1.0);

        // Calculate horizontal positions
        let spacing = 50.0 * scale_factor; // Space between YES and NO
        let total_width = yes_dims.width + no_dims.width + spacing;
        let yes_x = screen_width() / 2.0 - total_width / 2.0;
        let no_x = yes_x + yes_dims.width + spacing;

        // Determine which option is selected and its dimensions/position
        let (selected_x, _selected_text, selected_dims) = if selection == 0 {
            (yes_x, opt1, yes_dims)
        } else {
            (no_x, opt2, no_dims)
        };

        // --- Animated Cursor Drawing ---
        let cursor_color = animation_state.get_cursor_color(config);
        let cursor_scale = animation_state.get_cursor_scale();

        let base_width = selected_dims.width + (20.0 * scale_factor); // Padding around text
        let base_height = selected_dims.height + (10.0 * scale_factor); // Padding around text

        let scaled_width = base_width * cursor_scale;
        let scaled_height = base_height * cursor_scale;

        let offset_x = (scaled_width - base_width) / 2.0;
        let offset_y = (scaled_height - base_height) / 2.0;

        // Center the rect around the text
        let rect_x = selected_x - offset_x - ((base_width - selected_dims.width) / 2.0);

        // The total vertical padding we added
        let vertical_padding = 8.0 * scale_factor;

        // Adjust y_pos to account for half of the padding above the text
        let rect_y = option_y - selected_dims.height - offset_y - (vertical_padding / 2.0);

        draw_rectangle_lines(
            rect_x,
            rect_y,
            scaled_width,
            scaled_height,
            4.0 * scale_factor, // Border thickness
            cursor_color,
        );

        // Draw the YES/NO text
        text_with_config_color(font_cache, config, opt1, yes_x, option_y, font_size);
        text_with_config_color(font_cache, config, opt2, no_x, option_y, font_size);

    } else { // No options, just an "OK" implied for the Reset Complete screen
        let ok_text = "PRESS [SOUTH] TO RESTART";
        let text_dims = measure_text(ok_text, Some(current_font), font_size, 1.0);
        let text_x = screen_width() / 2.0 - text_dims.width / 2.0;
        let text_y = box_y + box_height - 40.0 * scale_factor;
        text_with_config_color(font_cache, config, ok_text, text_x, text_y, font_size);
    }
}

// DIALOG
pub fn render_dialog(
    dialog: &Dialog,
    memories: &Vec<Memory>,
    selected_memory: usize,
    icon_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    copy_op_state: &Arc<Mutex<CopyOperationState>>,
    placeholder: &Texture2D,
    scroll_offset: usize,
    animation_state: &AnimationState,
    playtime_cache: &mut PlaytimeCache,
    size_cache: &mut SizeCache,
    scale_factor: f32,
) {
    // --- Scaled variables are defined once at the top ---
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let tile_size = TILE_SIZE * scale_factor;
    let padding = PADDING * scale_factor;

    let current_font = get_current_font(font_cache, config);
    let (copy_progress, copy_running) = {
        if let Ok(state) = copy_op_state.lock() {
            (state.progress, state.running)
        } else {
            (0, false)
        }
    };

    // Dialog background - NOW SCALED
    if animation_state.dialog_transition_progress >= 1.0 {
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), UI_BG_COLOR_DIALOG);
    }

    // Game icon and name
    if let Some(mem) = memories.get(get_memory_index(selected_memory, scroll_offset)) {
        let icon = icon_cache.get(&mem.id).unwrap_or(placeholder);
        let params = DrawTextureParams { dest_size: Some(Vec2 { x: tile_size, y: tile_size }), ..Default::default() };
        let icon_pos = animation_state.get_dialog_transition_pos();
        draw_texture_ex(icon, icon_pos.x, icon_pos.y, WHITE, params);

        if animation_state.dialog_transition_progress >= 1.0 {
            let desc = mem.name.clone().unwrap_or_else(|| mem.id.clone());
            let playtime = get_game_playtime(mem, playtime_cache);
            let size = get_game_size(mem, size_cache);

            // Text calls - NOW SCALED
            text_with_config_color(font_cache, config, &desc, tile_size * 2.0, tile_size - (1.0 * scale_factor), font_size);
            let stats_text = format!("{:.1} MB | {:.1} H", size, playtime);
            text_with_config_color(font_cache, config, &stats_text, tile_size * 2.0, tile_size * 1.5 + (1.0 * scale_factor), font_size);
        }
    };

    // Copy progress bar - NOW SCALED
    if copy_running {
        draw_rectangle_lines(
            (font_size * 3) as f32, screen_height() / 2.0,
                             screen_width() - (font_size * 6) as f32, 1.2 * font_size as f32,
                             4.0 * scale_factor, WHITE
        );
        draw_rectangle(
            (font_size*3) as f32 + 0.2*font_size as f32, screen_height()/2.0 + 0.2*font_size as f32,
                       (screen_width() - (font_size*6) as f32 - 0.4*font_size as f32) * (copy_progress as f32 / 100.0),
                       0.8 * font_size as f32, WHITE
        );
    } else if animation_state.dialog_transition_progress >= 1.0 {
        if let Some(desc) = dialog.desc.clone() {
            let text_width = measure_text(&desc, Some(current_font), font_size, 1.0).width;
            let x_pos = (screen_width() - text_width) / 2.0;
            text_with_config_color(font_cache, config, &desc, x_pos, (font_size * 7) as f32, font_size);
        }

        // Centering and drawing dialog options - NOW SCALED
        let longest_width = measure_text( &dialog.options.iter() .find(|opt| opt.text.len() == dialog.options.iter().map(|opt| opt.text.len()).max().unwrap_or(0)) .map(|opt| opt.text.to_uppercase()).unwrap_or_default(), Some(current_font), font_size, 1.0).width;
        let options_start_x = (screen_width() - longest_width) / 2.0;

        for (i, option) in dialog.options.iter().enumerate() {
            let y_pos = (font_size * 10 + font_size * 2 * (i as u16)) as f32;
            let shake_offset = if option.disabled { animation_state.calculate_shake_offset(ShakeTarget::Dialog) * scale_factor } else { 0.0 };
            let x_pos = options_start_x + shake_offset;
            if option.disabled {
                text_disabled(font_cache, config, &option.text, x_pos, y_pos, font_size);
            } else {
                text_with_config_color(font_cache, config, &option.text, x_pos, y_pos, font_size);
            }
        }

        // Selection rectangle - NOW SCALED
        let selection_y = (font_size * 9 + font_size * 2 * (dialog.selection as u16)) as f32;
        let selected_option = &dialog.options[dialog.selection];
        let selection_shake = if selected_option.disabled { animation_state.calculate_shake_offset(ShakeTarget::Dialog) * scale_factor } else { 0.0 };

        let cursor_color = animation_state.get_cursor_color(config);
        let cursor_scale = animation_state.get_cursor_scale();
        let base_width = longest_width + (padding * 2.0); // Use scaled padding
        let base_height = (1.2 * font_size as f32) + (padding * 2.0); // Use scaled padding
        let scaled_width = base_width * cursor_scale;
        let scaled_height = base_height * cursor_scale;
        let offset_x = (scaled_width - base_width) / 2.0;
        let offset_y = (scaled_height - base_height) / 2.0;

        draw_rectangle_lines(
            options_start_x - padding + selection_shake - offset_x,
            selection_y - padding - offset_y,
            scaled_width, scaled_height, 4.0 * scale_factor, cursor_color
        );
    }
}

// ===================================
// CURSOR FUNCTIONS
// ===================================

pub fn pixel_pos(v: f32, scale_factor: f32) -> f32 {
    (PADDING + v * TILE_SIZE + v * PADDING) * scale_factor
}

pub fn get_memory_index(selected_memory: usize, scroll_offset: usize) -> usize {
    selected_memory + GRID_WIDTH * scroll_offset
}

pub fn calculate_icon_transition_positions(selected_memory: usize, scale_factor: f32) -> (Vec2, Vec2) {
    let xp = (selected_memory % GRID_WIDTH) as f32;
    let yp = (selected_memory / GRID_WIDTH) as f32;

    // Create scaled versions of constants used for positioning
    let grid_offset = GRID_OFFSET * scale_factor;
    let padding = PADDING * scale_factor;

    let grid_pos = Vec2::new(
        pixel_pos(xp, scale_factor),
                             pixel_pos(yp, scale_factor) + grid_offset
    );
    let dialog_pos = Vec2::new(padding, padding);
    (grid_pos, dialog_pos)
}

// ===================================
// TEXT RENDERING
// ===================================

/// Looks up the currently selected font in the cache.
/// Falls back to the "Default" font if the selection is not found.
pub fn get_current_font<'a>(
    font_cache: &'a HashMap<String, Font>,
    config: &Config,
) -> &'a Font {
    font_cache
    .get(&config.font_selection)
    .unwrap_or_else(|| &font_cache["Default"])
}

// A new function specifically for drawing text that respects the config color
pub fn text_with_config_color(font_cache: &HashMap<String, Font>, config: &Config, text: &str, x: f32, y: f32, font_size: u16) {
    let font = get_current_font(font_cache, config);

    // Shadow should scale with font size
    let shadow_offset = 1.0 * (font_size as f32 / FONT_SIZE as f32);

    // Shadow
    draw_text_ex(text, x + shadow_offset, y + shadow_offset, TextParams {
        font: Some(font),
        font_size,
        color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.9 },
        ..Default::default()
    });

    // Main Text (using the color from config)
    draw_text_ex(text, x, y, TextParams {
        font: Some(font),
        font_size,
        color: string_to_color(&config.font_color),
        ..Default::default()
    });
}

// text when "PLAY" or "COPY SESSION LOGS" is greyed out
pub fn text_disabled(font_cache: &HashMap<String, Font>, config: &Config, text : &str, x : f32, y: f32, font_size: u16) {
    let font = get_current_font(font_cache, config);
    let shadow_offset = 1.0 * (font_size as f32 / FONT_SIZE as f32);

    // SHADOW
    draw_text_ex(text, x + shadow_offset, y + shadow_offset, TextParams {
        font: Some(font),
        //font_size: font_size,
        font_size,
        color: Color {r:0.0, g:0.0, b:0.0, a:1.0},
        ..Default::default()
    });

    // MAIN TEXT
    draw_text_ex(text, x, y, TextParams {
        font: Some(font),
        //font_size: font_size,
        font_size,
        color: Color {r:0.4, g:0.4, b:0.4, a:1.0},
        ..Default::default()
    });
}
