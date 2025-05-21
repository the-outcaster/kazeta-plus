use macroquad::prelude::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
use std::collections::HashMap;
use gilrs::{Gilrs, Button, Axis};
use std::panic;
use futures;
use std::sync::atomic::{AtomicU16, Ordering};

mod save;

const SCREEN_WIDTH: i32 = 640;
const SCREEN_HEIGHT: i32 = 360;
const TILE_SIZE: f32 = 32.0;
const PADDING: f32 = 16.0;
const FONT_SIZE: u16 = 16;
const GRID_OFFSET: f32 = 52.0;
const GRID_WIDTH: usize = 13;
const GRID_HEIGHT: usize = 5;
const UI_BG_COLOR: Color = Color {r: 0.0, g: 0.0, b: 0.0, a: 0.5 };
const UI_BG_COLOR_DARK: Color = Color {r: 0.0, g: 0.0, b: 0.0, a: 0.3 };
const UI_BG_COLOR_DIALOG: Color = Color {r: 0.0, g: 0.0, b: 0.0, a: 0.8 };
const SELECTED_OFFSET: f32 = 5.0;


fn window_conf() -> Conf {
    Conf {
        window_title: "kazeta-bios".to_owned(),
        window_resizable: false,
        window_width: SCREEN_WIDTH,
        window_height: SCREEN_HEIGHT,
        high_dpi: false,
        fullscreen: false,

        ..Default::default()
    }
}

#[derive(Clone, Debug)]
struct Memory {
    id: String,
    name: Option<String>,
    size: u16,
}

#[derive(Clone, Debug)]
struct StorageMedia {
    id: String,
    free: u32,
}

struct DialogOption {
    text: String,
    value: String,
    disabled: bool,
}

struct Dialog {
    id: String,
    desc: Option<String>,
    options: Vec<DialogOption>,
    selection: usize,
}

struct CopyOperationState {
    progress: u16,
    running: bool,
    should_clear_dialogs: bool,
    error_message: Option<String>,
}

struct DrawContext {
    font: Font,
}

#[derive(Clone, Debug)]
enum UIFocus {
    Grid,
    StorageLeft,
    StorageRight,
}

struct InputState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    select: bool,
    next: bool,
    prev: bool,
    cycle: bool,
    analog_was_neutral: bool,
    ui_focus: UIFocus,
    shake_left: f32,  // Shake animation state for left arrow
    shake_right: f32, // Shake animation state for right arrow
}

impl InputState {
    const ANALOG_DEADZONE: f32 = 0.5;  // Increased deadzone for less sensitivity
    const SHAKE_DURATION: f32 = 0.2;    // Duration of shake animation in seconds
    const SHAKE_INTENSITY: f32 = 3.0;   // How far the arrow shakes

    fn new() -> Self {
        InputState {
            up: false,
            down: false,
            left: false,
            right: false,
            select: false,
            next: false,
            prev: false,
            cycle: false,
            analog_was_neutral: true,
            ui_focus: UIFocus::Grid,
            shake_left: 0.0,
            shake_right: 0.0,
        }
    }

    fn update_keyboard(&mut self) {
        self.up = is_key_pressed(KeyCode::Up);
        self.down = is_key_pressed(KeyCode::Down);
        self.left = is_key_pressed(KeyCode::Left);
        self.right = is_key_pressed(KeyCode::Right);
        self.select = is_key_pressed(KeyCode::Enter);
        self.next = is_key_pressed(KeyCode::RightBracket);
        self.prev = is_key_pressed(KeyCode::LeftBracket);
        self.cycle = is_key_pressed(KeyCode::Tab);
    }

    fn update_controller(&mut self, gilrs: &mut Gilrs) {
        // Handle button events
        while let Some(ev) = gilrs.next_event() {
            match ev.event {
                gilrs::EventType::ButtonPressed(Button::DPadUp, _) => self.up = true,
                gilrs::EventType::ButtonReleased(Button::DPadUp, _) => self.up = false,
                gilrs::EventType::ButtonPressed(Button::DPadDown, _) => self.down = true,
                gilrs::EventType::ButtonReleased(Button::DPadDown, _) => self.down = false,
                gilrs::EventType::ButtonPressed(Button::DPadLeft, _) => self.left = true,
                gilrs::EventType::ButtonReleased(Button::DPadLeft, _) => self.left = false,
                gilrs::EventType::ButtonPressed(Button::DPadRight, _) => self.right = true,
                gilrs::EventType::ButtonReleased(Button::DPadRight, _) => self.right = false,
                gilrs::EventType::ButtonPressed(Button::South, _) => self.select = true,
                gilrs::EventType::ButtonReleased(Button::South, _) => self.select = false,
                gilrs::EventType::ButtonPressed(Button::RightTrigger, _) => self.next = true,
                gilrs::EventType::ButtonReleased(Button::RightTrigger, _) => self.next = false,
                gilrs::EventType::ButtonPressed(Button::LeftTrigger, _) => self.prev = true,
                gilrs::EventType::ButtonReleased(Button::LeftTrigger, _) => self.prev = false,
                _ => {}
            }
        }

        // Handle analog stick input
        for (_, gamepad) in gilrs.gamepads() {
            let x = gamepad.value(Axis::LeftStickX);
            let y = gamepad.value(Axis::LeftStickY);

            // Apply deadzone to analog values
            let apply_deadzone = |value: f32| {
                if value.abs() < Self::ANALOG_DEADZONE {
                    0.0
                } else {
                    value
                }
            };

            let x = apply_deadzone(x);
            let y = apply_deadzone(y);

            // Check if stick is in neutral position
            let is_neutral = x.abs() < Self::ANALOG_DEADZONE && y.abs() < Self::ANALOG_DEADZONE;

            // Only trigger movement if stick was in neutral position last frame
            if self.analog_was_neutral {
                self.up = self.up || y > Self::ANALOG_DEADZONE;
                self.down = self.down || y < -Self::ANALOG_DEADZONE;
                self.left = self.left || x < -Self::ANALOG_DEADZONE;
                self.right = self.right || x > Self::ANALOG_DEADZONE;
            }

            // Update neutral state for next frame
            self.analog_was_neutral = is_neutral;
        }
    }

