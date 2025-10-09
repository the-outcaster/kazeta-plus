// Add necessary imports for the shared functions
use crate::config::Config;
use crate::{string_to_color, FONT_SIZE, BatteryInfo, MenuPosition, VERSION_NUMBER, BackgroundState, COLOR_TARGETS, UI_BG_COLOR,
    save, PathBuf, AnimationState, RECT_COLOR, Memory, Arc, Mutex, StorageMediaState, InputState, PlaytimeCache, SizeCache, TILE_SIZE,
    PADDING, GRID_OFFSET, SELECTED_OFFSET, GRID_WIDTH, UIFocus, pixel_pos, get_memory_index, UI_BG_COLOR_DARK, ShakeTarget,
    GRID_HEIGHT, get_game_playtime, get_game_size, Dialog, CopyOperationState, UI_BG_COLOR_DIALOG,
};
use macroquad::prelude::*;
use std::collections::HashMap;

pub mod about;
pub mod main_menu;
pub mod settings;

////////////////////////
// SCREEN RENDERING
////////////////////////

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

    // Battery and Clock
    if let Some(info) = battery_info {
        let status_symbol = match info.status.as_str() {
            "Charging" => "+",
            "Discharging" => "-",
            "Full" => "✓",
            _ => " ", // For "Unknown" or other statuses
        };

        // print clock
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
                              scale_factor: f32,
) {
    render_background(background_cache, config, background_state);
    render_ui_overlay(logo_cache, font_cache, config, battery_info, current_time_str, scale_factor);

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
        let ok_text = "PRESS A TO SHUT DOWN";
        let text_dims = measure_text(ok_text, Some(current_font), font_size, 1.0);
        let text_x = screen_width() / 2.0 - text_dims.width / 2.0;
        let text_y = box_y + box_height - 40.0 * scale_factor;
        text_with_config_color(font_cache, config, ok_text, text_x, text_y, font_size);
    }
}

