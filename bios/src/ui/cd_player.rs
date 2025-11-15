use macroquad::prelude::*;
use rodio::Sink;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
    thread,
};
// use std::time::Duration; // FIXED: Removed unused import
use crate::{
    audio::SoundEffects,
    config::Config,
    types::{AnimationState, BackgroundState, Screen},
    render_background, get_current_font, measure_text, text_with_config_color, InputState,
    //cd_player_backend::{CdPlayerBackend, PlayerStatus, update_playback_state}, // Import helper
    cd_player_backend::{CdPlayerBackend, PlayerStatus},
};

const TRACK_FONT_SIZE: u16 = 16;
const TRACK_PADDING: f32 = 8.0;
const TRACK_OPTION_HEIGHT: f32 = 30.0;

/// Holds the UI-specific state for the CD Player.
pub struct CdPlayerUiState {
    pub backend: Arc<Mutex<CdPlayerBackend>>,
    pub selected_track: usize,
    pub is_initialized: bool, // To track if we've scanned
}

impl CdPlayerUiState {
    pub fn new(backend: Arc<Mutex<CdPlayerBackend>>) -> Self {
        Self {
            backend,
            selected_track: 0,
            is_initialized: false,
        }
    }
}

/// Handles input and state logic for the CD Player.
pub fn update(
    ui_state: &mut CdPlayerUiState,
    input_state: &InputState,
    current_screen: &mut Screen,
    sound_effects: &SoundEffects,
    config: &Config,
    current_bgm: &mut Option<Sink>,
) {
    // Mute BGM while in this screen
    // Note: We do this every frame to ensure it stays muted,
    // but since we check for 'Some', it's very cheap.
    if let Some(sink) = current_bgm.as_ref() {
        if !sink.is_paused() {
            sink.pause(); // Better than set_volume(0.0), saves CPU!
        }
    }

    // We check for "song finished" first
    {
        let mut backend = ui_state.backend.lock().unwrap();
        if backend.status == PlayerStatus::Playing {
            // Check if the sink is empty
            let song_finished = backend.sink.as_ref().map_or(false, |s| s.empty());

            if song_finished {
                println!("[CD Player] Song finished. Advancing to next track.");

                let current_track = backend.current_track;
                let num_tracks = backend.toc.as_ref().map_or(0, |t| t.tracks.len());
                let next_track_index = current_track + 1;

                if next_track_index < num_tracks {
                    // --- Play the next track ---
                    // Update the UI cursor
                    ui_state.selected_track = next_track_index;

                    // Drop the lock so the 'play' thread can grab it
                    drop(backend);

                    CdPlayerBackend::play(
                        ui_state.backend.clone(),
                                          next_track_index
                    );
                } else {
                    // --- End of disc ---
                    // Last track finished. Stop.
                    backend.stop_internal();
                    backend.status = PlayerStatus::Stopped;
                    backend.track_duration = Duration::ZERO;
                }
            }
        }
    } // Lock is dropped here

    let mut backend = ui_state.backend.lock().unwrap();

    // Check if a playing track has finished
    //update_playback_state(&mut backend);

    // If we just entered this screen, scan the disc
    if !ui_state.is_initialized {
        ui_state.is_initialized = true;

        let backend_clone = ui_state.backend.clone();
        thread::spawn(move || {
            let mut backend = backend_clone.lock().unwrap();
            backend.scan_disc();
        });
        return; // Wait for scan to finish
    }

    // Don't process input if scanning or loading
    if backend.status == PlayerStatus::Scanning || backend.status == PlayerStatus::Loading {
        return;
    }

    if input_state.back {
        backend.stop(); // Stop music on exit
        *current_screen = Screen::Extras;
        sound_effects.play_back(config);

        // Resume BGM from theme
        if let Some(sink) = current_bgm.as_ref() {
            sink.play(); // Resume the paused sink
            sink.set_volume(config.bgm_volume); // Ensure volume is correct
        }

        ui_state.is_initialized = false; // Rescan next time
    }

    // --- Track List Navigation ---
    if let Some(toc) = &backend.toc {
        let num_tracks = toc.tracks.len();
        if !toc.tracks.is_empty() {
            let col_split_point = (num_tracks + 1) / 2;
            let current_track = ui_state.selected_track;
            let mut new_track = current_track;

            if input_state.up {
                if current_track == 0 { // At top of left col
                    new_track = num_tracks - 1; // Wrap to end
                } else if current_track == col_split_point { // At top of right col
                    new_track = col_split_point - 1; // Wrap to bottom of left
                } else {
                    new_track -= 1;
                }
            }
            if input_state.down {
                if current_track == num_tracks - 1 { // At end of right col
                    new_track = 0; // Wrap to start
                } else if current_track == col_split_point - 1 { // At bottom of left col
                    new_track = col_split_point; // Wrap to top of right
                } else {
                    new_track += 1;
                }
            }
            if input_state.left {
                if current_track >= col_split_point {
                    // Go from right column to left
                    new_track -= col_split_point;
                }
            }
            if input_state.right {
                if current_track < col_split_point {
                    // Go from left column to right
                    new_track += col_split_point;
                    // Clamp to end of list if right column is shorter
                    if new_track >= num_tracks {
                        new_track = num_tracks - 1;
                    }
                }
            }

            if new_track != current_track {
                ui_state.selected_track = new_track;
                sound_effects.play_cursor_move(config);
            }

            // seek
            let seek_duration = Duration::from_secs(15);
            if input_state.next { // 'next' for fast-forward
                backend.seek(seek_duration, true);
            }
            if input_state.prev { // 'prev' for rewind
                backend.seek(seek_duration, false);
            }

            if input_state.select {
                match backend.status {
                    PlayerStatus::Playing if backend.current_track == ui_state.selected_track => {
                        // It's playing and we're on the same track, so pause it.
                        backend.pause();
                    },
                    PlayerStatus::Paused if backend.current_track == ui_state.selected_track => {
                        // It's paused and we're on the same track, so resume it.
                        backend.resume();
                    },
                    _ => {
                        // It's stopped, or we're selecting a *different* track.
                        // We must drop the lock *before* calling play, as play spawns
                        // a thread that will also need to lock the backend.
                        drop(backend);

                        CdPlayerBackend::play(
                            ui_state.backend.clone(),
                            ui_state.selected_track
                        );
                        sound_effects.play_select(config);
                    }
                }
            }
        }
    }
}

