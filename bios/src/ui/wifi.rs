use crate::{
    text_with_config_color, get_current_font, DEV_MODE, VideoPlayer,
    audio::SoundEffects,
    config::Config, FONT_SIZE, Screen, BackgroundState, render_background, measure_text, InputState,
    ui::text_with_color,
};
use macroquad::prelude::*;
use std::{
    collections::HashMap,
    process::Command,
    sync::mpsc::{channel, Receiver, Sender},
    thread,
};

// Define the keyboard layout
const OSK_LAYOUT_LOWER: &[&str] = &[
    "1234567890!@#$%^()",
    "qwertyuiop\\~-=+[]&",
    "asdfghjkl |;:'\"<>*",
    "zxcvbnm   _./?`{},",
];

const OSK_LAYOUT_UPPER: &[&str] = &[
    "1234567890!@#$%^()",
    "QWERTYUIOP\\~-=+[]&",
    "ASDFGHJKL |;:'\"<>*",
    "ZXCVBNM   _./?`{},",
];

const OSK_SPECIAL_KEYS: &[&str] = &["SHOW", "SHIFT", "SPACE", "BACKSPACE", "ENTER"];

// [!] MODIFIED: Added 'security' field
#[derive(Debug, Clone)]
pub struct AccessPoint {
    pub ssid: String,
    pub signal_level: u8,
    pub security: String,
}

#[derive(PartialEq)]
pub enum WifiScreenState {
    Preparing,
    Scanning,
    List,
    PasswordInput,
    Connecting,
    Connected,
    Error(String),
}

enum WifiMessage {
    PreparationComplete(Result<(), String>),
}

pub struct WifiState {
    pub screen_state: WifiScreenState,
    pub networks: Result<Vec<AccessPoint>, String>,
    pub selected_index: usize,
    pub password_buffer: String,
    pub osk_coords: (usize, usize),
    pub osk_shift_active: bool,
    pub show_password: bool,
    rx: Receiver<WifiMessage>,
    _tx: Sender<WifiMessage>,
}

impl WifiState {
    pub fn new() -> Self {
        let (tx, rx) = channel();

        prepare_wifi_system(tx.clone());

        Self {
            screen_state: WifiScreenState::Preparing,
            networks: Ok(Vec::new()),
            selected_index: 0,
            password_buffer: String::new(),
            osk_coords: (0, 0),
            osk_shift_active: false,
            show_password: false,
            rx,
            _tx: tx,
        }
    }

    /// Scans for networks using the `nmcli` command-line tool.
    pub fn scan_networks(&mut self) {
        self.screen_state = WifiScreenState::Scanning;

        // [!] MODIFIED: Added SECURITY to the fields list
        let output = Command::new("nmcli")
        .args(&["--terse", "--fields", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"])
        .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut aps: Vec<AccessPoint> = Vec::new();
                for line in stdout.lines() {
                    // [!] MODIFIED: Parse 3 parts instead of 2
                    let parts: Vec<&str> = line.split(':').collect();
                    // Note: Split might produce more than 3 parts if SSID contains colon,
                    // but --terse usually handles escaping. For simple safety:
                    if parts.len() >= 3 {
                        let ssid = parts[0];
                        let signal_str = parts[1];
                        let security = parts[2]; // "WPA2", "WPA1 WPA2", or "" (empty means Open)

                        if let Ok(signal) = signal_str.parse::<u8>() {
                            if !ssid.is_empty() {
                                aps.push(AccessPoint {
                                    ssid: ssid.to_string(),
                                    signal_level: signal,
                                    security: security.to_string(),
                                });
                            }
                        }
                    }
                }
                // Sort by signal strength, strongest first
                aps.sort_by(|a, b| b.signal_level.cmp(&a.signal_level));
                self.networks = Ok(aps);
            }
            Err(e) => {
                self.networks = Err(format!("Failed to run nmcli: {}", e));
            }
        }
        self.screen_state = WifiScreenState::List;
        self.selected_index = 0;
    }

