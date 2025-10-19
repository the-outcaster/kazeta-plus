use bluer::{AdapterEvent, Result, Session, DiscoveryFilter};
use bluer::agent::{
    Agent, RequestAuthorization, RequestConfirmation, RequestPasskey, RequestPinCode,
};
use futures::StreamExt;
use macroquad::prelude::*;
use std::collections::HashMap;
use std::result::Result as StdResult; // Good, keep this
use std::thread;
use tokio::runtime::Runtime;
use tokio::time::{sleep, Duration};
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

// ===================================
// STRUCTS/ENUMS
// ===================================

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
    ForgetConfirm(BluetoothDevice),
}

enum BluetoothMessage {
    ScanResult(StdResult<Vec<BluetoothDevice>, String>),
    PairingSuccess(String),
    ConnectionUpdate(String),
    ForgetSuccess(String),
    Error(String),
}

pub struct BluetoothState {
    pub screen_state: BluetoothScreenState,
    pub devices: Vec<BluetoothDevice>,
    pub selected_index: usize,
    rx: TokioReceiver<BluetoothMessage>,
    tx_cmd: TokioSender<String>,
}

// ===================================
// IMPLEMENTATIONS
// ===================================

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

// ===================================
// FUNCTIONS
// ===================================

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
            BluetoothMessage::PairingSuccess(device_name) => {
                println!("[UI_UPDATE] Received PairingSuccess for {}", device_name);
                state.screen_state = BluetoothScreenState::Connecting(device_name);
            }
            BluetoothMessage::ConnectionUpdate(device_name) => {
                println!("[UI_UPDATE] Received ConnectionUpdate for {}", device_name);
                state.screen_state = BluetoothScreenState::Connected(device_name);
            }
            BluetoothMessage::ForgetSuccess(device_name) => {
                println!("[UI_UPDATE] Received ForgetSuccess for {}. List will refresh.", device_name);
                // The device list will update automatically from the agent's
                // DeviceRemoved event or the next poll.
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
                if input_state.secondary {
                    let device = state.devices[state.selected_index].clone();
                    println!("[UI_UPDATE] Forget button pressed for {}", device.name);
                    state.screen_state = BluetoothScreenState::ForgetConfirm(device);
                    sound_effects.play_select(config); // Or a different sound
                }
            }

            if input_state.back {
                println!("[UI_UPDATE] Back pressed on DeviceList - Navigating to Extras.");
                *current_screen = Screen::Extras;
                sound_effects.play_back(config);
            }
        }
        BluetoothScreenState::ForgetConfirm(device) => {
            if input_state.select { // "Yes"
                println!("[UI_UPDATE] Confirmed forget for {}", device.name);
                // Send the command to the agent
                let _ = state.tx_cmd.send(format!("forget {}", device.mac_address));
                state.screen_state = BluetoothScreenState::DeviceList; // Go back to list
                state.selected_index = 0; // Reset cursor
                sound_effects.play_select(config);
            } else if input_state.back { // "No"
                println!("[UI_UPDATE] Canceled forget for {}", device.name);
                state.screen_state = BluetoothScreenState::DeviceList;
                sound_effects.play_back(config);
            }
        }
        BluetoothScreenState::Error(_) | BluetoothScreenState::Connected(_) => {
            if input_state.select || input_state.back {
                println!("[UI_UPDATE] Back/Select pressed on Error/Connected - Navigating to DeviceList."); // Add log
                state.screen_state = BluetoothScreenState::DeviceList;
                state.selected_index = 0; // Reset cursor to the top
                sound_effects.play_select(config);
            }
        }
        // "Back" from a waiting screen should also go to the list
        BluetoothScreenState::Pairing(_) | BluetoothScreenState::Connecting(_) => {
            if input_state.back {
                println!("[UI_UPDATE] Back pressed on Pairing/Connecting - Navigating to DeviceList."); // Add log
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
        BluetoothScreenState::ForgetConfirm(device) => {
            let text = format!("Remove {}?", device.name);
            let dims = measure_text(&text, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, &text, center_x - dims.width / 2.0, center_y - line_height, font_size);

            let prompt = "Select = Yes / Back = No";
            let prompt_dims = measure_text(prompt, Some(font), font_size, 1.0);
            text_with_config_color(font_cache, config, prompt, center_x - prompt_dims.width / 2.0, center_y + line_height, font_size);
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
    tx: TokioSender<BluetoothMessage>,
    mut rx_cmd: TokioReceiver<String>,
) -> Result<()> {
    println!("[BT_AGENT] Initializing D-Bus...");
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;

    println!("[BT_AGENT] Registering auto-accept pairing agent...");

    // Agent is a struct. We create it and fill its fields with closures.
    let agent = Agent {
        // This closure is called for "Just Works" pairing or passkey confirmation.
        request_confirmation: Some(Box::new(|req: RequestConfirmation| {
            println!(
                "[BT_AGENT] Auto-accepting pairing confirmation (Passkey: {})",
                     req.passkey
            );
            // We return a Pinned Future that resolves to Ok(())
            Box::pin(async { Ok(()) })
        })),

        // This closure is called when the device requests a passkey (e.g., a mouse).
        request_passkey: Some(Box::new(|_req: RequestPasskey| {
            println!("[BT_AGENT] Auto-providing default passkey '0000'");
            Box::pin(async { Ok(0000) })
        })),

        // This closure is called when the device requests a legacy PIN code.
        request_pin_code: Some(Box::new(|_req: RequestPinCode| {
            println!("[BT_AGENT] Auto-providing default PIN '0000'");
            Box::pin(async { Ok("0000".to_string()) })
        })),

        // This closure is called to authorize the connection.
        request_authorization: Some(Box::new(|_req: RequestAuthorization| {
            println!("[BT_AGENT] Auto-authorizing connection");
            Box::pin(async { Ok(()) })
        })),

        // Use default handlers for all other events.
        ..Default::default()
    };

    // We must keep the handle alive, or the agent gets unregistered.
    let _agent_handle = session.register_agent(agent).await?;
    println!("[BT_AGENT] Agent registered.");

    adapter.set_powered(true).await?;
    println!("[BT_AGENT] D-Bus ready. Adapter: {}", adapter.name());

    // This tells the adapter to scan for all transport types (Classic and LE).
    println!("[BT_AGENT] Setting discovery filter...");
    let filter = DiscoveryFilter::default();

    // We are intentionally NOT setting filter.transport.
    // The default value (None) will cause BlueZ to use "auto" transport,
    // which scans for both Classic and LE devices. This is what we want.

    if let Err(e) = adapter.set_discovery_filter(filter).await {
        eprintln!("[BT_AGENT] Warning: Could not set discovery filter: {}. May only see known devices.", e);
        tx.send(BluetoothMessage::Error(format!("Filter failed: {}", e))).ok();
    }
    println!("[BT_AGENT] Filter set.");

    // Keep the stream active, as it works initially
    let mut discover_stream = adapter.discover_devices().await?;
    println!("[BT_AGENT] Discovery stream started. Entering main loop.");

    let mut ui_devices: HashMap<String, BluetoothDevice> = HashMap::new();
    let mut poll_timer = Box::pin(sleep(Duration::from_secs(3)));

    loop {
        tokio::select! {
            // --- Branch 1: Handle discovery events ---
            Some(evt) = discover_stream.next() => {
                let mut list_changed = false;
                match evt {
                    AdapterEvent::DeviceAdded(addr) => {
                        match adapter.device(addr) { // Sync
                            Ok(device) => {
                                if let Ok(Some(name)) = device.name().await {
                                    if !name.is_empty() && !ui_devices.contains_key(&addr.to_string()) {
                                        println!("[BT_AGENT] Discovered new device (event): {} ({})", name, addr);
                                        ui_devices.insert(addr.to_string(), BluetoothDevice { mac_address: addr.to_string(), name: name.clone() });
                                        list_changed = true;
                                    }
                                }
                            }
                            Err(e) => eprintln!("[BT_AGENT] Error getting device object {}: {}", addr, e),
                        }
                    }
                    AdapterEvent::DeviceRemoved(addr) => {
                        if ui_devices.remove(&addr.to_string()).is_some() {
                            println!("[BT_AGENT] Device removed (event): {}", addr);
                            list_changed = true;
                        }
                    }
                    AdapterEvent::PropertyChanged(_prop) => {
                        // We primarily rely on polling for updates now, but could add name checks here if needed
                    }
                }

                if list_changed {
                    println!("[BT_AGENT] Device list changed via event, updating UI ({} devices).", ui_devices.len());
                    let device_list: Vec<BluetoothDevice> = ui_devices.values().cloned().collect();
                    if tx.send(BluetoothMessage::ScanResult(Ok(device_list))).is_err() {
                        println!("[BT_AGENT] UI channel closed during event update. Exiting.");
                        break;
                    }
                }
            }

            // --- Branch 2: Handle commands ---
            Some(cmd) = rx_cmd.recv() => {
                println!("[BT_AGENT] Received command: {}", cmd);
                if cmd.starts_with("pair") {
                    let mac = cmd.split_whitespace().nth(1).unwrap_or_default();
                    println!("[BT_AGENT] Handling pair command for: {}", mac);

                    // --- Pairing Logic ---
                    println!("[BT_AGENT] Pausing discovery for pairing...");
                    drop(discover_stream); // Stop listening to events

                    sleep(Duration::from_millis(500)).await;

                    if let Some(device_info) = ui_devices.get(mac) {
                        let addr = device_info.mac_address.parse()?;
                        let device = adapter.device(addr)?; // Sync

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
                            println!("[BT_AGENT] Pairing failed.");
                        } else {
                            println!("[BT_AGENT] Pairing successful. Attempting to connect...");
                            tx.send(BluetoothMessage::PairingSuccess(device_info.name.clone())).ok();

                            if let Err(e) = device.connect().await {
                                tx.send(BluetoothMessage::Error(format!("Connection Failed: {}", e))).ok();
                                println!("[BT_AGENT] Connection failed.");
                            } else {
                                println!("[BT_AGENT] Connection successful.");
                                tx.send(BluetoothMessage::ConnectionUpdate(device_info.name.clone())).ok();
                            }
                        }
                    } else {
                        println!("[BT_AGENT] Device {} not found in known list.", mac);
                        tx.send(BluetoothMessage::Error(format!("Device not found: {}", mac))).ok();
                    }

                    // --- Resume Discovery Stream ---
                    println!("[BT_AGENT] Resuming discovery stream and scan...");
                    discover_stream = adapter.discover_devices().await?;

                    println!("[BT_AGENT] Discovery stream resumed.");
                    poll_timer = Box::pin(sleep(Duration::from_secs(0))); // Reset timer
                } else if cmd.starts_with("forget") {
                    let mac = cmd.split_whitespace().nth(1).unwrap_or_default();
                    println!("[BT_AGENT] Handling forget command for: {}", mac);

                    let addr = match mac.parse() {
                        Ok(addr) => addr,
                        Err(e) => {
                            eprintln!("[BT_AGENT] Invalid MAC address {}: {}", mac, e);
                            tx.send(BluetoothMessage::Error(format!("Invalid MAC: {}", e))).ok();
                            continue; // Wait for next command
                        }
                    };

                    let device_name = ui_devices.get(mac).map(|d| d.name.clone()).unwrap_or_else(|| mac.to_string());

                    // Pause discovery stream, just like we do for pairing
                    println!("[BT_AGENT] Pausing discovery for device removal...");
                    drop(discover_stream);
                    sleep(Duration::from_millis(250)).await;

                    match adapter.remove_device(addr).await {
                        Ok(_) => {
                            println!("[BT_AGENT] Successfully removed device {}", mac);
                            tx.send(BluetoothMessage::ForgetSuccess(device_name)).ok();
                            // The DeviceRemoved event will fire, which updates the UI.
                            // We'll also force a poll just to be safe.
                            poll_timer = Box::pin(sleep(Duration::from_secs(0)));
                        }
                        Err(e) => {
                            eprintln!("[BT_AGENT] Failed to remove device {}: {}", mac, e);
                            tx.send(BluetoothMessage::Error(format!("Failed to remove: {}", e))).ok();
                        }
                    }

                    // --- Resume Discovery Stream ---
                    println!("[BT_AGENT] Resuming discovery stream...");
                    discover_stream = adapter.discover_devices().await?;
                    println!("[BT_AGENT] Discovery stream resumed.");
                }
            },

            // --- Branch 3: Poll devices periodically ---
            _ = &mut poll_timer => {
                match adapter.device_addresses().await {
                    Ok(all_addresses) => {
                        let mut new_devices_map = HashMap::new();
                        for addr in all_addresses {
                            match adapter.device(addr) { // Sync call
                                Ok(device) => {
                                    if let Ok(Some(name)) = device.name().await {
                                        if !name.is_empty() {
                                            let addr_str = device.address().to_string();
                                            new_devices_map.insert(addr_str.clone(), BluetoothDevice { mac_address: addr_str, name });
                                        }
                                    }
                                }
                                Err(e) => eprintln!("[BT_AGENT] Error getting device object {}: {}", addr, e),
                            }
                        }

                        if new_devices_map != ui_devices {
                            println!("[BT_AGENT] Device list changed via poll ({} devices), updating UI.", new_devices_map.len());
                            ui_devices = new_devices_map;
                            let device_list: Vec<BluetoothDevice> = ui_devices.values().cloned().collect();
                            if tx.send(BluetoothMessage::ScanResult(Ok(device_list))).is_err() {
                                println!("[BT_AGENT] UI channel closed during poll update. Exiting.");
                                break;
                            }
                        }
                        // else { println!("[BT_AGENT] Polling found no changes."); } // Keep commented
                    }
                    Err(e) => {
                        eprintln!("[BT_AGENT] Error polling device addresses: {}", e);
                        tx.send(BluetoothMessage::Error(format!("Polling failed: {}", e))).ok();
                    }
                }
                // Reset the timer for the next poll
                poll_timer = Box::pin(sleep(Duration::from_secs(3)));
            },

            // --- Branch 4: Handle UI closing ---
            else => {
                println!("[BT_AGENT] UI channel closed or select! broke. Shutting down.");
                break;
            }
        }
        // println!("[BT_AGENT] End of select! loop iteration."); // Keep commented
    }
    println!("[BT_AGENT] Exiting run_bluetooth_agent.");
    Ok(())
}


fn manage_bluetooth_agent(
    tx: TokioSender<BluetoothMessage>,
    rx_cmd: TokioReceiver<String>,
) {
    thread::spawn(move || {
        println!("[BT_AGENT] Starting Bluetooth agent thread...");
        let rt = Runtime::new().expect("Failed to create Tokio runtime");

        let tx_err = tx.clone();
        if let Err(e) = rt.block_on(run_bluetooth_agent(tx, rx_cmd)) {
            eprintln!("[BT_AGENT] run_bluetooth_agent failed: {}", e);
            tx_err.send(BluetoothMessage::Error(format!("Agent failed: {}", e))).ok();
        }
        println!("[BT_AGENT] Bluetooth agent thread finished.");
    });
}