/// Draws the CD Player UI.
pub fn draw(
    ui_state: &mut CdPlayerUiState,
    animation_state: &AnimationState,
    background_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    background_state: &mut BackgroundState,
    scale_factor: f32,
) {
    let backend = ui_state.backend.lock().unwrap();

    // [!] Note: I'm using your original constants, but renamed for clarity
    let font_size = (TRACK_FONT_SIZE as f32 * scale_factor) as u16;
    let menu_padding = TRACK_PADDING * scale_factor;
    let menu_option_height = (TRACK_OPTION_HEIGHT * 0.8) * scale_factor; // Tighter track list
    let current_font = get_current_font(font_cache, config);

    // --- Common UI ---
    render_background(background_cache, config, background_state);
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.5));

    let mut y_pos = 50.0 * scale_factor;

    // --- Draw Title ---
    let title = "MUSIC CD PLAYER";
    let title_dims = measure_text(title, Some(current_font), font_size, 1.0);
    text_with_config_color(font_cache, config, title, (screen_width() - title_dims.width) / 2.0, y_pos, font_size);
    y_pos += title_dims.height + (30.0 * scale_factor);

    // --- Draw based on Status ---
    match backend.status {
        PlayerStatus::Scanning => {
            let text = "SCANNING DISC...";
            let dims = measure_text(text, Some(current_font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, (screen_width() - dims.width) / 2.0, screen_height() / 2.0, font_size);
        }
        PlayerStatus::Loading => {
            let text = "LOADING TRACK...";
            let dims = measure_text(text, Some(current_font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, (screen_width() - dims.width) / 2.0, screen_height() / 2.0, font_size);
        }
        PlayerStatus::NoDisc => {
            let text = "NO DISC DETECTED";
            let dims = measure_text(text, Some(current_font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, (screen_width() - dims.width) / 2.0, screen_height() / 2.0, font_size);
        }
        PlayerStatus::DataDisc => {
            let text = "GAME DISC INSERTED\n(Not an Audio CD)";
            let dims = measure_text(text, Some(current_font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, (screen_width() - dims.width) / 2.0, screen_height() / 2.0, font_size);
        }
        _ => { // Stopped, Playing, Paused
            // --- Draw Track List (Two Column) ---
            if let Some(toc) = &backend.toc {
                let num_tracks = toc.tracks.len();
                if num_tracks > 0 {
                    // Calculate split point
                    // (num_tracks + 1) / 2 ensures the left column gets the extra track if odd
                    let col_split_point = (num_tracks + 1) / 2;

                    let list_start_y = y_pos;
                    let start_x_left = 40.0 * scale_factor;
                    // Adjust right column start based on your screen width
                    let start_x_right = screen_width() * 0.55;
                    // Make cursor width a bit wider to fit text
                    let cursor_width = 160.0 * scale_factor;

                    for (i, track) in toc.tracks.iter().enumerate() {
                        let (x, y);
                        let text = format!("Track {:02} ({:02}:{:02})", track.number, track.start_msf.0, track.start_msf.1);
                        let text_dims = measure_text(&text, Some(current_font), font_size, 1.0);

                        if i < col_split_point {
                            // --- Left Column ---
                            x = start_x_left;
                            y = list_start_y + (i as f32 * menu_option_height);
                        } else {
                            // --- Right Column ---
                            x = start_x_right;
                            let col_index = i - col_split_point;
                            y = list_start_y + (col_index as f32 * menu_option_height);
                        }

                        if i == ui_state.selected_track {
                            // Draw cursor
                            let cursor_color = animation_state.get_cursor_color(config);
                            draw_rectangle_lines(
                                x - menu_padding,
                                y - (text_dims.height / 2.0) - (menu_padding / 2.0) - 10.0,
                                cursor_width,
                                text_dims.height + menu_padding,
                                4.0 * scale_factor,
                                cursor_color,
                            );
                        }

                        text_with_config_color(font_cache, config, &text, x, y, font_size);
                    }
                }
            }

            // --- Draw Playback Status ---
            // (This is where the timeline would go)
            let status_y = screen_height() - (80.0 * scale_factor);
            let time_y = screen_height() - (60.0 * scale_factor);
            let status_x = 40.0 * scale_factor;

            let status_text = match backend.status {
                PlayerStatus::Playing => format!("PLAYING: Track {:02}", backend.current_track + 1),
                PlayerStatus::Paused => format!("PAUSED: Track {:02}", backend.current_track + 1),
                PlayerStatus::Stopped => "STOPPED".to_string(),
                _ => "".to_string(),
            };

            text_with_config_color(font_cache, config, &status_text, status_x, status_y, font_size);

            // --- Draw Timeline ---
            let mut elapsed_time = Duration::ZERO;
            let total_duration = backend.track_duration;

            if backend.status == PlayerStatus::Playing {
                if let Some(start_time) = backend.playback_start_time {
                    elapsed_time = start_time.elapsed();
                }
            } else if backend.status == PlayerStatus::Paused {
                elapsed_time = backend.paused_elapsed_time.unwrap_or(Duration::ZERO);
            }

            // prevent timer from going over
            elapsed_time = elapsed_time.min(total_duration);

            // Prevent divide by zero and clamp progress
            let mut progress = 0.0;
            if total_duration > Duration::ZERO {
                progress = (elapsed_time.as_secs_f32() / total_duration.as_secs_f32()).clamp(0.0, 1.0);
            }

            // Draw the timeline bar
            let bar_width = 200.0 * scale_factor; // Width of the progress bar
            let bar_height = 8.0 * scale_factor;
            let bar_x = status_x + (160.0 * scale_factor); // Position it next to status text
            let bar_y = status_y + (font_size as f32 / 2.0) - (bar_height / 2.0); // Align with status text

            // Draw background
            draw_rectangle(bar_x, bar_y, bar_width, bar_height, Color::new(0.1, 0.1, 0.1, 0.8));
            // Draw progress
            draw_rectangle(bar_x, bar_y, bar_width * progress, bar_height, animation_state.get_cursor_color(config));

            // Draw Time
            let time_text = format!(
                "{:02}:{:02} / {:02}:{:02}",
                elapsed_time.as_secs() / 60,
                elapsed_time.as_secs() % 60,
                total_duration.as_secs() / 60,
                total_duration.as_secs() % 60
            );
            text_with_config_color(font_cache, config, &time_text, status_x, time_y, font_size);
        }
    }

    // --- Draw Controls Help ---
    let help_text = "[SOUTH] PLAY/PAUSE | [EAST] BACK | [LB/RB] SEEK 15 SECONDS";
    let help_dims = measure_text(help_text, Some(current_font), (12.0 * scale_factor) as u16, 1.0);
    text_with_config_color(font_cache, config, help_text, (screen_width() - help_dims.width) / 2.0, screen_height() - (20.0 * scale_factor), (12.0 * scale_factor) as u16);
}