// DATA VIEW
pub fn render_data_view(
    selected_memory: usize,
    memories: &Vec<Memory>,
    icon_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    storage_state: &Arc<Mutex<StorageMediaState>>,
    placeholder: &Texture2D,
    scroll_offset: usize,
    input_state: &mut InputState,
    animation_state: &mut AnimationState,
    playtime_cache: &mut PlaytimeCache,
    size_cache: &mut SizeCache,
    scale_factor: f32,
) {
    // --- Create scaled layout values at the top ---
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let tile_size = TILE_SIZE * scale_factor;
    let padding = PADDING * scale_factor;
    let grid_offset = GRID_OFFSET * scale_factor;
    let selected_offset = SELECTED_OFFSET * scale_factor;

    let xp = (selected_memory % GRID_WIDTH) as f32;
    let yp = (selected_memory / GRID_WIDTH) as f32;

    // Draw grid selection highlight when focused on grid
    if let UIFocus::Grid = input_state.ui_focus {
        let cursor_color = animation_state.get_cursor_color(config);
        let cursor_thickness = 6.0 * scale_factor;
        let cursor_scale = animation_state.get_cursor_scale();

        let base_size = tile_size + 6.0;
        let scaled_size = base_size * cursor_scale;
        let offset = (scaled_size - base_size) / 2.0;

        draw_rectangle_lines(
            //pixel_pos(xp)-3.0-SELECTED_OFFSET - offset,
            //pixel_pos(yp)-3.0-SELECTED_OFFSET+GRID_OFFSET - offset,
            pixel_pos(xp, scale_factor) - (3.0 * scale_factor) - selected_offset - offset,
                             pixel_pos(yp, scale_factor) - (3.0 * scale_factor) - selected_offset + grid_offset - offset,
                             scaled_size,
                             scaled_size,
                             cursor_thickness,
                             cursor_color
        );
    }

    for x in 0..GRID_WIDTH {
        for y in 0..GRID_HEIGHT {
            let memory_index = get_memory_index(x + GRID_WIDTH * y, scroll_offset);
            let pos_x = pixel_pos(x as f32, scale_factor);
            let pos_y = pixel_pos(y as f32, scale_factor) + grid_offset;

            if xp as usize == x && yp as usize == y {
                if let UIFocus::Grid = input_state.ui_focus {
                    draw_rectangle(pos_x-selected_offset, pos_y-selected_offset, tile_size, tile_size, UI_BG_COLOR);
                } else {
                    draw_rectangle(pos_x - (2.0 * scale_factor), pos_y- (2.0 * scale_factor), tile_size + (4.0 * scale_factor), tile_size + (4.0 * scale_factor), UI_BG_COLOR);
                }
            } else {
                draw_rectangle(pos_x - (2.0 * scale_factor), pos_y - (2.0 * scale_factor), tile_size + (4.0 * scale_factor), tile_size + (4.0 * scale_factor), UI_BG_COLOR);
            }

            let Some(mem) = memories.get(memory_index) else {
                continue;
            };

            // Skip rendering the icon at its grid position during transitions
            if xp as usize == x && yp as usize == y && animation_state.dialog_transition_time > 0.0 {
                continue;
            }

            let icon = match icon_cache.get(&mem.id) {
                Some(icon) => icon,
                None => placeholder,
            };

            let params = DrawTextureParams {
                dest_size: Some(Vec2 {x: tile_size, y: tile_size }),
                source: Some(Rect { x: 0.0, y: 0.0, h: icon.height(), w: icon.width() }),
                rotation: 0.0,
                flip_x: false,
                flip_y: false,
                pivot: None
            };
            if xp as usize == x && yp as usize == y {
                if let UIFocus::Grid = input_state.ui_focus {
                    draw_texture_ex(&icon, pos_x-selected_offset, pos_y-selected_offset, WHITE, params);
                } else {
                    draw_texture_ex(&icon, pos_x, pos_y, WHITE, params);
                }
            } else {
                draw_texture_ex(&icon, pos_x, pos_y, WHITE, params);
            }
        }
    }

    // --- Storage media info area (NOW FULLY SCALED) ---
    let storage_info_w = 512.0 * scale_factor;
    let storage_info_x = tile_size * 2.0;
    let storage_info_y = 16.0 * scale_factor;
    let storage_info_h = 36.0 * scale_factor;
    let nav_arrow_size = 10.0 * scale_factor;
    let nav_arrow_outline = 1.0 * scale_factor;
    let box_line_thickness = 4.0 * scale_factor;

    // Draw storage info background
    draw_rectangle(storage_info_x, storage_info_y, storage_info_w, storage_info_h, UI_BG_COLOR);
    draw_rectangle_lines(storage_info_x - box_line_thickness, storage_info_y - box_line_thickness, storage_info_w + (box_line_thickness * 2.0), storage_info_h + (box_line_thickness * 2.0), box_line_thickness, UI_BG_COLOR_DARK);

    if let Ok(state) = storage_state.lock() {
        if !state.media.is_empty() {
            // Draw storage info text (NOW in the correct, scaled box)
            text_with_config_color(font_cache, config, &state.media[state.selected].id.to_uppercase(), storage_info_x + (2.0 * scale_factor), storage_info_y + (17.0 * scale_factor), font_size);
            let free_space_text = format!("{} MB Free", state.media[state.selected].free as f32).to_uppercase();
            text_with_config_color(font_cache, config, &free_space_text, storage_info_x + (2.0 * scale_factor), storage_info_y + (33.0 * scale_factor), font_size);

            // Draw left arrow background
            let left_box_x = padding;  // Align with leftmost grid column
            let left_box_y = storage_info_y + storage_info_h / 2.0 - tile_size / 2.0;
            let left_shake = animation_state.calculate_shake_offset(ShakeTarget::LeftArrow);

            if let UIFocus::StorageLeft = input_state.ui_focus {
                let cursor_color = animation_state.get_cursor_color(config);
                let cursor_thickness = 6.0;
                let cursor_scale = animation_state.get_cursor_scale();

                let base_size = tile_size + 6.0;
                let scaled_size = base_size * cursor_scale;
                let offset = (scaled_size - base_size) / 2.0;

                draw_rectangle(left_box_x-selected_offset + left_shake, left_box_y-selected_offset, tile_size, tile_size, UI_BG_COLOR);
                draw_rectangle_lines(
                    left_box_x-3.0-selected_offset + left_shake - offset,
                    left_box_y-3.0-selected_offset - offset,
                    scaled_size,
                    scaled_size,
                    cursor_thickness,
                    cursor_color
                );
            } else {
                draw_rectangle(left_box_x-2.0 + left_shake, left_box_y-2.0, tile_size+4.0, tile_size+4.0, UI_BG_COLOR);
            }

            let left_offset = if let UIFocus::StorageLeft = input_state.ui_focus {
                selected_offset
            } else {
                0.0
            };

            let left_points = [
                Vec2::new(4.0 + left_box_x + tile_size/2.0 - nav_arrow_size - left_offset + left_shake, left_box_y + tile_size/2.0 - left_offset),
                Vec2::new(4.0 + left_box_x + tile_size/2.0 - left_offset + left_shake, left_box_y + tile_size/2.0 - nav_arrow_size - left_offset),
                Vec2::new(4.0 + left_box_x + tile_size/2.0 - left_offset + left_shake, left_box_y + tile_size/2.0 + nav_arrow_size - left_offset),
            ];
            let left_color = if state.selected > 0 {
                WHITE
            } else {
                Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 } // Dark gray when disabled
            };
            draw_triangle(left_points[0], left_points[1], left_points[2], left_color);
            draw_triangle_lines(left_points[0], left_points[1], left_points[2], nav_arrow_outline, BLACK);

            // Draw right arrow background
            let right_box_x = padding + (GRID_WIDTH as f32 - 1.0) * (tile_size + padding);  // Align with rightmost grid column
            let right_box_y = storage_info_y + storage_info_h / 2.0 - tile_size / 2.0;
            let right_shake = animation_state.calculate_shake_offset(ShakeTarget::RightArrow);

            if let UIFocus::StorageRight = input_state.ui_focus {
                let cursor_color = animation_state.get_cursor_color(config);
                let cursor_thickness = 6.0;
                let cursor_scale = animation_state.get_cursor_scale();

                let base_size = tile_size + 6.0;
                let scaled_size = base_size * cursor_scale;
                let offset = (scaled_size - base_size) / 2.0;

                draw_rectangle(right_box_x-selected_offset + right_shake, right_box_y-selected_offset, tile_size, tile_size, UI_BG_COLOR);
                draw_rectangle_lines(
                    right_box_x-3.0-selected_offset + right_shake - offset,
                    right_box_y-3.0-selected_offset - offset,
                    scaled_size,
                    scaled_size,
                    cursor_thickness,
                    cursor_color
                );
            } else {
                draw_rectangle(right_box_x-2.0 + right_shake, right_box_y-2.0, tile_size+4.0, tile_size+4.0, UI_BG_COLOR);
            }

            let right_offset = if let UIFocus::StorageRight = input_state.ui_focus {
                selected_offset
            } else {
                0.0
            };
            let right_points = [
                Vec2::new(right_box_x + tile_size/2.0 + nav_arrow_size - 4.0 - right_offset + right_shake, right_box_y + tile_size/2.0 - right_offset),
                Vec2::new(right_box_x + tile_size/2.0 - 4.0 - right_offset + right_shake, right_box_y + tile_size/2.0 - nav_arrow_size - right_offset),
                Vec2::new(right_box_x + tile_size/2.0 - 4.0 - right_offset + right_shake, right_box_y + tile_size/2.0 + nav_arrow_size - right_offset),
            ];
            let right_color = if state.selected < state.media.len() - 1 {
                WHITE
            } else {
                Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 } // Dark gray when disabled
            };
            draw_triangle(right_points[0], right_points[1], right_points[2], right_color);
            draw_triangle_lines(right_points[0], right_points[1], right_points[2], nav_arrow_outline, BLACK);
        }
    }

    // --- Draw highlight box for save info (NOW FULLY SCALED) ---
    draw_rectangle(16.0 * scale_factor, 309.0 * scale_factor, screen_width() - (32.0 * scale_factor), 40.0 * scale_factor, UI_BG_COLOR);
    draw_rectangle_lines(12.0 * scale_factor, 305.0 * scale_factor, screen_width() - (24.0 * scale_factor), 48.0 * scale_factor, box_line_thickness, UI_BG_COLOR_DARK);

    let memory_index = get_memory_index(selected_memory, scroll_offset);
    if input_state.ui_focus == UIFocus::Grid {
        if let Some(selected_mem) = memories.get(memory_index) {
            let desc = selected_mem.name.clone().unwrap_or_else(|| selected_mem.id.clone());
            let playtime = get_game_playtime(selected_mem, playtime_cache);
            let size = get_game_size(selected_mem, size_cache);
            let stats_text = format!("{:.1} MB | {:.1} H", size, playtime);

            // Draw save info text (NOW in the correct, scaled box)
            text_with_config_color(font_cache, config, &desc, 19.0 * scale_factor, 327.0 * scale_factor, font_size);
            text_with_config_color(font_cache, config, &stats_text, 19.0 * scale_factor, 345.0 * scale_factor, font_size);
        }
    }
    // --- Draw scroll indicators (NOW FULLY SCALED) ---
    let indicator_size = 8.0 * scale_factor;
    let distance_top = -13.0 * scale_factor;
    let distance_bottom = 4.0 * scale_factor;
    let outline_thickness = 1.0 * scale_factor;

    if scroll_offset > 0 {
        // Up arrow
        let center_x = screen_width() / 2.0;
        let top_y = grid_offset - distance_top;
        let points = [
            Vec2::new(center_x, top_y - indicator_size),
            Vec2::new(center_x - indicator_size, top_y),
            Vec2::new(center_x + indicator_size, top_y),
        ];
        draw_triangle(points[0], points[1], points[2], WHITE);
        draw_triangle_lines(points[0], points[1], points[2], outline_thickness, BLACK);
    }

    let next_row_start = get_memory_index(GRID_WIDTH * GRID_HEIGHT, scroll_offset);
    if next_row_start < memories.len() {
        // Down arrow
        let grid_bottom = grid_offset + GRID_HEIGHT as f32 * (tile_size + padding);
        let center_x = screen_width() / 2.0;
        let bottom_y = grid_bottom + distance_bottom;
        let points = [
            Vec2::new(center_x, bottom_y + indicator_size),
            Vec2::new(center_x - indicator_size, bottom_y),
            Vec2::new(center_x + indicator_size, bottom_y),
        ];
        draw_triangle(points[0], points[1], points[2], WHITE);
        draw_triangle_lines(points[0], points[1], points[2], outline_thickness, BLACK);
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
