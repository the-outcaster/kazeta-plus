use crate::GccMessage;

use std::fs;
use std::thread;
use std::time::Duration;
use std::sync::mpsc::Sender;

// The path where the overclocked driver exposes its poll rate
const GCC_POLL_RATE_PATH: &str = "/sys/module/gcadapter_oc/parameters/rate";

pub fn start_gcc_adapter_polling(tx: Sender<GccMessage>) {
    thread::spawn(move || {
        let mut was_connected = false;
        loop {
            match fs::read_to_string(GCC_POLL_RATE_PATH) {
                Ok(rate_str) => {
                    // The file contains the interval in milliseconds (e.g., "1")
                    if let Ok(rate_ms) = rate_str.trim().parse::<u32>() {
                        if rate_ms > 0 {
                            let poll_rate_hz = 1000 / rate_ms;
                            tx.send(GccMessage::RateUpdate(poll_rate_hz)).unwrap_or_default();
                            was_connected = true;
                        }
                    }
                }
                Err(_) => {
                    // File doesn't exist, so adapter is not connected or module not loaded
                    if was_connected {
                        tx.send(GccMessage::Disconnected).unwrap_or_default();
                        was_connected = false;
                    }
                }
            }
            // Check every 2 seconds
            thread::sleep(Duration::from_secs(2));
        }
    });
}