    /// Attempts to connect to a network using `nmcli`.
    fn attempt_connection(&mut self) {
        if let Ok(networks) = &self.networks {
            if let Some(selected_network) = networks.get(self.selected_index) {
                self.screen_state = WifiScreenState::Connecting;
                let ssid = &selected_network.ssid;
                let password = &self.password_buffer;

                // [!] RESTORED: Delete any existing profile for this SSID first.
                // This prevents the "key-mgmt property is missing" error by ensuring
                // we create a fresh profile with the correct security settings.
                let _ = Command::new("nmcli")
                .args(&["connection", "delete", ssid])
                .output();

                // [!] MODIFIED: Logic to handle Open vs Secured networks
                let mut cmd = Command::new("nmcli");
                cmd.arg("device").arg("wifi").arg("connect").arg(ssid);

                // Only add password argument if the buffer isn't empty
                // OR check selected_network.security.
                // But trusting the buffer is safer if the scan was weird.
                if !password.is_empty() {
                    cmd.arg("password").arg(password);
                }

                // [!] ADDED: Explicitly ensure the new profile is saved and set to auto-connect
                // (Though nmcli defaults to this, being explicit helps with persistence)
                // Note: We can't pass these to 'device wifi connect' easily in one line
                // without complex syntax, but the default behavior is persistent.

                let output = cmd.output();

                match output {
                    Ok(output) => {
                        if output.status.success() {
                            self.screen_state = WifiScreenState::Connected;
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            self.screen_state = WifiScreenState::Error(stderr.trim().to_string());
                        }
                    }
                    Err(e) => {
                        self.screen_state = WifiScreenState::Error(format!("Failed to run nmcli: {}", e));
                    }
                }
            }
        }
    }
}

pub fn update(
    wifi_state: &mut WifiState,
    input_state: &InputState,
    current_screen: &mut Screen,
    sound_effects: &SoundEffects,
    config: &Config,
) {
    if let Ok(msg) = wifi_state.rx.try_recv() {
        match msg {
            WifiMessage::PreparationComplete(Ok(_)) => {
                wifi_state.scan_networks();
            }
            WifiMessage::PreparationComplete(Err(e)) => {
                wifi_state.screen_state = WifiScreenState::Error(e);
            }
        }
    }
    if input_state.back {
        if wifi_state.screen_state == WifiScreenState::PasswordInput && wifi_state.show_password {
            wifi_state.show_password = false;
            sound_effects.play_back(config);
            return;
        }

        if !matches!(wifi_state.screen_state, WifiScreenState::List) {
            wifi_state.screen_state = WifiScreenState::List;
            wifi_state.password_buffer.clear();
            sound_effects.play_back(config);
        } else {
            *current_screen = Screen::Extras;
            sound_effects.play_back(config);
        }
        return;
    }

    match &mut wifi_state.screen_state {
        WifiScreenState::PasswordInput => {
            let (row, col) = &mut wifi_state.osk_coords;
            let current_layout = if wifi_state.osk_shift_active { OSK_LAYOUT_UPPER } else { OSK_LAYOUT_LOWER };
            let num_rows = current_layout.len() + 1;

            if input_state.down && *row < num_rows - 1 { *row += 1; sound_effects.play_cursor_move(&config); }
            if input_state.up && *row > 0 { *row -= 1; sound_effects.play_cursor_move(&config); }

            let current_physical_row_len = if *row < current_layout.len() { current_layout[*row].len() } else { OSK_SPECIAL_KEYS.len() };
            if *col >= current_physical_row_len { *col = current_physical_row_len - 1; }

            if input_state.right && *col < current_physical_row_len - 1 { *col += 1; sound_effects.play_cursor_move(&config); }
            if input_state.left && *col > 0 { *col -= 1; sound_effects.play_cursor_move(&config); }

            if input_state.select {
                sound_effects.play_select(config);
                if *row < current_layout.len() {
                    if let Some(key) = current_layout[*row].chars().nth(*col) {
                        wifi_state.password_buffer.push(key);
                        if wifi_state.osk_shift_active && *row > 0 { wifi_state.osk_shift_active = false; }
                    }
                } else {
                    match OSK_SPECIAL_KEYS[*col] {
                        "SHOW" => wifi_state.show_password = !wifi_state.show_password,
                        "SHIFT" => wifi_state.osk_shift_active = !wifi_state.osk_shift_active,
                        "SPACE" => wifi_state.password_buffer.push(' '),
                        "BACKSPACE" => { wifi_state.password_buffer.pop(); },
                        "ENTER" => wifi_state.attempt_connection(),
                        _ => {}
                    }
                }
            }
        }
        WifiScreenState::List => {
            if let Ok(networks) = &wifi_state.networks {
                if networks.is_empty() { return; }
                if input_state.down && wifi_state.selected_index < networks.len() - 1 { wifi_state.selected_index += 1; sound_effects.play_cursor_move(&config); }
                if input_state.up && wifi_state.selected_index > 0 { wifi_state.selected_index -= 1; sound_effects.play_cursor_move(&config); }

                if input_state.select {
                    sound_effects.play_select(config);

                    // [!] MODIFIED: Check security before going to password screen
                    let selected_ap = &networks[wifi_state.selected_index];

                    // If security string is empty, it's an Open network
                    if selected_ap.security.is_empty() {
                        // Skip password input, connect immediately
                        wifi_state.password_buffer.clear(); // Ensure empty
                        wifi_state.attempt_connection();
                    } else {
                        // It's secured, go to input
                        wifi_state.password_buffer.clear();
                        wifi_state.osk_coords = (0, 0);
                        wifi_state.osk_shift_active = false;
                        wifi_state.show_password = false;
                        wifi_state.screen_state = WifiScreenState::PasswordInput;
                    }
                }
            }
        }
        WifiScreenState::Connected | WifiScreenState::Error(_) => {
            if input_state.select {
                sound_effects.play_select(config);
                wifi_state.screen_state = WifiScreenState::List;
            }
        }
        _ => {}
    }
}

