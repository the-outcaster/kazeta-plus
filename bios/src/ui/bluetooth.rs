use macroquad::prelude::*;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::{
    audio::SoundEffects,
    config::Config,
    types::{AnimationState, BackgroundState, BatteryInfo, Screen},
    render_background, render_ui_overlay, get_current_font, measure_text, text_with_config_color,
    FONT_SIZE, InputState,
};

// --- Structs and Enums ---

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BluetoothDevice {
    pub mac_address: String,
    pub name: String,
}

pub enum BluetoothScreenState {
    DeviceList,
    Pairing(String),
    Connecting(String),
    Connected(String),
    Error(String),
}

enum BluetoothMessage {
    ScanResult(Result<Vec<BluetoothDevice>, String>),
    ConnectionUpdate(String),
    Error(String),
}

pub struct BluetoothState {
    pub screen_state: BluetoothScreenState,
    pub devices: Vec<BluetoothDevice>,
    pub selected_index: usize,
    rx: Receiver<BluetoothMessage>,
    tx_cmd: Sender<String>,
}

// --- Implementation ---

impl BluetoothState {
    pub fn new() -> Self {
        let (tx_msg, rx_msg) = channel();
        let (tx_cmd, rx_cmd) = channel();

        manage_bluetooth_agent(tx_msg, rx_cmd);

        Self {
            screen_state: BluetoothScreenState::DeviceList,
            devices: Vec::new(),
            selected_index: 0,
            rx: rx_msg,
            tx_cmd,
        }
    }
}

impl Drop for BluetoothState {
    fn drop(&mut self) {
        println!("[INFO] Bluetooth screen closing. Sending quit command to agent.");
        let _ = self.tx_cmd.send("quit".to_string());
    }
}


pub fn update(
    state: &mut BluetoothState,
    input_state: &InputState,
    current_screen: &mut Screen,
    sound_effects: &SoundEffects,
    config: &Config,
) {
    if let Ok(msg) = state.rx.try_recv() {
        match msg {
            BluetoothMessage::ScanResult(Ok(new_devices)) => {
                let mut device_added = false;
                for new_dev in new_devices {
                    if !state.devices.iter().any(|d| d.mac_address == new_dev.mac_address) {
                        state.devices.push(new_dev);
                        device_added = true;
                    }
                }
                if device_added {
                    state.devices.sort_by(|a, b| a.name.cmp(&b.name));
                }
            }
            BluetoothMessage::ScanResult(Err(e)) | BluetoothMessage::Error(e) => {
                state.screen_state = BluetoothScreenState::Error(e);
            }
            BluetoothMessage::ConnectionUpdate(device_name) => {
                state.screen_state = BluetoothScreenState::Connected(device_name);
            }
        }
    }

    if input_state.back {
        *current_screen = Screen::Extras;
        sound_effects.play_back(config);
    }

    match &mut state.screen_state {
        BluetoothScreenState::DeviceList => {
            if !state.devices.is_empty() {
                if input_state.down && state.selected_index < state.devices.len() - 1 {
                    state.selected_index += 1;
                    sound_effects.play_cursor_move(config);
                }
                if input_state.up && state.selected_index > 0 {
                    state.selected_index -= 1;
                    sound_effects.play_cursor_move(config);
                }
                if input_state.select {
                    let device = state.devices[state.selected_index].clone();
                    state.screen_state = BluetoothScreenState::Pairing(device.name.clone());
                    sound_effects.play_select(config);

                    let pair_cmd = format!("pair {}", device.mac_address);
                    let connect_cmd = format!("connect {}", device.mac_address);
                    let tx_clone = state.tx_cmd.clone();

                    let _ = state.tx_cmd.send(pair_cmd);

                    thread::spawn(move || {
                        thread::sleep(Duration::from_secs(5));
                        let _ = tx_clone.send(connect_cmd);
                    });
                }
            }
        }
        BluetoothScreenState::Error(_) | BluetoothScreenState::Connected(_) => {
            if input_state.select || input_state.back {
                *current_screen = Screen::Extras;
                sound_effects.play_select(config);
            }
        }
        BluetoothScreenState::Pairing(name) => {
            state.screen_state = BluetoothScreenState::Connecting(name.clone());
        }
        _ => {}
    }
}

