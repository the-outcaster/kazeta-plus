use std::panic;
use futures;
use crate::{*, ui::dialog::*, memory::*}; // Use wildcards for convenience or specify each type
use crate::audio::SoundEffects;

// This function will handle all input and state changes for the data screen
pub async fn update(
    input_state: &mut InputState,
    current_screen: &mut Screen,
    sound_effects: &SoundEffects,
    config: &Config,
    storage_state: &Arc<Mutex<StorageMediaState>>,
    memories: &mut Vec<Memory>,
    icon_cache: &mut HashMap<String, Texture2D>,
    icon_queue: &mut Vec<(String, String)>,
    selected_memory: &mut usize,
    scroll_offset: &mut usize,
    dialogs: &mut Vec<Dialog>,
    dialog_state: &mut DialogState,
    animation_state: &mut AnimationState,
    scale_factor: f32,
    copy_op_state: &Arc<Mutex<CopyOperationState>>,
) {
    let mut action_dialog_id = String::new();
    let mut action_option_value = String::new();

    // Check if memories need to be refreshed due to storage media changes
    if let Ok(mut state) = storage_state.lock() {
        if state.needs_memory_refresh {
            if !state.media.is_empty() {
                *memories = load_memories(&state.media[state.selected], icon_cache, icon_queue).await;
            } else {
                *memories = Vec::new();
            }
            state.needs_memory_refresh = false;
            dialogs.clear();
        }
    }
    match dialog_state {
        DialogState::None => {
            // Handle back navigation
            if input_state.back {
                *current_screen = Screen::MainMenu;
                sound_effects.play_back(&config);
            }

            // Handle storage media switching with tab/bumpers regardless of focus
            if input_state.cycle || input_state.next || input_state.prev {
                if let Ok(mut state) = storage_state.lock() {
                    if input_state.cycle {
                        if state.media.len() > 1 {
                            // Cycle wraps around
                            state.selected = (state.selected + 1) % state.media.len();
                            *memories = load_memories(&state.media[state.selected], icon_cache, icon_queue).await;
                            *scroll_offset = 0;
                            sound_effects.play_select(&config);
                        }
                    } else if input_state.next {
                        // Next stops at end
                        if state.selected < state.media.len() - 1 {
                            state.selected += 1;
                            *memories = load_memories(&state.media[state.selected], icon_cache, icon_queue).await;
                            *scroll_offset = 0;
                            sound_effects.play_select(&config);
                        } else {
                            animation_state.trigger_shake(false); // Shake right arrow when can't go next
                            sound_effects.play_reject(&config);
                        }
                    } else if input_state.prev {
                        // Prev stops at beginning
                        if state.selected > 0 {
                            state.selected -= 1;
                            *memories = load_memories(&state.media[state.selected], icon_cache, icon_queue).await;
                            *scroll_offset = 0;
                            sound_effects.play_select(&config);
                        } else {
                            animation_state.trigger_shake(true); // Shake left arrow when can't go prev
                            sound_effects.play_reject(&config);
                        }
                    }
                }
            }

            match input_state.ui_focus {
                UIFocus::Grid => {
                    if input_state.select {
                        let memory_index = get_memory_index(*selected_memory, *scroll_offset);
                        if let Some(_) = memories.get(memory_index) {
                            let (grid_pos, dialog_pos) = calculate_icon_transition_positions(*selected_memory, scale_factor);
                            animation_state.trigger_dialog_transition(grid_pos, dialog_pos);
                            dialogs.push(create_main_dialog(&storage_state));
                            *dialog_state = DialogState::Opening;
                            sound_effects.play_select(&config);
                        }
                    }
                    if input_state.right && *selected_memory < GRID_WIDTH * GRID_HEIGHT - 1 {
                        *selected_memory += 1;
                        animation_state.trigger_transition(&config.cursor_transition_speed);
                        sound_effects.play_cursor_move(&config);
                    }
                    if input_state.left && *selected_memory >= 1 {
                        *selected_memory -= 1;
                        animation_state.trigger_transition(&config.cursor_transition_speed);
                        sound_effects.play_cursor_move(&config);
                    }
                    if input_state.down {
                        if *selected_memory < GRID_WIDTH * GRID_HEIGHT - GRID_WIDTH {
                            *selected_memory += GRID_WIDTH;
                            animation_state.trigger_transition(&config.cursor_transition_speed);
                            sound_effects.play_cursor_move(&config);
                        } else {
                            // Check if there are any saves in the next row
                            let next_row_start = get_memory_index(GRID_WIDTH * GRID_HEIGHT, *scroll_offset);
                            if next_row_start < memories.len() {
                                *scroll_offset += 1;
                                animation_state.trigger_transition(&config.cursor_transition_speed);
                                sound_effects.play_cursor_move(&config);
                            }
                        }
                    }
                    if input_state.up {
                        if *selected_memory >= GRID_WIDTH {
                            *selected_memory -= GRID_WIDTH;
                            animation_state.trigger_transition(&config.cursor_transition_speed);
                            sound_effects.play_cursor_move(&config);
                        } else if *scroll_offset > 0 {
                            *scroll_offset -= 1;
                            animation_state.trigger_transition(&config.cursor_transition_speed);
                            sound_effects.play_cursor_move(&config);
                        } else {
                            // Allow moving to storage navigation from leftmost or rightmost column
                            if *selected_memory % GRID_WIDTH == 0 {
                                input_state.ui_focus = UIFocus::StorageLeft;
                                animation_state.trigger_transition(&config.cursor_transition_speed);
                                sound_effects.play_cursor_move(&config);
                            } else if *selected_memory % GRID_WIDTH == GRID_WIDTH - 1 {
                                input_state.ui_focus = UIFocus::StorageRight;
                                animation_state.trigger_transition(&config.cursor_transition_speed);
                                sound_effects.play_cursor_move(&config);
                            }
                        }
                    }
                },
                UIFocus::StorageLeft => {
                    if input_state.right {
                        input_state.ui_focus = UIFocus::StorageRight;
                        animation_state.trigger_transition(&config.cursor_transition_speed);
                        sound_effects.play_cursor_move(&config);
                    }
                    if input_state.down {
                        input_state.ui_focus = UIFocus::Grid;
                        *selected_memory = 0; // Move to leftmost grid position
                        animation_state.trigger_transition(&config.cursor_transition_speed);
                        sound_effects.play_cursor_move(&config);
                    }
                    if input_state.select {
                        if let Ok(mut state) = storage_state.lock() {
                            if state.selected > 0 {
                                state.selected -= 1;
                                *memories = load_memories(&state.media[state.selected], icon_cache, icon_queue).await;
                                *scroll_offset = 0;
                                sound_effects.play_select(&config);
                            } else {
                                animation_state.trigger_shake(true);
                                sound_effects.play_reject(&config);
                            }
                        }
                    }
                },
                UIFocus::StorageRight => {
                    if input_state.left {
                        input_state.ui_focus = UIFocus::StorageLeft;
                        animation_state.trigger_transition(&config.cursor_transition_speed);
                        sound_effects.play_cursor_move(&config);
                    }
                    if input_state.down {
                        input_state.ui_focus = UIFocus::Grid;
                        *selected_memory = GRID_WIDTH - 1; // Move to rightmost grid position
                        animation_state.trigger_transition(&config.cursor_transition_speed);
                        sound_effects.play_cursor_move(&config);
                    }
                    if input_state.select {
                        if let Ok(mut state) = storage_state.lock() {
                            if state.selected < state.media.len() - 1 {
                                state.selected += 1;
                                *memories = load_memories(&state.media[state.selected], icon_cache, icon_queue).await;
                                *scroll_offset = 0;
                                sound_effects.play_select(&config);
                            } else {
                                animation_state.trigger_shake(false);
                                sound_effects.play_reject(&config);
                            }
                        }
                    }
                },
            }
        },
        DialogState::Open => {
            // When dialog is fully open, only render the dialog
            if let Some(dialog) = dialogs.last_mut() {
                //render_dialog(dialog, &memories, *selected_memory, &icon_cache, &font_cache, &config, &copy_op_state, &placeholder, *scroll_offset, &animation_state, &mut playtime_cache, &mut size_cache, scale_factor);

                let mut selection: i32 = dialog.selection as i32 + dialog.options.len() as i32;
                if input_state.up {
                    selection -= 1;
                    animation_state.trigger_transition(&config.cursor_transition_speed);
                    sound_effects.play_cursor_move(&config);
                }

                if input_state.down {
                    selection += 1;
                    animation_state.trigger_transition(&config.cursor_transition_speed);
                    sound_effects.play_cursor_move(&config);
                }

                let mut cancel = false;
                if input_state.back {
                    cancel = true;
                }

                let next_selection = selection as usize % dialog.options.len();
                if next_selection != dialog.selection {
                    // Store the new selection to apply after we're done with the immutable borrow
                    let new_selection = next_selection;
                    dialog.selection = new_selection;
                } else {
                    // We need to handle the select input
                    if input_state.select {
                        let selected_option = &dialog.options[dialog.selection];
                        if !selected_option.disabled {
                            action_dialog_id = dialog.id.clone();
                            action_option_value = selected_option.value.clone();

                            if selected_option.value == "CANCEL" || selected_option.value == "OK" {
                                cancel = true;
                            } else {
                                sound_effects.play_select(&config);
                            }
                        } else {
                            animation_state.trigger_dialog_shake();
                            sound_effects.play_reject(&config);
                        }
                    }
                }

                if cancel {
                    let (grid_pos, dialog_pos) = calculate_icon_transition_positions(*selected_memory, scale_factor);
                    animation_state.trigger_dialog_transition(dialog_pos, grid_pos);
                    *dialog_state = DialogState::Closing;
                    sound_effects.play_back(&config);
                }
            }
            // Handle dialog actions
            match (action_dialog_id.as_str(), action_option_value.as_str()) {
                ("main", "COPY") => {
                    dialogs.push(create_copy_storage_dialog(&storage_state));
                },
                ("main", "DELETE") => {
                    dialogs.push(create_confirm_delete_dialog());
                },
                ("main", "CANCEL") => {
                    let (grid_pos, dialog_pos) = calculate_icon_transition_positions(*selected_memory, scale_factor);
                    animation_state.trigger_dialog_transition(dialog_pos, grid_pos);
                    *dialog_state = DialogState::Closing;
                    //sound_effects.play_back(&config);
                },
                ("confirm_delete", "DELETE") => {
                    if let Ok(mut state) = storage_state.lock() {
                        let memory_index = get_memory_index(*selected_memory, *scroll_offset);
                        if let Some(mem) = memories.get(memory_index) {
                            if let Err(e) = save::delete_save(&mem.id, &state.media[state.selected].id) {
                                dialogs.push(create_error_dialog(format!("ERROR: {}", e)));
                            } else {
                                state.needs_memory_refresh = true;
                                *dialog_state = DialogState::None;
                                sound_effects.play_back(&config);
                            }
                        }
                    }
                },
                ("confirm_delete", "CANCEL") => {
                    let (grid_pos, dialog_pos) = calculate_icon_transition_positions(*selected_memory, scale_factor);
                    animation_state.trigger_dialog_transition(dialog_pos, grid_pos);
                    *dialog_state = DialogState::Closing;
                    //sound_effects.play_back(&config);
                },
                ("copy_storage_select", target_id) if target_id != "CANCEL" => {
                    let memory_index = get_memory_index(*selected_memory, *scroll_offset);
                    let mem = memories[memory_index].clone();
                    let target_id = target_id.to_string();
                    if let Ok(state) = storage_state.lock() {
                        let to_media = StorageMedia { id: target_id, free: 0 };

                        // Check if save already exists
                        if check_save_exists(&mem, &to_media, icon_cache, icon_queue).await {
                            dialogs.push(create_save_exists_dialog());
                        } else {
                            let thread_state = copy_op_state.clone();
                            let from_media = state.media[state.selected].clone();
                            thread::spawn(move || {
                                copy_memory(&mem, &from_media, &to_media, thread_state);
                            });
                        }
                    }
                },
                ("copy_storage_select", "CANCEL") => {
                    let (grid_pos, dialog_pos) = calculate_icon_transition_positions(*selected_memory, scale_factor);
                    animation_state.trigger_dialog_transition(dialog_pos, grid_pos);
                    *dialog_state = DialogState::Closing;
                    sound_effects.play_back(&config);
                },
                ("save_exists", "OK") => {
                    let (grid_pos, dialog_pos) = calculate_icon_transition_positions(*selected_memory, scale_factor);
                    animation_state.trigger_dialog_transition(dialog_pos, grid_pos);
                    *dialog_state = DialogState::Closing;
                    sound_effects.play_back(&config);
                },
                ("error", "OK") => {
                    let (grid_pos, dialog_pos) = calculate_icon_transition_positions(*selected_memory, scale_factor);
                    animation_state.trigger_dialog_transition(dialog_pos, grid_pos);
                    *dialog_state = DialogState::Closing;
                    sound_effects.play_back(&config);
                },
                _ => {} // handles opening and closing states
            }

            if !icon_queue.is_empty() {
                let (cart_id, icon_path) = icon_queue.remove(0);
                let texture_future = load_texture(&icon_path);
                let texture_result = panic::catch_unwind(|| {
                    futures::executor::block_on(texture_future)
                });

                if let Ok(Ok(texture)) = texture_result {
                    icon_cache.insert(cart_id.clone(), texture);
                }
            }

            // Display any copy operation errors
            if let Ok(mut copy_state) = copy_op_state.lock() {
                if let Some(error_msg) = copy_state.error_message.take() {
                    dialogs.push(create_error_dialog(error_msg));
                    *dialog_state = DialogState::Opening;
                }
                if copy_state.should_clear_dialogs {
                    *dialog_state = DialogState::Closing;
                    copy_state.should_clear_dialogs = false;
                }
            }
        },
        _ => {}
    }
}

