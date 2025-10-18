use bluer::{Device, Result, Session}; // Updated use
use macroquad::prelude::*;
use std::collections::HashMap;
use std::result::Result as StdResult; // Use StdResult for our enum
use std::thread;
use tokio::runtime::Runtime;
use tokio::time::{sleep, Duration}; // Use tokio's sleep
use tokio::sync::mpsc::{
    unbounded_channel as tokio_channel, UnboundedReceiver as TokioReceiver,
    UnboundedSender as TokioSender,
};

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

// -- FIX -- This enum is now fully utilized.
enum BluetoothMessage {
    ScanResult(StdResult<Vec<BluetoothDevice>, String>),
    PairingSuccess(String),
    ConnectionUpdate(String),
    Error(String),
}

pub struct BluetoothState {
    pub screen_state: BluetoothScreenState,
    pub devices: Vec<BluetoothDevice>,
    pub selected_index: usize,
    rx: TokioReceiver<BluetoothMessage>,
    tx_cmd: TokioSender<String>,
}

// --- Implementation ---

impl BluetoothState {
    pub fn new() -> Self {
        let (tx_msg, rx_msg) = tokio_channel(); // Use tokio's channel
        let (tx_cmd, rx_cmd) = tokio_channel(); // Use tokio's channel

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
        println!("[INFO] Bluetooth screen closing. Agent will shut down.");
        // We don't need to send "quit". The agent will detect
        // the channel closing when this struct is dropped.
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
                let mut sorted_devices = new_devices;
                sorted_devices.sort_by(|a, b| a.name.cmp(&b.name));
                state.devices = sorted_devices;

                println!("[UI_DEBUG] Updated device list. Count: {}", state.devices.len());
            }
            BluetoothMessage::ScanResult(Err(e)) | BluetoothMessage::Error(e) => {
                state.screen_state = BluetoothScreenState::Error(e);
            }
            // -- FIX -- This now correctly handles the message from the agent.
            BluetoothMessage::PairingSuccess(device_name) => {
                state.screen_state = BluetoothScreenState::Connecting(device_name);
            }
            BluetoothMessage::ConnectionUpdate(device_name) => {
                state.screen_state = BluetoothScreenState::Connected(device_name);
            }
        }
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

                    let _ = state.tx_cmd.send(format!("pair {}", device.mac_address));
                }
                if input_state.back {
                    *current_screen = Screen::Extras;
                    sound_effects.play_back(config);
                }
            }
        }
        BluetoothScreenState::Error(_) | BluetoothScreenState::Connected(_) => {
            if input_state.select || input_state.back {
                state.screen_state = BluetoothScreenState::DeviceList;
                state.selected_index = 0; // Reset cursor to the top
                sound_effects.play_select(config);
            }
        }
        // "Back" from a waiting screen should also go to the list
        BluetoothScreenState::Pairing(_) | BluetoothScreenState::Connecting(_) => {
            if input_state.back {
                state.screen_state = BluetoothScreenState::DeviceList;
                state.selected_index = 0;
                sound_effects.play_back(config);
                // The agent will continue its last command, but if it
                // succeeds, it will just update the UI state, which is fine.
            }
        }
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
            let start_y = 130.0 * scale_factor;
            if state.devices.is_empty() {
                let dot_count = (get_time() * 2.0) as usize % 4;
                let dots = ".".repeat(dot_count);
                let text = format!("Scanning for new devices{}", dots);
                let dims = measure_text(&text, Some(font), font_size, 1.0);
                text_with_config_color(font_cache, config, &text, center_x - dims.width / 2.0, center_y, font_size);
            } else {
                for (i, device) in state.devices.iter().enumerate() {
                    let y_pos = start_y + (i as f32 * line_height);
                    let dims = measure_text(&device.name, Some(font), font_size, 1.0);
                    let x_pos = center_x - dims.width / 2.0;

                    if i == state.selected_index {
                        let cursor_color = animation_state.get_cursor_color(config);
                        draw_rectangle_lines(x_pos - 20.0, y_pos - font_size as f32 * 1.3, dims.width + 40.0, line_height, 8.0, cursor_color);
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

async fn run_bluetooth_agent(
    tx: TokioSender<BluetoothMessage>, // This is now a tokio channel
    mut rx_cmd: TokioReceiver<String>, // This is now a tokio channel
) -> Result<()> {
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let mut discover = Some(adapter.discover_devices().await?);
    println!("[BT_AGENT] D-Bus session started. Discovery enabled.");

    let mut ui_devices: HashMap<String, BluetoothDevice> = HashMap::new();
    let mut poll_timer = Box::pin(sleep(Duration::from_secs(3)));

    loop {
        tokio::select! {
            // --- Branch 1: Handle commands from the UI ---
            // FIX: This now correctly awaits the tokio channel
            Some(cmd) = rx_cmd.recv() => {
                if cmd.starts_with("pair") {
                    let mac = cmd.split_whitespace().nth(1).unwrap_or_default();
                    println!("[BT_AGENT] Received pair command for: {}", mac);

                    println!("[BT_AGENT] Pausing discovery for pairing...");
                    if let Some(discover_stream) = discover.take() {
                        drop(discover_stream);
                    }
                    sleep(Duration::from_millis(500)).await;

                    if let Some(device_info) = ui_devices.get(mac) {
                        let addr = device_info.mac_address.parse()?;
                        let device: Device = adapter.device(addr)?;

                        if device.is_paired().await? {
                            println!("[BT_AGENT] Device already paired. Removing to re-pair...");
                            if let Err(e) = adapter.remove_device(device.address()).await {
                                println!("[BT_AGENT] Note: Could not remove device: {}", e);
                            }
                            sleep(Duration::from_millis(1000)).await;
                        } else {
                            println!("[BT_AGENT] Device not paired. Proceeding with pairing...");
                        }

                        println!("[BT_AGENT] Attempting to pair...");
                        if let Err(e) = device.pair().await {
                            tx.send(BluetoothMessage::Error(format!("Pairing Failed: {}", e))).ok();
                        } else {
                            println!("[BT_AGENT] Pairing successful. Attempting to connect...");
                            tx.send(BluetoothMessage::PairingSuccess(device_info.name.clone())).ok();

                            if let Err(e) = device.connect().await {
                                tx.send(BluetoothMessage::Error(format!("Connection Failed: {}", e))).ok();
                            } else {
                                println!("[BT_AGENT] Connection successful.");
                                tx.send(BluetoothMessage::ConnectionUpdate(device_info.name.clone())).ok();
                            }
                        }
                    } else {
                        tx.send(BluetoothMessage::Error(format!("Device not found: {}", mac))).ok();
                    }

                    discover = Some(adapter.discover_devices().await?);
                    println!("[BT_AGENT] Discovery resumed.");

                    poll_timer = Box::pin(sleep(Duration::from_secs(0))); // Reset timer
                }
            },

            // --- Branch 2: Periodically scan for devices ---
            _ = &mut poll_timer => {
                let mut new_devices_map = HashMap::new();
                let all_addresses = adapter.device_addresses().await?;
                for addr in all_addresses {
                    let device = adapter.device(addr)?;
                    if let Ok(Some(name)) = device.name().await {
                        if !name.is_empty() {
                            let addr_str = device.address().to_string();
                            new_devices_map.insert(addr_str.clone(), BluetoothDevice { mac_address: addr_str, name });
                        }
                    }
                }

                if new_devices_map != ui_devices {
                    println!("[BT_AGENT] Found {} named devices, updating UI.", new_devices_map.len());
                    ui_devices = new_devices_map;
                    let device_list: Vec<BluetoothDevice> = ui_devices.values().cloned().collect();
                    if tx.send(BluetoothMessage::ScanResult(Ok(device_list))).is_err() {
                        break;
                    }
                }

                poll_timer = Box::pin(sleep(Duration::from_secs(10)));
            },

            // --- Branch 3: Handle UI shutting down ---
            else => {
                // This happens if the tx_cmd channel closes
                // (because BluetoothState was dropped)
                println!("[BT_AGENT] UI channel closed. Shutting down.");
                break;
            }
        }
    }
    Ok(())
}

// This wrapper function just spawns the thread and runs the async agent
fn manage_bluetooth_agent(
    tx: TokioSender<BluetoothMessage>,
    rx_cmd: TokioReceiver<String>,
) {
    thread::spawn(move || {
        println!("[BT_AGENT] Starting Bluetooth agent thread...");
        let rt = Runtime::new().expect("Failed to create Tokio runtime");

        if let Err(e) = rt.block_on(run_bluetooth_agent(tx.clone(), rx_cmd)) {
            tx.send(BluetoothMessage::Error(format!("Agent failed: {}", e))).ok();
        }
        println!("[BT_AGENT] Bluetooth agent thread finished.");
    });
}
