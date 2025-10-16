use crate::config::Config;
use chrono::{FixedOffset, Utc};
use std::fs;
use std::process::Command;

use crate::Regex;
use crate::{SystemInfo, AudioSink, BatteryInfo, read_line_from_file};

// BRIGHTNESS CONTROL
// Gets the current brightness as a value between 0.0 and 1.0
pub fn get_current_brightness() -> Option<f32> {
    let Ok(max_out) = Command::new("brightnessctl").arg("max").output() else { return None };
    let Ok(get_out) = Command::new("brightnessctl").arg("get").output() else { return None };

    let max_str = String::from_utf8_lossy(&max_out.stdout);
    let get_str = String::from_utf8_lossy(&get_out.stdout);

    let max_val = max_str.trim().parse::<f32>().ok()?;
    let get_val = get_str.trim().parse::<f32>().ok()?;

    if max_val > 0.0 {
        Some(get_val / max_val)
    } else {
        None
    }
}

// Sets the brightness, taking a value between 0.0 and 1.0
pub fn set_brightness(level: f32) {
    // Clamp the value between 0.0 and 1.0
    let clamped_level = level.clamp(0.0, 1.0);
    // brightnessctl can take a percentage directly
    let percent_str = format!("{:.0}%", clamped_level * 100.0);

    // This command usually doesn't need sudo if the user is in the 'video' group
    let _ = Command::new("brightnessctl").arg("set").arg(percent_str).status();
}

// get system info
pub fn get_system_info() -> SystemInfo {
    // --- OS Name ---
    let os_name = read_line_from_file("/etc/os-release", "PRETTY_NAME=")
    .map(|name| name.replace("\"", "")) // Remove quotes
    .unwrap_or_else(|| "Kazeta+ OS".to_string());

    // --- Kernel Version ---
    let kernel = Command::new("uname").arg("-r").output()
    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    .unwrap_or_else(|_| "N/A".to_string());

    // --- CPU Model ---
    let cpu = read_line_from_file("/proc/cpuinfo", "model name")
    .map(|name| name.replace(": ", ""))
    .unwrap_or_else(|| "N/A".to_string());

    // --- GPU Model ---
    let gpu = Command::new("sh").arg("-c").arg("lspci | grep -i 'vga\\|display'")
    .output()
    .ok() // Convert Result to Option, so we can chain .and_then()
    .and_then(|o| String::from_utf8_lossy(&o.stdout)
    .lines()
    .next() // Get the first line if any
    .and_then(|line| line.split(": ").nth(2)) // Try to split and get the 3rd part
    .map(|s| s.trim().to_string())) // Trim and convert to String
    .unwrap_or_else(|| "N/A".to_string()); // If any step failed, default to "N/A"

    // --- Total RAM ---
    let ram_total = read_line_from_file("/proc/meminfo", "MemTotal:")
    .and_then(|val| val.replace("kB", "").trim().parse::<f32>().ok())
    .map(|kb| format!("{:.1} GB", kb / 1024.0 / 1024.0)) // Convert from KB to GB
    .unwrap_or_else(|| "N/A".to_string());

    SystemInfo { os_name, kernel, cpu, gpu, ram_total }
}

pub fn get_available_sinks() -> Vec<AudioSink> {
    println!("[Debug] Running get_available_sinks...");
    let mut sinks = Vec::new();

    let Ok(output) = Command::new("wpctl").arg("status").output() else {
        println!("[Debug] Failed to run 'wpctl status' command.");
        return sinks;
    };
    println!("[Debug] 'wpctl status' command finished successfully.");

    let output_str = String::from_utf8_lossy(&output.stdout);

    let re = Regex::new(r"([*]?)\s*(\d+)\.\s+(.+?)\s+\[vol:").unwrap();
    let mut in_sinks_section = false;

    for line in output_str.lines() {
        if line.contains("Sinks:") {
            in_sinks_section = true;
            continue;
        }

        if in_sinks_section {
            // --- THIS IS THE FIX ---
            // If we hit the header of the next section, we're done with sinks.
            if line.contains("Sources:") || line.contains("Filters:") {
                break;
            }

            if let Some(caps) = re.captures(line) {
                if let (Some(id_str), Some(name_str)) = (caps.get(2), caps.get(3)) {
                    if let Ok(id) = id_str.as_str().parse::<u32>() {
                        let cleaned_name = name_str.as_str()
                        .replace("Analog Stereo", "")
                        .replace("Digital Stereo (HDMI 2)", "HDMI")
                        .trim()
                        .to_string();

                        sinks.push(AudioSink {
                            id,
                            name: cleaned_name,
                        });
                    }
                }
            }
        }
    }

    println!("[Debug] Found sinks: {:#?}", sinks);
    sinks
}

