use macroquad::prelude::*;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::io::{BufRead, BufReader};
use std::thread;
use std::time;
use std::collections::HashMap;

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
const KAZETA_BIN: &'static str = "kazeta";


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
}

#[derive(Clone, Debug)]
struct DialogOption {
    text: String,
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
    done: bool,
}

struct DrawContext {
    font: Font,
}

fn pixel_pos(v: f32) -> f32 {
    PADDING + v*TILE_SIZE + v*PADDING
}

fn copy_memory(memory: &Memory, from_media: &StorageMedia, to_media: &StorageMedia, state: Arc<Mutex<CopyOperationState>>) {

    if let Ok(mut copy_state) = state.lock() {
        copy_state.progress = 0;
        copy_state.running = true;
        copy_state.done = false;
    }

    thread::sleep(time::Duration::from_millis(1000));

    let mut cmd = Command::new(KAZETA_BIN)
    .args([ "save", "copy", &memory.id, &from_media.id, &to_media.id ])
    .stderr(Stdio::piped())
    .spawn()
    .expect("Failed to start command");

    let stderr = cmd.stderr.take().expect("Failed to capture stdout");
    let reader = BufReader::new(stderr);

    for line in reader.lines() {
        if let Ok(line) = line {
            if let Ok(value) = line.trim().parse::<u16>() {
                if value > 0 {
                    if let Ok(mut copy_state) = state.lock() {
                        copy_state.progress = value.min(100);
                    }
                }
            }
        }
    }

    if let Ok(mut copy_state) = state.lock() {
        copy_state.progress = 100;
    }

    thread::sleep(time::Duration::from_millis(1000));

    if let Ok(mut copy_state) = state.lock() {
        copy_state.running = false;
        copy_state.done = true;
    }

}

async fn remove_memory(memory: &Memory, from_media: &StorageMedia) {
    Command::new(KAZETA_BIN)
    .args([ "save", "delete", &memory.id, &from_media.id ])
    .output()
    .unwrap();
}