pub fn draw(
    state: &BluetoothState,
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

    let font = get_current_font(font_cache, config);
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let line_height = font_size as f32 * 1.8;

    let center_x = screen_width() / 2.0;
    let center_y = screen_height() / 2.0;

    match &state.screen_state {
        BluetoothScreenState::DeviceList => {
            let start_y = 100.0 * scale_factor;
            if state.devices.is_empty() {
                let text = "Scanning... Ensure your device is in pairing mode.";
                let dims = measure_text(text, Some(font), font_size, 1.0);
                text_with_config_color(font_cache, config, text, center_x - dims.width / 2.0, center_y, font_size);
            } else {
                for (i, device) in state.devices.iter().enumerate() {
                    let y_pos = start_y + (i as f32 * line_height);
                    let dims = measure_text(&device.name, Some(font), font_size, 1.0);
                    let x_pos = center_x - dims.width / 2.0;

                    if i == state.selected_index {
                        let cursor_color = animation_state.get_cursor_color(config);
                        draw_rectangle_lines(x_pos - 20.0, y_pos - font_size as f32, dims.width + 40.0, line_height, 3.0, cursor_color);
                    }
                    text_with_config_color(font_cache, config, &device.name, x_pos, y_pos, font_size);
                }
            }
        }
        BluetoothScreenState::Pairing(name) => {
            let text = format!("Pairing with {}...", name);
            let dims = measure_text(&text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, &text, center_x - dims.width / 2.0, center_y, font_size);
        }
        BluetoothScreenState::Connecting(name) => {
            let text = format!("Connecting to {}...", name);
            let dims = measure_text(&text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, &text, center_x - dims.width / 2.0, center_y, font_size);
        }
        BluetoothScreenState::Connected(name) => {
            let text = format!("Successfully connected to {}!", name);
            let dims = measure_text(&text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, &text, center_x - dims.width / 2.0, center_y, font_size);
        }
        BluetoothScreenState::Error(msg) => {
            let text = format!("Error: {}", msg);
            let dims = measure_text(&text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, &text, center_x - dims.width / 2.0, center_y, font_size);
        }
    }
}


// --- Background Thread Function ---

fn manage_bluetooth_agent(tx: Sender<BluetoothMessage>, rx_cmd: Receiver<String>) {
    thread::spawn(move || {
        let mut child = match Command::new("sudo")
        .args(&["bluetoothctl"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        {
            Ok(child) => child,
                  Err(e) => {
                      let _ = tx.send(BluetoothMessage::Error(format!("Failed to start bluetoothctl: {}", e)));
                      return;
                  }
        };

        let stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");

        thread::spawn(move || {
            let mut writer = stdin;
            // -- FIX -- Add small delays between commands to prevent race conditions.
            thread::sleep(Duration::from_millis(200));
            let _ = writer.write_all(b"power on\n");
            thread::sleep(Duration::from_millis(200));
            let _ = writer.write_all(b"scan on\n");

            while let Ok(cmd) = rx_cmd.recv() {
                if cmd == "quit" {
                    let _ = writer.write_all(b"scan off\n");
                    let _ = writer.write_all(b"exit\n");
                    break;
                } else {
                    let full_cmd = format!("{}\n", cmd);
                    let _ = writer.write_all(full_cmd.as_bytes());
                }
            }
        });

        let reader = BufReader::new(stdout);
        let mut devices = HashMap::new();
        let mut last_scan_send = Instant::now();

        for line in reader.lines() {
            if let Ok(line) = line {
                let trimmed_line = line.trim();
                println!("[BT_DEBUG] {}", trimmed_line);

                if trimmed_line.starts_with("[NEW] Device") {
                    let parts: Vec<&str> = trimmed_line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let mac = parts[2].to_string();
                        let name = parts[3..].join(" ");
                        if !name.is_empty() && name != mac {
                            devices.insert(mac.clone(), BluetoothDevice { mac_address: mac, name });
                        }
                    }
                }

                if line.contains("Connection successful") {
                    if let Some(mac) = line.split_whitespace().nth(2) {
                        if let Some(device) = devices.get(mac) {
                            if tx.send(BluetoothMessage::ConnectionUpdate(device.name.clone())).is_err() { break; }
                        }
                    }
                }

                if last_scan_send.elapsed() > Duration::from_secs(2) {
                    let device_list: Vec<BluetoothDevice> = devices.values().cloned().collect();
                    // Send even if empty, so the UI can update to "No devices found" if they disappear.
                    if tx.send(BluetoothMessage::ScanResult(Ok(device_list))).is_err() {
                        break;
                    }
                    last_scan_send = Instant::now();
                }
            } else {
                break;
            }
        }

        let _ = child.kill();
        println!("[BT_DEBUG] Bluetooth agent thread finished.");
    });
}