    fn update_shake(&mut self, delta_time: f32) {
        // Update left arrow shake
        if self.shake_left > 0.0 {
            self.shake_left = (self.shake_left - delta_time).max(0.0);
        }
        // Update right arrow shake
        if self.shake_right > 0.0 {
            self.shake_right = (self.shake_right - delta_time).max(0.0);
        }
    }

    fn trigger_shake(&mut self, is_left: bool) {
        if is_left {
            self.shake_left = Self::SHAKE_DURATION;
        } else {
            self.shake_right = Self::SHAKE_DURATION;
        }
    }
}

fn pixel_pos(v: f32) -> f32 {
    PADDING + v*TILE_SIZE + v*PADDING
}

fn copy_memory(memory: &Memory, from_media: &StorageMedia, to_media: &StorageMedia, state: Arc<Mutex<CopyOperationState>>) {
    if let Ok(mut copy_state) = state.lock() {
        copy_state.progress = 0;
        copy_state.running = true;
    }

    thread::sleep(time::Duration::from_millis(1000));

    let progress = Arc::new(AtomicU16::new(0));
    let progress_clone = progress.clone();

    // Spawn a thread to update the progress
    let state_clone = state.clone();
    thread::spawn(move || {
        while progress_clone.load(Ordering::SeqCst) < 100 {
            if let Ok(mut copy_state) = state_clone.lock() {
                copy_state.progress = progress_clone.load(Ordering::SeqCst);
            }
            thread::sleep(time::Duration::from_millis(100));
        }
    });

    if let Err(e) = save::copy_save(&memory.id, &from_media.id, &to_media.id, progress) {
        if let Ok(mut copy_state) = state.lock() {
            copy_state.running = false;
            copy_state.should_clear_dialogs = true;
            copy_state.error_message = Some(format!("Failed to copy save: {}", e));
        }
        return;
    }

    if let Ok(mut copy_state) = state.lock() {
        copy_state.progress = 100;
    }

    thread::sleep(time::Duration::from_millis(1000));

    if let Ok(mut copy_state) = state.lock() {
        copy_state.running = false;
        copy_state.should_clear_dialogs = true;
    }
}

async fn load_memories(media: &StorageMedia, cache: &mut HashMap<String, Texture2D>, queue: &mut Vec<(String, String)>) -> Vec<Memory> {
    let mut memories = Vec::new();

    if let Ok(details) = save::get_save_details(&media.id) {
        for (cart_id, name, icon_path, size) in details {
            if !cache.contains_key(&cart_id) {
                queue.push((cart_id.clone(), icon_path.clone()));
            }

            let m = Memory {
                id: cart_id,
                name: Some(name),
                size: size,
            };
            memories.push(m);
        }
    }

    memories
}

fn text(ctx : &DrawContext, text : &str, x : f32, y: f32) {
    draw_text_ex(&text.to_uppercase(), x+1.0, y+1.0, TextParams {
        font: Some(&ctx.font),
        font_size: FONT_SIZE,
        color: Color {r:0.0, g:0.0, b:0.0, a:0.9},
        ..Default::default()
    });
    draw_text_ex(&text.to_uppercase(), x, y, TextParams {
        font: Some(&ctx.font),
        font_size: FONT_SIZE,
        ..Default::default()
    });
}

fn text_disabled(ctx : &DrawContext, text : &str, x : f32, y: f32) {
    draw_text_ex(&text.to_uppercase(), x+1.0, y+1.0, TextParams {
        font: Some(&ctx.font),
        font_size: FONT_SIZE,
        color: Color {r:0.0, g:0.0, b:0.0, a:0.4},
        ..Default::default()
    });
    draw_text_ex(&text.to_uppercase(), x, y, TextParams {
        font: Some(&ctx.font),
        font_size: FONT_SIZE,
        color: Color {r:0.5, g:0.5, b:0.5, a:0.5},
        ..Default::default()
    });
}

#[derive(Clone, Debug)]
struct StorageMediaState {
    media: Vec<StorageMedia>,
    selected: usize,
    needs_memory_refresh: bool,
}

impl StorageMediaState {
    fn new() -> Self {
        StorageMediaState {
            media: Vec::new(),
            selected: 0,
            needs_memory_refresh: false,
        }
    }