pub fn draw(
    wifi_state: &WifiState,
    animation_state: &mut crate::AnimationState,
    background_cache: &HashMap<String, Texture2D>,
    video_cache: &mut HashMap<String, VideoPlayer>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    background_state: &mut BackgroundState,
    scale_factor: f32,
) {
    render_background(&background_cache, video_cache, &config, background_state);

    let font = get_current_font(font_cache, config);
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let line_height = font_size as f32 + 10.0 * scale_factor;
    let container_w = screen_width() * 0.8;
    let container_h = screen_height() * 0.7;
    let container_x = (screen_width() - container_w) / 2.0;
    let container_y = (screen_height() - container_h) / 2.0;
    draw_rectangle(container_x, container_y, container_w, container_h, Color::new(0.0, 0.0, 0.0, 0.75));
    let text_x = container_x + 40.0 * scale_factor;

    match &wifi_state.screen_state {
        WifiScreenState::Preparing => {
            let text = "Preparing network services...";
            let text_dims = measure_text(text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        WifiScreenState::PasswordInput => {
            if let Ok(networks) = &wifi_state.networks {
                if let Some(network) = networks.get(wifi_state.selected_index) {
                    let prompt = format!("Enter password for \"{}\":", network.ssid);
                    text_with_config_color(font_cache, config, &prompt, text_x, container_y + 40.0 * scale_factor, font_size);

                    let password_display: String = if wifi_state.show_password {
                        wifi_state.password_buffer.clone()
                    } else {
                        wifi_state.password_buffer.chars().map(|_| '*').collect()
                    };

                    let input_box_y = container_y + 60.0 * scale_factor + 10.0;
                    let input_box_height = line_height * 0.8;
                    let input_text_font_size = (font_size as f32 * 0.9) as u16;

                    draw_rectangle(text_x, input_box_y, container_w - 80.0 * scale_factor, input_box_height, BLACK);
                    let text_y_inside_box = input_box_y + (input_box_height / 2.0) + (input_text_font_size as f32 / 2.5);
                    draw_text_ex(&password_display, text_x + 10.0 * scale_factor, text_y_inside_box, TextParams { font: Some(font), font_size: input_text_font_size, color: WHITE, ..Default::default() });

                    // --- [!] FIX 1: DYNAMIC SCALING FOR 4:3 ASPECT RATIOS ---
                    // Calculate ideal sizing
                    let base_osk_size = (font_size as f32) as u16;
                    let base_spacing = base_osk_size as f32 * 1.5;

                    // Calculate available width for the keyboard
                    let available_width = container_w - 80.0 * scale_factor; // padding
                    let max_chars_in_row = OSK_LAYOUT_LOWER[0].len() as f32;

                    // Determine if we need to shrink
                    let needed_width = max_chars_in_row * base_spacing;

                    let (osk_font_size, key_spacing) = if needed_width > available_width {
                        // It's too wide (likely 4:3 ratio), shrink it to fit
                        let new_spacing = available_width / max_chars_in_row;
                        let new_size = (new_spacing / 1.5) as u16;
                        (new_size, new_spacing)
                    } else {
                        // It fits fine
                        (base_osk_size, base_spacing)
                    };
                    // -------------------------------------------------------

                    let osk_start_y = input_box_y + input_box_height + line_height * 1.2;

                    let cursor_color = animation_state.get_cursor_color(config);
                    let cursor_scale = animation_state.get_cursor_scale();
                    let line_thickness = 4.0 * cursor_scale;
                    let current_layout = if wifi_state.osk_shift_active { OSK_LAYOUT_UPPER } else { OSK_LAYOUT_LOWER };

                    // --- Character Grid Loop ---
                    for (r, row_str) in current_layout.iter().enumerate() {
                        for (c, key) in row_str.chars().enumerate() {
                            let key_str = key.to_string();
                            let text_dims = measure_text(&key_str, Some(font), osk_font_size, 1.0);
                            let cell_x = text_x + (c as f32 * key_spacing);
                            let text_draw_x = cell_x + (key_spacing - text_dims.width) / 2.0;
                            let key_y = osk_start_y + (r as f32 * key_spacing);

                            let is_selected = (r, c) == wifi_state.osk_coords;

                            // [!] FIX 2: Apply Cursor Styles to Characters
                            if is_selected && config.cursor_style == "BOX" {
                                let box_h = osk_font_size as f32 + 10.0;
                                let box_y = key_y - osk_font_size as f32 - 5.0;
                                draw_rectangle_lines(text_draw_x - 5.0, box_y, text_dims.width + 10.0, box_h, line_thickness, cursor_color);
                            }

                            if is_selected && config.cursor_style == "TEXT" {
                                text_with_color(font_cache, config, &key_str, text_draw_x, key_y, osk_font_size, cursor_color);
                            } else {
                                text_with_config_color(font_cache, config, &key_str, text_draw_x, key_y, osk_font_size);
                            }
                        }
                    }

                    // --- Special Keys Row ---
                    let special_row_y = osk_start_y + (current_layout.len() as f32 * key_spacing) + 20.0;
                    let key_gap = 40.0 * scale_factor;
                    let mut total_row_width = 0.0;
                    for key_str in OSK_SPECIAL_KEYS.iter() {
                        total_row_width += measure_text(key_str, Some(font), osk_font_size, 1.0).width;
                    }
                    total_row_width += ((OSK_SPECIAL_KEYS.len() - 1) as f32) * key_gap;

                    // Check if special row fits, scale gap if needed
                    let actual_key_gap = if total_row_width > available_width {
                        let text_width_sum: f32 = OSK_SPECIAL_KEYS.iter().map(|k| measure_text(k, Some(font), osk_font_size, 1.0).width).sum();
                        (available_width - text_width_sum) / (OSK_SPECIAL_KEYS.len() as f32 - 1.0)
                    } else {
                        key_gap
                    };

                    // Re-calculate total with new gap to center it
                    let mut recalc_width = 0.0;
                    for key_str in OSK_SPECIAL_KEYS.iter() {
                        recalc_width += measure_text(key_str, Some(font), osk_font_size, 1.0).width;
                    }
                    recalc_width += ((OSK_SPECIAL_KEYS.len() - 1) as f32) * actual_key_gap;

                    let mut current_key_x = container_x + (container_w - recalc_width) / 2.0;

                    for (c, key_str) in OSK_SPECIAL_KEYS.iter().enumerate() {
                        let text_dims = measure_text(key_str, Some(font), osk_font_size, 1.0);
                        let is_selected = (current_layout.len(), c) == wifi_state.osk_coords;
                        let is_active = (*key_str == "SHIFT" && wifi_state.osk_shift_active) || (*key_str == "SHOW" && wifi_state.show_password);

                        let mut box_color = if is_active { Color::new(0.3, 0.7, 1.0, 1.0) } else { WHITE };

                        // [!] FIX 3: Apply Cursor Styles to Special Keys
                        if is_selected {
                            box_color = cursor_color;
                            // Only draw the selection box if we are in BOX mode
                            if config.cursor_style == "BOX" {
                                let box_h = osk_font_size as f32 + 10.0;
                                let box_y = special_row_y - osk_font_size as f32 - 5.0;
                                draw_rectangle_lines(current_key_x - 5.0, box_y, text_dims.width + 10.0, box_h, line_thickness, box_color);
                            }
                        } else if is_active {
                            // Always draw box for active toggle states (SHIFT/SHOW) so user knows they are ON
                            let box_h = osk_font_size as f32 + 10.0;
                            let box_y = special_row_y - osk_font_size as f32 - 5.0;
                            draw_rectangle_lines(current_key_x - 5.0, box_y, text_dims.width + 10.0, box_h, 2.0, box_color);
                        }

                        if is_selected && config.cursor_style == "TEXT" {
                            text_with_color(font_cache, config, key_str, current_key_x, special_row_y, osk_font_size, cursor_color);
                        } else {
                            text_with_config_color(font_cache, config, key_str, current_key_x, special_row_y, osk_font_size);
                        }

                        current_key_x += text_dims.width + actual_key_gap;
                    }
                }
            }
        }
        WifiScreenState::List => {
            text_with_config_color(font_cache, config, "Available Wi-Fi Networks", text_x, container_y + 30.0 * scale_factor, font_size);
            match &wifi_state.networks {
                Ok(networks) => {
                    if networks.is_empty() {
                        text_with_config_color(font_cache, config, "No networks found.", text_x, container_y + 80.0 * scale_factor, font_size);
                    } else {
                        for (i, ap) in networks.iter().take(10).enumerate() {
                            let y_pos = container_y + 80.0 * scale_factor + (i as f32 * line_height * 1.5);

                            // [!] MODIFIED: Box vs Text Cursor Logic (optional, keeping Box for consistency with your previous styles)
                            if i == wifi_state.selected_index {
                                // Use a subtle background highlight for selection
                                draw_rectangle(container_x, y_pos - font_size as f32 - 5.0, container_w, line_height, Color::new(1.0, 1.0, 1.0, 0.2));
                            }

                            text_with_config_color(font_cache, config, &ap.ssid, text_x, y_pos, font_size);

                            let signal_text = format!("{}%", ap.signal_level);
                            let signal_dims = measure_text(&signal_text, Some(font), font_size, 1.0);
                            let signal_x = container_x + container_w - signal_dims.width - (40.0 * scale_factor);
                            text_with_config_color(font_cache, config, &signal_text, signal_x, y_pos, font_size);

                            // [!] MODIFIED: Draw Lock Icon if Secured
                            if !ap.security.is_empty() {
                                let lock_text = "🔒"; // Or use a texture if you have one
                                let lock_dims = measure_text(lock_text, Some(font), font_size, 1.0);
                                let lock_x = signal_x - lock_dims.width - (20.0 * scale_factor);
                                text_with_config_color(font_cache, config, lock_text, lock_x, y_pos, font_size);
                            }
                        }
                    }
                }
                Err(e) => {
                    text_with_config_color(font_cache, config, &format!("Error: {}", e), text_x, container_y + 80.0 * scale_factor, font_size);
                }
            }
        }
        WifiScreenState::Connected => {
            let text = "Successfully Connected!";
            let text_dims = measure_text(text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
        WifiScreenState::Error(msg) => {
            text_with_config_color(font_cache, config, "Connection Failed", text_x, container_y + 80.0 * scale_factor, font_size);

            // Simple wrapping: split into chunks or just display multiple lines if it contains newlines
            // For the "key-mgmt" error which is one long line, we can split it for display.
            let max_width = container_w - 80.0 * scale_factor; // Approximate available width

            // A quick hack to split long error messages for display
            let chars_per_line = (max_width / (font_size as f32 * 0.6)) as usize; // Approximate char width

            let mut y_offset = container_y + 80.0 * scale_factor + line_height;

            // Split the message into chunks that fit
            let mut start = 0;
            let len = msg.len();
            while start < len {
                let end = usize::min(start + chars_per_line, len);
                let slice = &msg[start..end];
                text_with_config_color(font_cache, config, slice, text_x, y_offset, font_size);
                y_offset += line_height;
                start = end;
            }
        }
        _ => {
            let text = match &wifi_state.screen_state {
                WifiScreenState::Scanning => "Scanning...",
                WifiScreenState::Connecting => "Connecting...",
                _ => ""
            };
            let text_dims = measure_text(text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, text, screen_width() / 2.0 - text_dims.width / 2.0, screen_height() / 2.0, font_size);
        }
    }
}

// --- Background Thread Functions ---

fn prepare_wifi_system(tx: Sender<WifiMessage>) {
    thread::spawn(move || {
        let output;

        if DEV_MODE {
            tx.send(WifiMessage::PreparationComplete(Ok(()))).unwrap();
            return;
        } else {
            output = Command::new("sudo")
            .arg("/usr/bin/kazeta-wifi-setup")
            .output();
        }

        let result = match output {
            Ok(out) => {
                if out.status.success() {
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    Err(format!("Setup script failed: {}", stderr.trim()))
                }
            }
            Err(e) => Err(format!("Failed to run setup script: {}", e)),
        };

        tx.send(WifiMessage::PreparationComplete(result)).unwrap();
    });
}