/// Gets the current time and formats it using the UTC offset from the config.
pub fn get_current_local_time_string(config: &Config) -> String {
    // 1. Parse the offset string from the config (e.g., "UTC-4")
    let offset_str = config.timezone.replace("UTC", "");
    let offset_hours: i32 = if offset_str.is_empty() {
        0
    } else {
        offset_str.parse().unwrap_or(0)
    };

    // 2. Create a FixedOffset in seconds (1 hour = 3600 seconds)
    let fixed_offset = FixedOffset::east_opt(offset_hours * 3600).unwrap_or(FixedOffset::east_opt(0).unwrap());

    // 3. Get the current time in UTC
    let utc_now = Utc::now();

    // 4. Convert the UTC time to the desired offset
    let local_now = utc_now.with_timezone(&fixed_offset);

    // 5. Format for display (e.g., "05:08 PM")
    local_now.format("%-I:%M %p").to_string()
}

/// Gets the current system volume using wpctl.
pub fn get_system_volume() -> Option<f32> {
    let output = Command::new("wpctl").arg("get-volume").arg("@DEFAULT_AUDIO_SINK@").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    // The output is "Volume: 0.50", so we split by ": " and parse the second part.
    output_str.split(": ").nth(1)?.trim().parse::<f32>().ok()
}

/// Adjusts the system volume up or down.
pub fn adjust_system_volume(adjustment: &str) {
    // We use "-l 1.0" to limit the volume to 100% and prevent distortion.
    let _ = Command::new("wpctl")
    .arg("set-volume")
    .arg("-l")
    .arg("1.0")
    .arg("@DEFAULT_AUDIO_SINK@")
    .arg(adjustment)
    .status(); // .status() runs the command and waits for it to finish
}

/// Scans for a battery device and gets its capacity and status.
pub fn get_battery_info() -> Option<BatteryInfo> {
    const POWER_SUPPLY_PATH: &str = "/sys/class/power_supply";

    let entries = fs::read_dir(POWER_SUPPLY_PATH).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue; }

        let type_path = path.join("type");
        if let Ok(device_type) = fs::read_to_string(type_path) {
            if device_type.trim() == "Battery" {
                // This is a battery. Let's get both capacity and status.
                let capacity_path = path.join("capacity");
                let status_path = path.join("status");

                if let (Ok(percentage), Ok(status)) =
                    (fs::read_to_string(capacity_path), fs::read_to_string(status_path))
                    {
                        return Some(BatteryInfo {
                            percentage: percentage.trim().to_string(),
                                    status: status.trim().to_string(),
                        });
                    }
            }
        }
    }
    None
}

/// Gets the current IP address of the device.
pub fn get_ip_address() -> String {
    let output = Command::new("ip")
    .arg("-4")
    .arg("addr")
    .arg("show")
    .arg("scope")
    .arg("global")
    .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Look for a line with "inet", then parse it.
            for line in stdout.lines() {
                if line.trim().starts_with("inet") {
                    let parts: Vec<&str> = line.trim().split_whitespace().collect();
                    if let Some(ip_with_cidr) = parts.get(1) {
                        // The IP is usually followed by a CIDR mask, like "192.168.1.5/24".
                        // We split by "/" and take the first part.
                        if let Some(ip) = ip_with_cidr.split('/').next() {
                            return ip.to_string();
                        }
                    }
                }
            }
            // If no global IP was found after checking all lines
            "N/A".to_string()
        }
        Err(_) => "N/A".to_string(),
    }
}
