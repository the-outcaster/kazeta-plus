//use macroquad::audio::{self, load_sound, load_sound_from_bytes, Sound, PlaySoundParams, stop_sound};
use macroquad::audio::{load_sound, load_sound_from_bytes, play_sound, stop_sound, PlaySoundParams, Sound};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::{HashSet, HashMap}; // FIX #1: Correct path for HashMap
use crate::config::{Config, get_user_data_dir};

#[derive(Clone)]
pub struct SoundEffects {
    pub cursor_move: Sound,
    pub select: Sound,
    pub reject: Sound,
    pub back: Sound,
}

impl SoundEffects {
    pub async fn load(pack_name: &str) -> Self {
        let default_move = load_sound_from_bytes(include_bytes!("../move.wav")).await.unwrap();
        let default_select = load_sound_from_bytes(include_bytes!("../select.wav")).await.unwrap();
        let default_reject = load_sound_from_bytes(include_bytes!("../reject.wav")).await.unwrap();
        let default_back = load_sound_from_bytes(include_bytes!("../back.wav")).await.unwrap();

        if pack_name == "Default" {
            return SoundEffects {
                cursor_move: default_move,
                select: default_select,
                reject: default_reject,
                back: default_back,
            };
        }

        let system_pack_path = format!("../sfx/{}", pack_name);
        let user_pack_path = get_user_data_dir().map(|d| d.join("sfx").join(pack_name));

        // FIX #2: Nested functions cannot be `pub`
        async fn load_one_sfx(
            name: &str,
            user_path_base: &Option<PathBuf>,
            system_path_base: &str,
            fallback: &Sound,
        ) -> Sound {
            if let Some(base) = user_path_base {
                if let Ok(sound) = load_sound(&base.join(name).to_string_lossy()).await {
                    return sound;
                }
            }
            if let Ok(sound) = load_sound(&std::path::Path::new(system_path_base).join(name).to_string_lossy()).await {
                return sound;
            }
            fallback.clone()
        }

        let (cursor_move, select, reject, back) = futures::join!(
            load_one_sfx("move.wav", &user_pack_path, &system_pack_path, &default_move),
            load_one_sfx("select.wav", &user_pack_path, &system_pack_path, &default_select),
            load_one_sfx("reject.wav", &user_pack_path, &system_pack_path, &default_reject),
            load_one_sfx("back.wav", &user_pack_path, &system_pack_path, &default_back)
        );

        SoundEffects { cursor_move, select, reject, back }
    }

    pub fn play_cursor_move(&self, config: &Config) {
        play_sound(&self.cursor_move, PlaySoundParams {
            looped: false,
            volume: config.sfx_volume,
        });
    }

    pub fn play_select(&self, config: &Config) {
        play_sound(&self.select, PlaySoundParams {
            looped: false,
            volume: config.sfx_volume,
        });
    }

    pub fn play_reject(&self, config: &Config) {
        play_sound(&self.reject, PlaySoundParams {
            looped: false,
            volume: config.sfx_volume,
        });
    }

    pub fn play_back(&self, config: &Config) {
        play_sound(&self.back, PlaySoundParams {
            looped: false,
            volume: config.sfx_volume,
        });
    }
}

// Scans the 'sfx/' directory for sound pack folders.
pub fn find_sound_packs() -> Vec<String> {
    let mut packs = HashSet::new();
    packs.insert("Default".to_string()); // "Default" is always an option

    // 1. Scan the system directory relative to the BIOS
    let system_sfx_dir = std::path::Path::new("../sfx");
    if let Ok(entries) = fs::read_dir(system_sfx_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                packs.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }

    // 2. Scan the user's data directory
    if let Some(user_sfx_dir) = get_user_data_dir().map(|d| d.join("sfx")) {
        if let Ok(entries) = fs::read_dir(user_sfx_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    packs.insert(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
    }

    // 3. Convert the set back to a sorted list for the UI
    let mut sorted_packs: Vec<String> = packs.into_iter().collect();
    sorted_packs.sort();
    sorted_packs
}

pub fn play_new_bgm(
    track_name: &str,
    volume: f32,
    music_cache: &HashMap<String, Sound>,
    current_bgm: &mut Option<Sound>,
) {
    if let Some(sound) = current_bgm.take() {
        stop_sound(&sound);
    }

    if track_name != "OFF" {
        if let Some(sound_to_play) = music_cache.get(track_name) {
            let sound_handle = sound_to_play.clone();
            play_sound(&sound_handle, PlaySoundParams { looped: true, volume });
            *current_bgm = Some(sound_handle);
        }
    }
}
