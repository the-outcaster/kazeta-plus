use crate::{Memory, StorageMedia, save, CopyOperationState, PlaytimeCache, SizeCache};
use std::sync::{Arc, Mutex, atomic::{AtomicU16, Ordering}};
use std::{thread, time};
use std::collections::HashMap;
use macroquad::prelude::*; // For Texture2D if needed by structs

pub async fn load_memories(media: &StorageMedia, cache: &mut HashMap<String, Texture2D>, queue: &mut Vec<(String, String)>) -> Vec<Memory> {
    let mut memories = Vec::new();

    if let Ok(details) = save::get_save_details(&media.id) {
        for (cart_id, name, icon_path) in details {
            if !cache.contains_key(&cart_id) {
                queue.push((cart_id.clone(), icon_path.clone()));
            }

            let m = Memory {
                id: cart_id,
                name: Some(name),
                drive_name: media.id.clone(),
            };
            memories.push(m);
        }
    }

    memories
}

pub async fn check_save_exists(memory: &Memory, target_media: &StorageMedia, icon_cache: &mut HashMap<String, Texture2D>, icon_queue: &mut Vec<(String, String)>) -> bool {
    let target_memories = load_memories(target_media, icon_cache, icon_queue).await;
    target_memories.iter().any(|m| m.id == memory.id)
}

pub fn copy_memory(memory: &Memory, from_media: &StorageMedia, to_media: &StorageMedia, state: Arc<Mutex<CopyOperationState>>) {
    // Initialize the copy operation state
    if let Ok(mut copy_state) = state.lock() {
        copy_state.progress = 0;
        copy_state.running = true;
        copy_state.error_message = None;
    }

    // Small delay to show the operation has started
    thread::sleep(time::Duration::from_millis(500));

    // Create progress tracking
    let progress = Arc::new(AtomicU16::new(0));
    let progress_clone = progress.clone();
    let state_clone = state.clone();

    // Spawn a thread to monitor progress from the copy operation
    let monitor_handle = thread::spawn(move || {
        loop {
            let current_progress = progress_clone.load(Ordering::SeqCst);

            // Update the UI state with the current progress
            if let Ok(mut copy_state) = state_clone.lock() {
                // Only update if the operation is still running
                if copy_state.running {
                    copy_state.progress = current_progress;
                } else {
                    // Operation completed, exit the monitoring loop
                    break;
                }
            }

            // If we've reached 100%, the copy operation should be finishing soon
            if current_progress >= 100 {
                break;
            }

            thread::sleep(time::Duration::from_millis(50));
        }
    });

    // Perform the actual copy operation
    let copy_result = save::copy_save(&memory.id, &from_media.id, &to_media.id, progress);

    // Handle the result
    match copy_result {
        Ok(_) => {
            // Ensure progress shows 100% on success
            if let Ok(mut copy_state) = state.lock() {
                copy_state.progress = 100;
            }

            // Pause for 1.5 seconds to show completion clearly while keeping the operation running
            thread::sleep(time::Duration::from_millis(1500));

            // Mark operation as complete (this will allow the monitoring thread to exit)
            if let Ok(mut copy_state) = state.lock() {
                copy_state.running = false;
                copy_state.should_clear_dialogs = true;
            }

            // Wait for the monitoring thread to finish
            monitor_handle.join().ok();
        },
        Err(e) => {
            // Handle error case (this will also stop the monitoring thread)
            if let Ok(mut copy_state) = state.lock() {
                copy_state.running = false;
                copy_state.should_clear_dialogs = true;
                copy_state.error_message = Some(format!("Failed to copy save: {}", e));
            }

            // Wait for the monitoring thread to finish
            monitor_handle.join().ok();
        }
    }
}

/// Get playtime for a specific game, using cache when available
pub fn get_game_playtime(memory: &Memory, playtime_cache: &mut PlaytimeCache) -> f32 {
    let cache_key = (memory.id.clone(), memory.drive_name.clone());

    if let Some(&cached_playtime) = playtime_cache.get(&cache_key) {
        cached_playtime
    } else {
        let calculated_playtime = save::calculate_playtime(&memory.id, &memory.drive_name);
        playtime_cache.insert(cache_key, calculated_playtime);
        calculated_playtime
    }
}

/// Get size for a specific game, using cache when available
pub fn get_game_size(memory: &Memory, size_cache: &mut SizeCache) -> f32 {
    let cache_key = (memory.id.clone(), memory.drive_name.clone());

    if let Some(&cached_size) = size_cache.get(&cache_key) {
        cached_size
    } else {
        let calculated_size = save::calculate_save_size(&memory.id, &memory.drive_name);
        size_cache.insert(cache_key, calculated_size);
        calculated_size
    }
}