async fn load_memories(media: &StorageMedia, cache: &mut HashMap<String, Texture2D>) -> Vec<Memory> {
    let mut memories = Vec::new();

    let results = Command::new(KAZETA_BIN)
    .args([ "save", "details", &media.id ])
    .output()
    .unwrap();

    for (_, line) in String::from_utf8(results.stdout).unwrap().lines().enumerate() {
        let parts: Vec<&str> = line.split(":::").collect();

        if parts.len() != 4 {
            continue;
        }

        let cart_id = parts[0].trim().to_string();
        let name = parts[1].trim().to_string();
        let icon_path = parts[2].trim().to_string();
        let size = parts[3].trim().to_string().parse::<u16>().unwrap();

        if !cache.contains_key(&cart_id) {
            cache.insert(cart_id.clone(), load_texture(&icon_path).await.unwrap());
        }

        let m = Memory {
            id: cart_id,
            name: Some(name),
            size: size,
        };
        memories.push(m);
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
        let device_list_raw = Command::new(KAZETA_BIN)
            .args([ "device", "list" ])
            .output()
            .unwrap();

        let mut new_media = Vec::new();
        for device in String::from_utf8(device_list_raw.stdout).unwrap().lines() {
            let storage = StorageMedia { id: device.to_string() };
            new_media.push(storage);
        }

        // Done if media list has not changed
        if self.media.len() == new_media.len() &&
           !self.media.iter().zip(new_media.iter()).any(|(a, b)| a.id != b.id) {
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

#[macroquad::main(window_conf)]
async fn main() {
    let mut dialogs: Vec<Dialog> = Vec::new();
    let font = load_ttf_font_from_bytes(include_bytes!("../november.ttf")).unwrap();
    let background = Texture2D::from_file_with_format(include_bytes!("../background.png"), Some(ImageFormat::Png));
    let mut icon_cache: HashMap<String, Texture2D> = HashMap::new();


    let ctx : DrawContext = DrawContext {
        font: font,
    };

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
        done: false,
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
        clear_background(GREEN);

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
                    memories = load_memories(&state.media[state.selected], &mut icon_cache).await;
                } else {
                    memories = Vec::new();
                }
                state.needs_memory_refresh = false;
                dialogs.clear();
            }
        }

        match dialogs.last_mut() {
            None => {
                let xp = (selected_memory % GRID_WIDTH) as f32;
                let yp = (selected_memory / GRID_WIDTH) as f32;
                draw_rectangle_lines(pixel_pos(xp)-3.0-SELECTED_OFFSET, pixel_pos(yp)-3.0-SELECTED_OFFSET+GRID_OFFSET, TILE_SIZE+6.0, TILE_SIZE+6.0, 6.0, Color { r: 1.0, g: 1.0, b: 1.0, a: 0.8});

                for x in 0..GRID_WIDTH {
                    for y in 0..GRID_HEIGHT {

                        if xp as usize == x && yp as usize == y {
                            draw_rectangle( pixel_pos(x as f32)-SELECTED_OFFSET, pixel_pos(y as f32)-SELECTED_OFFSET+GRID_OFFSET, TILE_SIZE, TILE_SIZE, UI_BG_COLOR);
                        } else {
                            draw_rectangle( pixel_pos(x as f32)-2.0, pixel_pos(y as f32)+GRID_OFFSET-2.0, TILE_SIZE+4.0, TILE_SIZE+4.0, UI_BG_COLOR);
                        }

                        let Some(mem) = memories.get(x+GRID_WIDTH*y) else {
                            continue;
                        };

                        let Some(icon) = icon_cache.get(&mem.id) else {
                            continue;
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
                            draw_texture_ex(&icon, pixel_pos(x as f32)-SELECTED_OFFSET, pixel_pos(y as f32)-SELECTED_OFFSET+GRID_OFFSET, WHITE, params);
                        } else {
                            draw_texture_ex(&icon, pixel_pos(x as f32), pixel_pos(y as f32)+GRID_OFFSET, WHITE, params);
                        }
                    }
                }

                draw_rectangle( 16.0,310.0, 608.0, 36.0, UI_BG_COLOR);
                draw_rectangle_lines(16.0-4.0, 310.0-4.0, 608.0+8.0, 36.0+8.0, 4.0, UI_BG_COLOR_DARK);

                draw_rectangle( 16.0,16.0, 608.0, 36.0, UI_BG_COLOR);
                draw_rectangle_lines(16.0-4.0, 16.0-4.0, 608.0+8.0, 36.0+8.0, 4.0, UI_BG_COLOR_DARK);

                if let Ok(state) = storage_state.lock() {
                    if !state.media.is_empty() {
                        text(&ctx, &state.media[state.selected].id, 18.0, 33.0);
                        text(&ctx, "1 / 512 GB", 18.0, 49.0);
                    }
                }

                if let Some(selected_mem) = memories.get(selected_memory) {
                    let desc = match selected_mem.name.clone() {
                        Some(name) => name,
                        None => selected_mem.id.clone(),
                    };
                    text(&ctx, &desc, 18.0, 327.0);
                    text(&ctx, &format!("{} MB", selected_mem.size.to_string()), 18.0, 343.0);
                }

                if is_key_pressed(KeyCode::Right) && selected_memory < GRID_WIDTH * GRID_HEIGHT - 1 {
                    selected_memory += 1;
                }
                if is_key_pressed(KeyCode::Left) && selected_memory >= 1 {
                    selected_memory -= 1;
                }
                if is_key_pressed(KeyCode::Down) && selected_memory < GRID_WIDTH * GRID_HEIGHT - GRID_WIDTH {
                    selected_memory += GRID_WIDTH;
                }
                if is_key_pressed(KeyCode::Up) && selected_memory >= GRID_WIDTH {
                    selected_memory -= GRID_WIDTH;
                }

                if is_key_pressed(KeyCode::Tab) {
                    if let Ok(mut state) = storage_state.lock() {
                        if state.media.len() > 1 {
                            state.selected = (state.selected + 1) % state.media.len();
                            memories = load_memories(&state.media[state.selected], &mut icon_cache).await;
                        }
                    }
                }

                if is_key_pressed(KeyCode::Enter) {
                    if let Some(_) = memories.get(selected_memory) {
                        if let Ok(state) = storage_state.lock() {
                            let has_external_devices = state.media.len() > 1;
                            let options = vec![
                                DialogOption { text: "COPY".to_string(), disabled: !has_external_devices },
                                DialogOption { text: "DELETE".to_string(), disabled: false },
                                DialogOption { text: "CANCEL".to_string(), disabled: false },
                            ];
                            dialogs.push(Dialog { id: "main".to_string(), desc: None, options: options, selection: 0 });
                        }
                    }
                }
            },
            Some(dialog) => {

                let (copy_progress, copy_running, _copy_done) = {
                    if let Ok(state) = copy_op_state.lock() {
                        (state.progress, state.running, state.done)
                    } else {
                        (0, false, false)
                    }
                };

                draw_rectangle( 0.0,0.0, SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32, UI_BG_COLOR_DIALOG);

                // draw game icon and name
                if let Some(mem) = memories.get(selected_memory) {
                    if let Some(icon) = icon_cache.get(&mem.id) {
                        let params = DrawTextureParams {
                            dest_size: Some(Vec2 {x: TILE_SIZE, y: TILE_SIZE }),
                            source: Some(Rect { x: 0.0, y: 0.0, h: icon.height(), w: icon.width() }),
                            rotation: 0.0,
                            flip_x: false,
                            flip_y: false,
                            pivot: None
                        };

                        draw_texture_ex(&icon, PADDING as f32, PADDING as f32, WHITE, params);
                    };

                    let desc = match mem.name.clone() {
                        Some(name) => name,
                        None => mem.id.clone(),
                    };
                    text(&ctx, &desc, TILE_SIZE*2.0, TILE_SIZE);
                    text(&ctx, &format!("{} MB", mem.size.to_string()), TILE_SIZE*2.0, TILE_SIZE*1.5);
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
                        text(&ctx, &desc, 10.0, (FONT_SIZE*5) as f32);
                    }

                    for (i, option) in dialog.options.iter().enumerate() {
                        if option.disabled {
                            text_disabled(&ctx, &option.text, (FONT_SIZE*8) as f32, (FONT_SIZE*7 + FONT_SIZE*2*(i as u16)) as f32);
                        } else {
                            text(&ctx, &option.text, (FONT_SIZE*8) as f32, (FONT_SIZE*7 + FONT_SIZE*2*(i as u16)) as f32);
                        }
                    }

                    draw_rectangle_lines((FONT_SIZE*3) as f32, (FONT_SIZE*6 + FONT_SIZE*2*(dialog.selection as u16)) as f32, (SCREEN_WIDTH as u16 - FONT_SIZE*6) as f32, 1.2*FONT_SIZE as f32, 4.0, Color {r: 1.0, g: 1.0, b: 1.0, a: 1.0 });


                    let mut selection: i32 = dialog.selection as i32 + dialog.options.len() as i32;
                    if is_key_pressed(KeyCode::Up) {
                        selection -= 1;
                    }

                    if is_key_pressed(KeyCode::Down) {
                        selection += 1;
                    }

                    dialog.selection = selection as usize % dialog.options.len();


                    let selected_option = &dialog.options[dialog.selection];
                    if is_key_pressed(KeyCode::Enter) && !selected_option.disabled {
                        if dialog.id == "main" {
                            if selected_option.text == "COPY" {
                                let mut options = Vec::new();
                                if let Ok(state) = storage_state.lock() {
                                    for drive in state.media.iter() {
                                        if drive.id == state.media[state.selected].id {
                                            continue;
                                        }
                                        options.push(DialogOption { text: drive.id.clone(), disabled: false });
                                    }
                                }
                                options.push(DialogOption { text: "CANCEL".to_string(), disabled: false });
                                dialogs.push(Dialog {
                                    id: "copy_storage_select".to_string(),
                                    desc: Some("COPY TO WHERE?".to_string()),
                                    options: options,
                                    selection: 0
                                });
                            } else if selected_option.text == "DELETE" {
                                dialogs.push(Dialog {
                                    id: "confirm_delete".to_string(),
                                    desc: Some("CONFIRM DELETE?".to_string()),
                                    options: vec![
                                        DialogOption { text: "DELETE".to_string(), disabled: false },
                                        DialogOption { text: "CANCEL".to_string(), disabled: false }
                                    ],
                                    selection: 1
                                });
                            } else if selected_option.text == "CANCEL" {
                                dialogs.pop();
                            }
                        } else if dialog.id == "confirm_delete" {
                            if selected_option.text == "CANCEL" {
                                dialogs.clear();
                            } else if selected_option.text == "DELETE" {
                                if let Ok(state) = storage_state.lock() {
                                    remove_memory(&memories[selected_memory], &state.media[state.selected]).await;
                                    memories = load_memories(&state.media[state.selected], &mut icon_cache).await;
                                }
                                dialogs.clear();
                            }
                        } else if dialog.id == "copy_storage_select" {
                            if selected_option.text == "CANCEL" {
                                dialogs.clear();
                            } else {
                                let thread_state = copy_op_state.clone();
                                let mem = memories[selected_memory].clone();
                                if let Ok(state) = storage_state.lock() {
                                    let from_media = state.media[state.selected].clone();
                                    let to_media = StorageMedia { id: selected_option.text.clone() };
                                    thread::spawn(move || {
                                        copy_memory(&mem, &from_media, &to_media, thread_state);
                                    });
                                }
                            }
                        }
                    }
                }
            },
        };

        next_frame().await
    }
}