    fn update_media(&mut self) {
        let mut new_media = Vec::new();

        if let Ok(devices) = save::list_devices() {
            for (id, free) in devices {
                new_media.push(StorageMedia {
                    id,
                    free,
                });
            }
        }

        // Done if media list has not changed
        if self.media.len() == new_media.len() &&
           !self.media.iter().zip(new_media.iter()).any(|(a, b)| a.id != b.id) {
            self.media = new_media; // update free space
            return;
        }

        // Try to keep the same device selected if it still exists
        let mut new_pos = 0;
        if let Some(old_selected_media) = self.media.get(self.selected) {
            if let Some(pos) = new_media.iter().position(|m| m.id == old_selected_media.id) {
                new_pos = pos;
            }
        }

        self.selected = new_pos;
        self.media = new_media;
        self.needs_memory_refresh = true;
    }
}

fn get_memory_index(selected_memory: usize, scroll_offset: usize) -> usize {
    selected_memory + GRID_WIDTH * scroll_offset
}

fn render_main_view(
    ctx: &DrawContext,
    selected_memory: usize,
    memories: &Vec<Memory>,
    icon_cache: &HashMap<String, Texture2D>,
    storage_state: &Arc<Mutex<StorageMediaState>>,
    placeholder: &Texture2D,
    scroll_offset: usize,
    input_state: &InputState,
) {
    let xp = (selected_memory % GRID_WIDTH) as f32;
    let yp = (selected_memory / GRID_WIDTH) as f32;

    // Draw grid selection highlight when focused on grid
    if let UIFocus::Grid = input_state.ui_focus {
        draw_rectangle_lines(pixel_pos(xp)-3.0-SELECTED_OFFSET, pixel_pos(yp)-3.0-SELECTED_OFFSET+GRID_OFFSET, TILE_SIZE+6.0, TILE_SIZE+6.0, 6.0, Color { r: 1.0, g: 1.0, b: 1.0, a: 0.8});
    }

    for x in 0..GRID_WIDTH {
        for y in 0..GRID_HEIGHT {
            let memory_index = get_memory_index(x + GRID_WIDTH * y, scroll_offset);

            if xp as usize == x && yp as usize == y {
                if let UIFocus::Grid = input_state.ui_focus {
                    draw_rectangle(pixel_pos(x as f32)-SELECTED_OFFSET, pixel_pos(y as f32)-SELECTED_OFFSET+GRID_OFFSET, TILE_SIZE, TILE_SIZE, UI_BG_COLOR);
                } else {
                    draw_rectangle(pixel_pos(x as f32)-2.0, pixel_pos(y as f32)+GRID_OFFSET-2.0, TILE_SIZE+4.0, TILE_SIZE+4.0, UI_BG_COLOR);
                }
            } else {
                draw_rectangle(pixel_pos(x as f32)-2.0, pixel_pos(y as f32)+GRID_OFFSET-2.0, TILE_SIZE+4.0, TILE_SIZE+4.0, UI_BG_COLOR);
            }

            let Some(mem) = memories.get(memory_index) else {
                continue;
            };

            let icon = match icon_cache.get(&mem.id) {
                Some(icon) => icon,
                None => placeholder,
            };

            let params = DrawTextureParams {
                dest_size: Some(Vec2 {x: TILE_SIZE, y: TILE_SIZE }),
                source: Some(Rect { x: 0.0, y: 0.0, h: icon.height(), w: icon.width() }),
                rotation: 0.0,
                flip_x: false,
                flip_y: false,
                pivot: None
            };
            if xp as usize == x && yp as usize == y {
                if let UIFocus::Grid = input_state.ui_focus {
                    draw_texture_ex(&icon, pixel_pos(x as f32)-SELECTED_OFFSET, pixel_pos(y as f32)-SELECTED_OFFSET+GRID_OFFSET, WHITE, params);
                } else {
                    draw_texture_ex(&icon, pixel_pos(x as f32), pixel_pos(y as f32)+GRID_OFFSET, WHITE, params);
                }
            } else {
                draw_texture_ex(&icon, pixel_pos(x as f32), pixel_pos(y as f32)+GRID_OFFSET, WHITE, params);
            }
        }
    }

    // Storage media info area with navigation
    const STORAGE_INFO_WIDTH: f32 = 512.0;
    const STORAGE_INFO_X: f32 = TILE_SIZE*2.0;
    const STORAGE_INFO_Y: f32 = 16.0;
    const STORAGE_INFO_HEIGHT: f32 = 36.0;
    const NAV_ARROW_SIZE: f32 = 10.0;
    const NAV_ARROW_DISTANCE: f32 = 4.0;
    const NAV_ARROW_OUTLINE: f32 = 1.0;

    // Draw storage info background
    draw_rectangle(STORAGE_INFO_X, STORAGE_INFO_Y, STORAGE_INFO_WIDTH, STORAGE_INFO_HEIGHT, UI_BG_COLOR);
    draw_rectangle_lines(STORAGE_INFO_X-4.0, STORAGE_INFO_Y-4.0, STORAGE_INFO_WIDTH+8.0, STORAGE_INFO_HEIGHT+8.0, 4.0, UI_BG_COLOR_DARK);

    if let Ok(state) = storage_state.lock() {
        if !state.media.is_empty() {
            // Draw left arrow background
            let left_box_x = PADDING;  // Align with leftmost grid column
            let left_box_y = STORAGE_INFO_Y + STORAGE_INFO_HEIGHT/2.0 - TILE_SIZE/2.0;
            let left_shake = if input_state.shake_left > 0.0 {
                (input_state.shake_left / InputState::SHAKE_DURATION * std::f32::consts::PI * 8.0).sin() * InputState::SHAKE_INTENSITY
            } else {
                0.0
            };

            if let UIFocus::StorageLeft = input_state.ui_focus {
                draw_rectangle(left_box_x-SELECTED_OFFSET + left_shake, left_box_y-SELECTED_OFFSET, TILE_SIZE, TILE_SIZE, UI_BG_COLOR);
                draw_rectangle_lines(left_box_x-3.0-SELECTED_OFFSET + left_shake, left_box_y-3.0-SELECTED_OFFSET, TILE_SIZE+6.0, TILE_SIZE+6.0, 6.0, Color { r: 1.0, g: 1.0, b: 1.0, a: 0.8});
            } else {
                draw_rectangle(left_box_x-2.0 + left_shake, left_box_y-2.0, TILE_SIZE+4.0, TILE_SIZE+4.0, UI_BG_COLOR);
            }

            let left_offset = if let UIFocus::StorageLeft = input_state.ui_focus {
                SELECTED_OFFSET
            } else {
                0.0
            };

            let left_points = [
                Vec2::new(4.0 + left_box_x + TILE_SIZE/2.0 - NAV_ARROW_SIZE - left_offset + left_shake, left_box_y + TILE_SIZE/2.0 - left_offset),
                Vec2::new(4.0 + left_box_x + TILE_SIZE/2.0 - left_offset + left_shake, left_box_y + TILE_SIZE/2.0 - NAV_ARROW_SIZE - left_offset),
                Vec2::new(4.0 + left_box_x + TILE_SIZE/2.0 - left_offset + left_shake, left_box_y + TILE_SIZE/2.0 + NAV_ARROW_SIZE - left_offset),
            ];
            let left_color = if state.selected > 0 {
                WHITE
            } else {
                Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 } // Dark gray when disabled
            };
            draw_triangle(left_points[0], left_points[1], left_points[2], left_color);
            draw_triangle_lines(left_points[0], left_points[1], left_points[2], NAV_ARROW_OUTLINE, BLACK);

            // Draw right arrow background
            let right_box_x = PADDING + (GRID_WIDTH as f32 - 1.0) * (TILE_SIZE + PADDING);  // Align with rightmost grid column
            let right_box_y = STORAGE_INFO_Y + STORAGE_INFO_HEIGHT/2.0 - TILE_SIZE/2.0;
            let right_shake = if input_state.shake_right > 0.0 {
                (input_state.shake_right / InputState::SHAKE_DURATION * std::f32::consts::PI * 8.0).sin() * InputState::SHAKE_INTENSITY
            } else {
                0.0
            };

            if let UIFocus::StorageRight = input_state.ui_focus {
                draw_rectangle(right_box_x-SELECTED_OFFSET + right_shake, right_box_y-SELECTED_OFFSET, TILE_SIZE, TILE_SIZE, UI_BG_COLOR);
                draw_rectangle_lines(right_box_x-3.0-SELECTED_OFFSET + right_shake, right_box_y-3.0-SELECTED_OFFSET, TILE_SIZE+6.0, TILE_SIZE+6.0, 6.0, Color { r: 1.0, g: 1.0, b: 1.0, a: 0.8});
            } else {
                draw_rectangle(right_box_x-2.0 + right_shake, right_box_y-2.0, TILE_SIZE+4.0, TILE_SIZE+4.0, UI_BG_COLOR);
            }

            let right_offset = if let UIFocus::StorageRight = input_state.ui_focus {
                SELECTED_OFFSET
            } else {
                0.0
            };
            let right_points = [
                Vec2::new(right_box_x + TILE_SIZE/2.0 + NAV_ARROW_SIZE - 4.0 - right_offset + right_shake, right_box_y + TILE_SIZE/2.0 - right_offset),
                Vec2::new(right_box_x + TILE_SIZE/2.0 - 4.0 - right_offset + right_shake, right_box_y + TILE_SIZE/2.0 - NAV_ARROW_SIZE - right_offset),
                Vec2::new(right_box_x + TILE_SIZE/2.0 - 4.0 - right_offset + right_shake, right_box_y + TILE_SIZE/2.0 + NAV_ARROW_SIZE - right_offset),
            ];
            let right_color = if state.selected < state.media.len() - 1 {
                WHITE
            } else {
                Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 } // Dark gray when disabled
            };
            draw_triangle(right_points[0], right_points[1], right_points[2], right_color);
            draw_triangle_lines(right_points[0], right_points[1], right_points[2], NAV_ARROW_OUTLINE, BLACK);

            // Draw storage info text
            text(&ctx, &state.media[state.selected].id, STORAGE_INFO_X + 2.0, STORAGE_INFO_Y + 17.0);
            text(&ctx, &format!("{} MB Free", state.media[state.selected].free), STORAGE_INFO_X + 2.0, STORAGE_INFO_Y + 33.0);
        }
    }

    // Draw highlight box for save info
    draw_rectangle(16.0, 309.0, SCREEN_WIDTH as f32 - 32.0, 40.0, UI_BG_COLOR);
    draw_rectangle_lines(12.0, 305.0, SCREEN_WIDTH as f32 - 24.0, 48.0, 4.0, UI_BG_COLOR_DARK);

    let memory_index = get_memory_index(selected_memory, scroll_offset);
    if let Some(selected_mem) = memories.get(memory_index) {
        let desc = match selected_mem.name.clone() {
            Some(name) => name,
            None => selected_mem.id.clone(),
        };

        text(&ctx, &desc, 19.0, 327.0);
        text(&ctx, &format!("{} MB", selected_mem.size.to_string()), 19.0, 345.0);
    }

    // Draw scroll indicators last so they appear on top
    const SCROLL_INDICATOR_SIZE: f32 = 8.0;  // Size from center to edge
    const SCROLL_INDICATOR_DISTANCE_TOP: f32 = -13.0;  // Distance from grid edge
    const SCROLL_INDICATOR_DISTANCE_BOTTOM: f32 = 4.0;  // Distance from grid edge
    const SCROLL_INDICATOR_OUTLINE: f32 = 1.0;  // Outline thickness

    if scroll_offset > 0 {
        // Up arrow (pointing up)
        let points = [
            Vec2::new(SCREEN_WIDTH as f32 / 2.0, GRID_OFFSET - SCROLL_INDICATOR_DISTANCE_TOP - SCROLL_INDICATOR_SIZE),
            Vec2::new(SCREEN_WIDTH as f32 / 2.0 - SCROLL_INDICATOR_SIZE, GRID_OFFSET - SCROLL_INDICATOR_DISTANCE_TOP),
            Vec2::new(SCREEN_WIDTH as f32 / 2.0 + SCROLL_INDICATOR_SIZE, GRID_OFFSET - SCROLL_INDICATOR_DISTANCE_TOP),
        ];
        draw_triangle(points[0], points[1], points[2], WHITE);
        draw_triangle_lines(points[0], points[1], points[2], SCROLL_INDICATOR_OUTLINE, BLACK);
    }

    let next_row_start = get_memory_index(GRID_WIDTH * GRID_HEIGHT, scroll_offset);
    if next_row_start < memories.len() {
        // Down arrow (pointing down)
        let grid_bottom = GRID_OFFSET + GRID_HEIGHT as f32 * (TILE_SIZE + PADDING);
        let points = [
            Vec2::new(SCREEN_WIDTH as f32 / 2.0, grid_bottom + SCROLL_INDICATOR_DISTANCE_BOTTOM + SCROLL_INDICATOR_SIZE),
            Vec2::new(SCREEN_WIDTH as f32 / 2.0 - SCROLL_INDICATOR_SIZE, grid_bottom + SCROLL_INDICATOR_DISTANCE_BOTTOM),
            Vec2::new(SCREEN_WIDTH as f32 / 2.0 + SCROLL_INDICATOR_SIZE, grid_bottom + SCROLL_INDICATOR_DISTANCE_BOTTOM),
        ];
        draw_triangle(points[0], points[1], points[2], WHITE);
        draw_triangle_lines(points[0], points[1], points[2], SCROLL_INDICATOR_OUTLINE, BLACK);
    }
}