// This function will handle all drawing for the data screen
pub fn draw(
    selected_memory: usize,
    memories: &Vec<Memory>,
    icon_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    storage_state: &Arc<Mutex<StorageMediaState>>,
    placeholder: &Texture2D,
    scroll_offset: usize,
    input_state: &InputState,
    animation_state: &AnimationState,
    playtime_cache: &mut PlaytimeCache,
    size_cache: &mut SizeCache,
    _scale_factor: f32, // we're now ignoring this
    dialog_state: &DialogState,
) {
    // Calculate Safe Scale Factor & Centering Offsets
    // We assume the UI was designed for 640x360
    const BASE_W: f32 = 640.0;
    const BASE_H: f32 = 360.0;

    let scale_w = screen_width() / BASE_W;
    let scale_h = screen_height() / BASE_H;

    // Take the smaller of the two scales to ensure it fits on both axes
    let scale_factor = scale_w.min(scale_h);

    // Calculate offsets to center the UI
    let ui_w = BASE_W * scale_factor;
    let ui_h = BASE_H * scale_factor;
    let offset_x = (screen_width() - ui_w) / 2.0;
    let offset_y = (screen_height() - ui_h) / 2.0;

    // If we have extra vertical space (like in 4:3), use it to spread the UI.
    // We use 1/3 of the extra space for the top gap, and 1/3 for the bottom gap.
    let extra_h = (screen_height() - ui_h).max(0.0);
    let spread = extra_h / 3.0;

    // We moved Header UP by 1*spread, and Footer DOWN by 1*spread.
    // So we have 2*spread of extra vertical space to fill with the grid.
    let row_spread = if GRID_HEIGHT > 1 {
        (spread * 2.0) / (GRID_HEIGHT as f32 - 1.0)
    } else {
        0.0
    };

    if *dialog_state == DialogState::Opening || *dialog_state == DialogState::Closing || *dialog_state == DialogState::None {

        // During opening, only render the main view and the transitioning icon
        // Only render the icon during transition
        if animation_state.dialog_transition_time > 0.0 {
            let memory_index = get_memory_index(selected_memory, scroll_offset);
            if let Some(mem) = memories.get(memory_index) {
                let icon = match icon_cache.get(&mem.id) {
                    Some(icon) => icon,
                    None => &placeholder,
                };

                let params = DrawTextureParams {
                    dest_size: Some(Vec2 {x: TILE_SIZE, y: TILE_SIZE }),
                    source: Some(Rect { x: 0.0, y: 0.0, h: icon.height(), w: icon.width() }),
                    rotation: 0.0,
                    flip_x: false,
                    flip_y: false,
                    pivot: None
                };

                let icon_pos = animation_state.get_dialog_transition_pos();
                draw_texture_ex(&icon, icon_pos.x, icon_pos.y, WHITE, params);
            }
        }

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
                // Add offset_x to X and offset_y to Y
                offset_x + pixel_pos(xp, scale_factor) - (3.0 * scale_factor) - selected_offset - offset,
                offset_y + pixel_pos(yp, scale_factor) - (3.0 * scale_factor) - selected_offset + grid_offset - offset - spread + (yp * row_spread),
                scaled_size,
                scaled_size,
                cursor_thickness,
                cursor_color
            );
        }

        for x in 0..GRID_WIDTH {
            for y in 0..GRID_HEIGHT {
                let memory_index = get_memory_index(x + GRID_WIDTH * y, scroll_offset);

                // Add offsets to grid positions
                let pos_x = offset_x + pixel_pos(x as f32, scale_factor);
                let pos_y = offset_y + pixel_pos(y as f32, scale_factor) + grid_offset - spread + (y as f32 * row_spread);

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

        // --- Storage media info area ---
        let storage_info_w = 512.0 * scale_factor;

        // Add offset_x and offset_y to UI elements below
        let storage_info_x = offset_x + tile_size * 2.0;
        let storage_info_y = (offset_y + 16.0 * scale_factor) - spread;

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

                // Get free space in MB
                let free_mb = state.media[state.selected].free as f32;
                // Convert MB to GB
                let free_gb = free_mb / 1024.0;

                // Format to show GB with one decimal place
                let free_space_text = format!("{:.1} GB Free", free_gb).to_uppercase();
                text_with_config_color(font_cache, config, &free_space_text, storage_info_x + (2.0 * scale_factor), storage_info_y + (33.0 * scale_factor), font_size);

                // Draw left arrow background
                let left_box_x = offset_x + padding;
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
                let right_box_x = offset_x + padding + (GRID_WIDTH as f32 - 1.0) * (tile_size + padding);
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

        // --- Draw highlight box for save info ---
        // Add offsets to the save info box
        let save_info_x = offset_x + 16.0 * scale_factor;
        let save_info_y = (offset_y + 309.0 * scale_factor) + spread;

        let save_box_w = 640.0 * scale_factor - (32.0 * scale_factor); // Scale relative to 640 base

        draw_rectangle(save_info_x, save_info_y, save_box_w, 40.0 * scale_factor, UI_BG_COLOR);
        draw_rectangle_lines(save_info_x - (4.0*scale_factor), save_info_y - (4.0*scale_factor), save_box_w + (8.0 * scale_factor), 48.0 * scale_factor, box_line_thickness, UI_BG_COLOR_DARK);

        let memory_index = get_memory_index(selected_memory, scroll_offset);
        if input_state.ui_focus == UIFocus::Grid {
            if let Some(selected_mem) = memories.get(memory_index) {
                let desc = selected_mem.name.clone().unwrap_or_else(|| selected_mem.id.clone());
                let playtime = get_game_playtime(selected_mem, playtime_cache);
                let size = get_game_size(selected_mem, size_cache);
                let stats_text = format!("{:.1} MB | {:.1} H", size, playtime);

                // Use save_info_x/y for text positioning
                text_with_config_color(font_cache, config, &desc, save_info_x + (3.0 * scale_factor), save_info_y + (18.0 * scale_factor), font_size);
                text_with_config_color(font_cache, config, &stats_text, save_info_x + (3.0 * scale_factor), save_info_y + (36.0 * scale_factor), font_size);
            }
        }
        // --- Draw scroll indicators ---
        let indicator_size = 8.0 * scale_factor;
        let distance_top = -13.0 * scale_factor;
        let distance_bottom = 4.0 * scale_factor;
        let outline_thickness = 1.0 * scale_factor;

        if scroll_offset > 0 {
            // Up arrow
            // Center X relative to the screen, Y relative to grid (offset included)
            let center_x = screen_width() / 2.0;
            let top_y = (offset_y + grid_offset - spread) - distance_top;

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
            let grid_bottom = (offset_y + grid_offset - spread) + GRID_HEIGHT as f32 * (tile_size + padding + row_spread);
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
}
