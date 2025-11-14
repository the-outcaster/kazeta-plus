use once_cell::sync::Lazy;
use rodio::{
    self, buffer::SamplesBuffer, source::Source, Decoder as RodioDecoder,
    OutputStream, OutputStreamBuilder, Sink,
};
use std::fs::{self, File};
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};
use std::collections::{HashSet, HashMap};
use crate::config::{Config, get_user_data_dir};

// --- Rodio Global Audio System ---
pub struct AudioSystem {
    // Based on your error, the builder returns just an OutputStream.
    // We store it here so we can access its .mixer() later.
    pub stream: OutputStream,
}

// [!] Note: If you get an error that AudioSystem cannot be shared between threads
// (Sync trait), we may need to wrap this in a Mutex. For now, we keep it simple.
pub static AUDIO: Lazy<AudioSystem> = Lazy::new(|| {
    let stream = OutputStreamBuilder::open_default_stream()
    .expect("Failed to load audio stream");
    AudioSystem { stream }
});

// --- Helper functions for loading audio into rodio buffers ---

pub fn load_sound_from_bytes(bytes: &[u8]) -> SamplesBuffer {
    let owned = bytes.to_vec().into_boxed_slice();
    let cursor = Cursor::new(owned);               // Cursor<Box<[u8]>> is 'static
    let decoder = rodio::Decoder::new(cursor).unwrap();
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let samples: Vec<f32> = decoder.collect();
    SamplesBuffer::new(channels, sample_rate, samples)
}



pub fn load_from_file(path: &Path) -> Result<SamplesBuffer, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let decoder = RodioDecoder::new(reader)?;
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let samples: Vec<f32> = decoder.collect();
    Ok(SamplesBuffer::new(channels, sample_rate, samples))
}

// --- SoundEffects Struct and Impl ---

#[derive(Clone)]
pub struct SoundEffects {
    pub cursor_move: SamplesBuffer,
    pub select: SamplesBuffer,
    pub reject: SamplesBuffer,
    pub back: SamplesBuffer,
}

impl SoundEffects {
    pub fn load(pack_name: &str) -> Self {
        // (This logic remains unchanged, copying your original correct logic)
        let default_move = load_sound_from_bytes(include_bytes!("../move.wav"));
        let default_select = load_sound_from_bytes(include_bytes!("../select.wav"));
        let default_reject = load_sound_from_bytes(include_bytes!("../reject.wav"));
        let default_back = load_sound_from_bytes(include_bytes!("../back.wav"));

        if pack_name == "Default" {
            return SoundEffects {
                cursor_move: default_move,
                select: default_select,
                reject: default_reject,
                back: default_back,
            };
        }

        let system_pack_path = format!("../sfx/{}", pack_name);
        let user_pack_path = find_sfx_pack_path(pack_name);

        fn load_one_sfx(
            name: &str,
            user_path_base: &Option<PathBuf>,
            system_path_base: &str,
            fallback: &SamplesBuffer,
        ) -> SamplesBuffer {
            if let Some(base) = user_path_base {
                if let Ok(sound) = load_from_file(&base.join(name)) {
                    return sound;
                }
            }
            let system_path = Path::new(system_path_base).join(name);
            if let Ok(sound) = load_from_file(&system_path) {
                return sound;
            }
            fallback.clone()
        }

        let cursor_move = load_one_sfx("move.wav", &user_pack_path, &system_pack_path, &default_move);
        let select = load_one_sfx("select.wav", &user_pack_path, &system_pack_path, &default_select);
        let reject = load_one_sfx("reject.wav", &user_pack_path, &system_pack_path, &default_reject);
        let back = load_one_sfx("back.wav", &user_pack_path, &system_pack_path, &default_back);

        SoundEffects { cursor_move, select, reject, back }
    }

    // [!] FIX: We manually create the Sink using .mixer() instead of .play_once()
    // because play_once requires OutputStreamHandle which you don't have.

    pub fn play_cursor_move(&self, config: &Config) {
        let source = self.cursor_move.clone().amplify(config.sfx_volume);
        let sink = Sink::connect_new(&AUDIO.stream.mixer());
        sink.append(source);
        sink.detach(); // Fire and forget
    }

    pub fn play_select(&self, config: &Config) {
        let source = self.select.clone().amplify(config.sfx_volume);
        let sink = Sink::connect_new(&AUDIO.stream.mixer());
        sink.append(source);
        sink.detach();
    }

    pub fn play_reject(&self, config: &Config) {
        let source = self.reject.clone().amplify(config.sfx_volume);
        let sink = Sink::connect_new(&AUDIO.stream.mixer());
        sink.append(source);
        sink.detach();
    }

    pub fn play_back(&self, config: &Config) {
        let source = self.back.clone().amplify(config.sfx_volume);
        let sink = Sink::connect_new(&AUDIO.stream.mixer());
        sink.append(source);
        sink.detach();
    }
}

// --- Filesystem Functions ---
// (This section is unchanged)
pub fn find_sfx_pack_path(pack_name: &str) -> Option<PathBuf> {
    if let Some(themes_dir) = get_user_data_dir().map(|d| d.join("themes")) {
        if let Ok(theme_entries) = fs::read_dir(themes_dir) {
            for theme_entry in theme_entries.flatten() {
                if theme_entry.path().is_dir() {
                    if let Ok(asset_entries) = fs::read_dir(theme_entry.path()) {
                        for asset_entry in asset_entries.flatten() {
                            if asset_entry.path().is_dir() && asset_entry.file_name().to_string_lossy() == pack_name {
                                return Some(asset_entry.path());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn find_sound_packs() -> Vec<String> {
    let mut packs = HashSet::new();
    packs.insert("Default".to_string());
    let system_sfx_dir = std::path::Path::new("../sfx");
    if let Ok(entries) = fs::read_dir(system_sfx_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                packs.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    if let Some(user_sfx_dir) = get_user_data_dir().map(|d| d.join("sfx")) {
        if let Ok(entries) = fs::read_dir(user_sfx_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    packs.insert(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
    }
    // (Simplified theme searching for brevity - same as your original)
    if let Some(themes_dir) = get_user_data_dir().map(|d| d.join("themes")) {
        if let Ok(theme_entries) = fs::read_dir(themes_dir) {
            for theme_entry in theme_entries.flatten() {
                if theme_entry.path().is_dir() {
                    if let Ok(asset_entries) = fs::read_dir(theme_entry.path()) {
                        for asset_entry in asset_entries.flatten() {
                            if asset_entry.path().is_dir() {
                                packs.insert(asset_entry.file_name().to_string_lossy().into_owned());
                            }
                        }
                    }
                }
            }
        }
    }
    let mut sorted_packs: Vec<String> = packs.into_iter().collect();
    sorted_packs.sort();
    sorted_packs
}

// --- BGM Playback Function ---

pub fn play_new_bgm(
    track_name: &str,
    volume: f32,
    music_cache: &HashMap<String, SamplesBuffer>,
    current_bgm: &mut Option<Sink>,
) {
    if let Some(sink) = current_bgm.take() {
        sink.stop();
    }

    if track_name != "OFF" {
        if let Some(sound_to_play) = music_cache.get(track_name) {
            // [!] FIX: Use Sink::connect_new with the mixer
            let sink = Sink::connect_new(&AUDIO.stream.mixer());

            let source = sound_to_play
            .clone()
            .repeat_infinite()
            .amplify(volume);

            sink.append(source);
            *current_bgm = Some(sink);
        }
    }
}