fn render_dialog(
    ctx: &DrawContext,
    dialog: &Dialog,
    memories: &Vec<Memory>,
    selected_memory: usize,
    icon_cache: &HashMap<String, Texture2D>,
    copy_op_state: &Arc<Mutex<CopyOperationState>>,
    placeholder: &Texture2D,
    scroll_offset: usize,
) {
    let (copy_progress, copy_running) = {
        if let Ok(state) = copy_op_state.lock() {
            (state.progress, state.running)
        } else {
            (0, false)
        }
    };

    draw_rectangle(0.0, 0.0, SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32, UI_BG_COLOR_DIALOG);

    // draw game icon and name
    let memory_index = get_memory_index(selected_memory, scroll_offset);
    if let Some(mem) = memories.get(memory_index) {
        let icon = match icon_cache.get(&mem.id) {
            Some(icon) => icon,
            None => placeholder,
        };

        let params = DrawTextureParams {
            dest_size: Some(Vec2 {x: TILE_SIZE, y: TILE_SIZE }),
            source: Some(Rect { x: 0.0, y: 0.0, h: icon.height(), w: icon.width() }),
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
            pivot: None
        };

        draw_texture_ex(&icon, PADDING as f32, PADDING as f32, WHITE, params);

        let desc = match mem.name.clone() {
            Some(name) => name,
            None => mem.id.clone(),
        };
        text(&ctx, &desc, TILE_SIZE*2.0, TILE_SIZE-1.0);
        text(&ctx, &format!("{} MB", mem.size.to_string()), TILE_SIZE*2.0, TILE_SIZE*1.5+1.0);
    };

    if copy_running {
        draw_rectangle_lines(
            (FONT_SIZE*3) as f32,
            SCREEN_HEIGHT as f32 / 2.0,
            (SCREEN_WIDTH as u16 - FONT_SIZE*6) as f32,
            1.2*FONT_SIZE as f32,
            4.0,
            Color {r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
        );
        draw_rectangle(
            (FONT_SIZE*3) as f32 + 0.2*FONT_SIZE as f32,
            SCREEN_HEIGHT as f32 / 2.0 + 0.2*FONT_SIZE as f32,
            ((SCREEN_WIDTH as u16 - FONT_SIZE*6) as f32 - 0.4*FONT_SIZE as f32) * (copy_progress as f32 / 100.0),
            0.8*FONT_SIZE as f32,
            Color {r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
        );
    } else {
        if let Some(desc) = dialog.desc.clone() {
            text(&ctx, &desc, (FONT_SIZE*5) as f32, (FONT_SIZE*5) as f32);
        }

        // Find the longest option text for centering
        let longest_option = dialog.options.iter()
            .map(|opt| opt.text.len())
            .max()
            .unwrap_or(0);

        // Calculate the width of the longest option in pixels
        let longest_width = measure_text(&dialog.options.iter()
            .find(|opt| opt.text.len() == longest_option)
            .map(|opt| opt.text.to_uppercase())
            .unwrap_or_default(),
            Some(&ctx.font),
            FONT_SIZE,
            1.0).width;

        // Calculate the starting X position to center all options
        let options_start_x = (SCREEN_WIDTH as f32 - longest_width) / 2.0;

        // Add padding to the selection rectangle
        const SELECTION_PADDING_X: f32 = 16.0;  // Padding on each side
        const SELECTION_PADDING_Y: f32 = 4.0;   // Padding on top and bottom

        for (i, option) in dialog.options.iter().enumerate() {
            let y_pos = (FONT_SIZE*7 + FONT_SIZE*2*(i as u16)) as f32;
            if option.disabled {
                text_disabled(&ctx, &option.text, options_start_x, y_pos);
            } else {
                text(&ctx, &option.text, options_start_x, y_pos);
            }
        }

        // Draw selection rectangle with padding
        let selection_y = (FONT_SIZE*6 + FONT_SIZE*2*(dialog.selection as u16)) as f32;
        draw_rectangle_lines(
            options_start_x - SELECTION_PADDING_X,
            selection_y - SELECTION_PADDING_Y,
            longest_width + (SELECTION_PADDING_X * 2.0),
            1.2*FONT_SIZE as f32 + (SELECTION_PADDING_Y * 2.0),
            4.0,
            Color {r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
        );
    }
}

fn create_confirm_delete_dialog() -> Dialog {
    Dialog {
        id: "confirm_delete".to_string(),
        desc: Some("CONFIRM DELETE?".to_string()),
        options: vec![
            DialogOption {
                text: "DELETE".to_string(),
                value: "DELETE".to_string(),
                disabled: false,
            },
            DialogOption {
                text: "CANCEL".to_string(),
                value: "CANCEL".to_string(),
                disabled: false,
            }
        ],
        selection: 1,
    }
}

fn create_copy_storage_dialog(storage_state: &Arc<Mutex<StorageMediaState>>) -> Dialog {
    let mut options = Vec::new();
    if let Ok(state) = storage_state.lock() {
        for drive in state.media.iter() {
            if drive.id == state.media[state.selected].id {
                continue;
            }
            options.push(DialogOption {
                text: format!("{} ({} MB Free)", drive.id.clone(), drive.free),
                value: drive.id.clone(),
                disabled: false,
            });
        }
    }
    options.push(DialogOption {
        text: "CANCEL".to_string(),
        value: "CANCEL".to_string(),
        disabled: false,
    });

    Dialog {
        id: "copy_storage_select".to_string(),
        desc: Some("SELECT DESTINATION".to_string()),
        options,
        selection: 0,
    }
}

fn create_main_dialog(storage_state: &Arc<Mutex<StorageMediaState>>) -> Dialog {
    let has_external_devices = if let Ok(state) = storage_state.lock() {
        state.media.len() > 1
    } else {
        false
    };

    let options = vec![
        DialogOption {
            text: "COPY".to_string(),
            value: "COPY".to_string(),
            disabled: !has_external_devices,
        },
        DialogOption {
            text: "DELETE".to_string(),
            value: "DELETE".to_string(),
            disabled: false,
        },
        DialogOption {
            text: "CANCEL".to_string(),
            value: "CANCEL".to_string(),
            disabled: false,
        },
    ];

    Dialog {
        id: "main".to_string(),
        desc: None,
        options,
        selection: 0,
    }
}

async fn check_save_exists(memory: &Memory, target_media: &StorageMedia, icon_cache: &mut HashMap<String, Texture2D>, icon_queue: &mut Vec<(String, String)>) -> bool {
    let target_memories = load_memories(target_media, icon_cache, icon_queue).await;
    target_memories.iter().any(|m| m.id == memory.id)
}

fn create_save_exists_dialog() -> Dialog {
    Dialog {
        id: "save_exists".to_string(),
        desc: Some("SAVE DATA ALREADY EXISTS AT THIS LOCATION".to_string()),
        options: vec![
            DialogOption {
                text: "OK".to_string(),
                value: "OK".to_string(),
                disabled: false,
            }
        ],
        selection: 0,
    }
}

fn create_error_dialog(message: String) -> Dialog {
    Dialog {
        id: "error".to_string(),
        desc: Some(message),
        options: vec![
            DialogOption {
                text: "OK".to_string(),
                value: "OK".to_string(),
                disabled: false,
            }
        ],
        selection: 0,
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut dialogs: Vec<Dialog> = Vec::new();
    let font = load_ttf_font_from_bytes(include_bytes!("../november.ttf")).unwrap();
    let background = Texture2D::from_file_with_format(include_bytes!("../background.png"), Some(ImageFormat::Png));
    let placeholder = Texture2D::from_file_with_format(include_bytes!("../placeholder.png"), Some(ImageFormat::Png));
    let mut icon_cache: HashMap<String, Texture2D> = HashMap::new();
    let mut icon_queue: Vec<(String, String)> = Vec::new();
    let mut scroll_offset = 0;

    let ctx : DrawContext = DrawContext {
        font: font,
    };

    // Initialize gamepad support
    let mut gilrs = Gilrs::new().unwrap();
    let mut input_state = InputState::new();

    // Create thread-safe storage media state
    let storage_state = Arc::new(Mutex::new(StorageMediaState::new()));

    // Initialize storage media list
    if let Ok(mut state) = storage_state.lock() {
        state.update_media();
    };

    // Spawn background thread for storage media detection
    let thread_storage_state = storage_state.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(time::Duration::from_secs(1));
            if let Ok(mut state) = thread_storage_state.lock() {
                state.update_media();
            }
        }
    });

    let mut memories = Vec::new();
    let mut selected_memory = 0;

    let copy_op_state = Arc::new(Mutex::new(CopyOperationState {
        progress: 0,
        running: false,
        should_clear_dialogs: false,
        error_message: None,
    }));


    let mut bgx = 0.0;

    let color_targets: [Color; 6] = [
        Color { r: 1.0, g: 0.5, b: 0.5, a: 1.0 },
        Color { r: 1.0, g: 1.0, b: 0.5, a: 1.0 },
        Color { r: 0.5, g: 1.0, b: 0.5, a: 1.0 },
        Color { r: 0.5, g: 1.0, b: 1.0, a: 1.0 },
        Color { r: 0.5, g: 0.5, b: 1.0, a: 1.0 },
        Color { r: 1.0, g: 0.5, b: 1.0, a: 1.0 },
    ];

    let mut bg_color = color_targets[0].clone();
    let mut tg_color = color_targets[1].clone();

    let mut target = 1;

    const DELTA: f32 = 0.0001;

    loop {
        draw_texture(&background, bgx-(SCREEN_WIDTH as f32), 0.0, bg_color);
        draw_texture(&background, bgx, 0.0, bg_color);
        bgx = (bgx + 0.1) % (SCREEN_WIDTH as f32);

        if bg_color.r < tg_color.r {
            bg_color.r += DELTA;
        } else if bg_color.r > tg_color.r {
            bg_color.r -= DELTA;
        }

        if bg_color.g < tg_color.g {
            bg_color.g += DELTA;
        } else if bg_color.g > tg_color.g {
            bg_color.g -= DELTA;
        }

        if bg_color.b < tg_color.b {
            bg_color.b += DELTA;
        } else if bg_color.b > tg_color.b {
            bg_color.b -= DELTA;
        }

        if (bg_color.r - tg_color.r).abs() < 0.01 && (bg_color.g - tg_color.g).abs() < 0.01 && (bg_color.b - tg_color.b).abs() < 0.01 {
            target = (target + 1) % 6;
            tg_color = color_targets[target].clone();
        }

        // Check if memories need to be refreshed due to storage media changes
        if let Ok(mut state) = storage_state.lock() {
            if state.needs_memory_refresh {
                if !state.media.is_empty() {
                    memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                } else {
                    memories = Vec::new();
                }
                state.needs_memory_refresh = false;
                dialogs.clear();
            }
        }

        let mut action_dialog_id = String::new();
        let mut action_option_value = String::new();

        // Update input state from both keyboard and controller
        input_state.update_keyboard();
        input_state.update_controller(&mut gilrs);

        // Update shake animations
        input_state.update_shake(get_frame_time());

        match dialogs.last_mut() {
            None => {
                render_main_view(&ctx, selected_memory, &memories, &icon_cache, &storage_state, &placeholder, scroll_offset, &input_state);

                // Handle storage media switching with tab/bumpers regardless of focus
                if input_state.cycle || input_state.next || input_state.prev {
                    if let Ok(mut state) = storage_state.lock() {
                        if state.media.len() > 1 {
                            if input_state.cycle {
                                // Cycle wraps around
                                state.selected = (state.selected + 1) % state.media.len();
                                memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                                scroll_offset = 0;
                            } else if input_state.next {
                                // Next stops at end
                                if state.selected < state.media.len() - 1 {
                                    state.selected += 1;
                                    memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                                    scroll_offset = 0;
                                } else {
                                    input_state.trigger_shake(false); // Shake right arrow when can't go next
                                }
                            } else if input_state.prev {
                                // Prev stops at beginning
                                if state.selected > 0 {
                                    state.selected -= 1;
                                    memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                                    scroll_offset = 0;
                                } else {
                                    input_state.trigger_shake(true); // Shake left arrow when can't go prev
                                }
                            }
                        }
                    }
                }

                match input_state.ui_focus {
                    UIFocus::Grid => {
                        if input_state.right && selected_memory < GRID_WIDTH * GRID_HEIGHT - 1 {
                            selected_memory += 1;
                        }
                        if input_state.left && selected_memory >= 1 {
                            selected_memory -= 1;
                        }
                        if input_state.down {
                            if selected_memory < GRID_WIDTH * GRID_HEIGHT - GRID_WIDTH {
                                selected_memory += GRID_WIDTH;
                            } else {
                                // Check if there are any saves in the next row
                                let next_row_start = get_memory_index(GRID_WIDTH * GRID_HEIGHT, scroll_offset);
                                if next_row_start < memories.len() {
                                    scroll_offset += 1;
                                }
                            }
                        }
                        if input_state.up {
                            if selected_memory >= GRID_WIDTH {
                                selected_memory -= GRID_WIDTH;
                            } else if scroll_offset > 0 {
                                scroll_offset -= 1;
                            } else {
                                // Allow moving to storage navigation from leftmost or rightmost column
                                if selected_memory % GRID_WIDTH == 0 {
                                    input_state.ui_focus = UIFocus::StorageLeft;
                                } else if selected_memory % GRID_WIDTH == GRID_WIDTH - 1 {
                                    input_state.ui_focus = UIFocus::StorageRight;
                                }
                            }
                        }
                    },
                    UIFocus::StorageLeft => {
                        if input_state.right {
                            input_state.ui_focus = UIFocus::StorageRight;
                        }
                        if input_state.down {
                            input_state.ui_focus = UIFocus::Grid;
                            selected_memory = 0; // Move to leftmost grid position
                        }
                        if input_state.select {
                            if let Ok(mut state) = storage_state.lock() {
                                if state.selected > 0 {
                                    state.selected -= 1;
                                    memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                                    scroll_offset = 0;
                                } else {
                                    input_state.trigger_shake(true);
                                }
                            }
                        }
                    },
                    UIFocus::StorageRight => {
                        if input_state.left {
                            input_state.ui_focus = UIFocus::StorageLeft;
                        }
                        if input_state.down {
                            input_state.ui_focus = UIFocus::Grid;
                            selected_memory = GRID_WIDTH - 1; // Move to rightmost grid position
                        }
                        if input_state.select {
                            if let Ok(mut state) = storage_state.lock() {
                                if state.selected < state.media.len() - 1 {
                                    state.selected += 1;
                                    memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                                    scroll_offset = 0;
                                } else {
                                    input_state.trigger_shake(false);
                                }
                            }
                        }
                    },
                }

                if input_state.select {
                    match input_state.ui_focus {
                        UIFocus::Grid => {
                            let memory_index = get_memory_index(selected_memory, scroll_offset);
                            if let Some(_) = memories.get(memory_index) {
                                dialogs.push(create_main_dialog(&storage_state));
                            }
                        },
                        UIFocus::StorageLeft => {
                            if let Ok(mut state) = storage_state.lock() {
                                if state.selected > 0 {
                                    state.selected -= 1;
                                    memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                                    scroll_offset = 0;
                                }
                            }
                        },
                        UIFocus::StorageRight => {
                            if let Ok(mut state) = storage_state.lock() {
                                if state.selected < state.media.len() - 1 {
                                    state.selected += 1;
                                    memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                                    scroll_offset = 0;
                                }
                            }
                        },
                    }
                }
            },
            Some(dialog) => {
                render_dialog(&ctx, dialog, &memories, selected_memory, &icon_cache, &copy_op_state, &placeholder, scroll_offset);

                let mut selection: i32 = dialog.selection as i32 + dialog.options.len() as i32;
                if input_state.up {
                    selection -= 1;
                }

                if input_state.down {
                    selection += 1;
                }

                let next_selection = selection as usize % dialog.options.len();
                if next_selection != dialog.selection {
                    dialog.selection = next_selection;
                }

                if input_state.select {
                    let selected_option = &dialog.options[dialog.selection];
                    if !selected_option.disabled {
                        action_dialog_id = dialog.id.clone();
                        action_option_value = selected_option.value.clone();
                    }
                }

                if let Ok(mut state) = copy_op_state.lock() {
                    if state.should_clear_dialogs {
                        dialogs.clear();
                        state.should_clear_dialogs = false;
                    }
                }
            },
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
                dialogs.clear();
            },
            ("confirm_delete", "DELETE") => {
                if let Ok(mut state) = storage_state.lock() {
                    let memory_index = get_memory_index(selected_memory, scroll_offset);
                    if let Some(mem) = memories.get(memory_index) {
                        if let Err(e) = save::delete_save(&mem.id, &state.media[state.selected].id) {
                            dialogs.push(create_error_dialog(format!("ERROR: {}", e)));
                        } else {
                            state.needs_memory_refresh = true;
                            dialogs.clear();
                        }
                    }
                }
            },
            ("confirm_delete", "CANCEL") => {
                dialogs.clear();
            },
            ("copy_storage_select", target_id) if target_id != "CANCEL" => {
                let memory_index = get_memory_index(selected_memory, scroll_offset);
                let mem = memories[memory_index].clone();
                let target_id = target_id.to_string();
                if let Ok(state) = storage_state.lock() {
                    let to_media = StorageMedia { id: target_id, free: 0 };

                    // Check if save already exists
                    if check_save_exists(&mem, &to_media, &mut icon_cache, &mut icon_queue).await {
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
                dialogs.clear();
            },
            ("save_exists", "OK") => {
                dialogs.pop();
            },
            ("error", "OK") => {
                dialogs.clear();
            },
            _ => {}
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
            }
        }

        next_frame().await
    }
}
