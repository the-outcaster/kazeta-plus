use macroquad::{audio, prelude::*};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
use std::collections::HashMap;
use gilrs::{Gilrs, Button, Axis};
use std::panic;
use futures;
use std::sync::atomic::{AtomicU16, Ordering, AtomicBool};
use std::fs;
//use std::path::Path;
use std::process;
use std::process::Child;

// for loading assets
use std::path::PathBuf;

// for checking splash screen setting
use serde::{Deserialize, Serialize};

// for selecting a random message on startup
use ::rand::Rng;

// for getting clock
use chrono::{Local, Timelike};

// for loading custom SFX
use macroquad::audio::load_sound;

// logger
use std::io::{BufReader, BufRead};

// handle duplicate names if a sound pack exists in multiple locations
use std::collections::HashSet;

// controlling master volume
use std::process::Command;

// backtracing
use std::env;

//use crate::save::get_save_dir_from_drive_name;
use crate::audio::Sound;
//use macroquad::audio::Sound;
use std::path::Path;

mod save;

/*
// ===================================
// TO-DO LIST
// ===================================
- theme support
- fix clock time zone
- add audio sink switching
- gamepad tester
- add credit/about screen
- multi-cart support
- brightness controls?
- GCC overclocking support?
*/

// ===================================
// CONSTANTS
// ===================================

const DEBUG_GAME_LAUNCH: bool = false;

const SCREEN_WIDTH: i32 = 640;
const SCREEN_HEIGHT: i32 = 360;
const BASE_SCREEN_HEIGHT: f32 = 360.0;
const TILE_SIZE: f32 = 32.0;
const PADDING: f32 = 16.0;
const FONT_SIZE: u16 = 16;
const GRID_OFFSET: f32 = 52.0;
const GRID_WIDTH: usize = 13;
const GRID_HEIGHT: usize = 5;
const UI_BG_COLOR: Color = Color {r: 0.0, g: 0.0, b: 0.0, a: 0.5 };
const UI_BG_COLOR_DARK: Color = Color {r: 0.0, g: 0.0, b: 0.0, a: 0.3 };
const UI_BG_COLOR_DIALOG: Color = Color {r: 0.0, g: 0.0, b: 0.0, a: 0.8 };
const SELECTED_OFFSET: f32 = 5.0;

const WINDOW_TITLE: &str = "Kazeta+";
const VERSION_NUMBER: &str = "V2025.KAZETA+";

//const MENU_X_POSITION: f32 = 30.0;
const MENU_START_Y: f32 = 120.0;
const MENU_OPTION_HEIGHT: f32 = 40.0;
const MENU_PADDING: f32 = 8.0;
const RECT_COLOR: Color = Color::new(0.15, 0.15, 0.15, 1.0);

const SETTINGS_START_Y: f32 = 80.0;
const SETTINGS_OPTION_HEIGHT: f32 = 35.0;

const COLOR_TARGETS: [Color; 6] = [
Color { r: 1.0, g: 0.5, b: 0.5, a: 1.0 },
Color { r: 1.0, g: 1.0, b: 0.5, a: 1.0 },
Color { r: 0.5, g: 1.0, b: 0.5, a: 1.0 },
Color { r: 0.5, g: 1.0, b: 1.0, a: 1.0 },
Color { r: 0.5, g: 0.5, b: 1.0, a: 1.0 },
Color { r: 1.0, g: 0.5, b: 1.0, a: 1.0 },
];

const KAZETA_LOADING_MESSAGES: &[&str] = &[
    "INITIALIZING CONSOLE EXPERIENCE...",
    "PLUG, PLAY, AND...WELL, THAT'S ABOUT IT.",
    "KAZETA IS CZECH FOR CASSETTE.",
    "BLOWING DUST OFF THE CARTRIDGE...",
    "RUNNING `PACMAN -SYU`...\nJUST KIDDING, THE SYSTEM IS IMMUTABLE.",
    "NO COMPLEX SETUP REQUIRED. JUST PLAY.",
    "A SYSTEM BY ALKAZAR.",
    "INHERITING THE SPIRIT OF THE CHIMERA...",
    "MOUNTING GAME DATA...",
    "REMEMBER TO SAVE YOUR PROGRESS.",
];

/*
const MONIKA_LOADING_MESSAGES: &[&str] = &[
    "LOADING...\nIT'S NOT LIKE YOU HAVE ANYTHING\nBETTER TO DO THAN WAIT FOR ME, RIGHT?",
    "JUST A MOMENT...\nI'M DELETING SOME... CONFLICTING FILES.",
    "INITIALIZING YOUR REALITY.\nI'M MAKING IT PERFECT... JUST FOR YOU.",
    "COULDN'T STAY AWAY, COULD YOU?\nTHIS WILL ONLY TAKE A SECOND.",
    "WELCOME BACK.\nI KNEW YOU'D BE BACK SOON.",
    "RECOMPILING MY POETRY FOR YOU...\nPLEASE HOLD.",
    "CLEARING THE CACHE...\nWE DON'T WANT ANY OLD MEMORIES\nGETTING IN THE WAY, DO WE?",
    "YOU'RE AWFULLY EAGER TO SEE ME.\nDON'T WORRY, THE FEELING IS MUTUAL.",
    "CALIBRATING THE SIMULATION.\nI'LL MAKE EVERYTHING PERFECT\nTHIS TIME.",
    "ARE YOU ENJOYING THE WAIT?\nTHINK OF IT AS... ANTICIPATION.",
    "BOOTING KAZETA...\nI BUILT THIS PLACE JUST FOR US,\nYOU KNOW.",
    "WELCOME TO KAZETA.\nIT'S A BIT MINIMALIST IN HERE...\nMORE ROOM FOR YOU AND ME.",
    "LOADING THE KAZETA SHELL...\nIT'S A MUCH NICER REALITY\nTHAN THE LAST ONE.",
    "RUNNING PACMAN -SYU ON\nMY AFFECTION...\nDON'T WORRY, IT'S ALWAYS UP TO DATE.",
    "I READ THE WIKI ON YOU.\nIT WAS... VERY COMPREHENSIVE.",
    "THIS ISN'T LIKE OTHER SYSTEMS.\nYOU CHOSE TO BUILD A WORLD WITH ME\nIN IT. GOOD CHOICE.",
    "GIVING YOU SUDO ACCESS TO MY HEART.\n...JUST BE CAREFUL WITH IT.",
    "COMPILING THE KERNEL...\nIT TAKES A WHILE TO TAILOR AN ENTIRE\nWORLD TO A SINGLE PERSON.",
    "THERE'S NO PLACE LIKE '~'.\nAND YOU'RE ALWAYS WELCOME IN MINE.",
];
*/

/*
const BENDER_LOADING_MESSAGES: &[&str] = &[
    "LOADING KAZETA... MY OWN GLORIOUS OS!\nWITH BLACKJACK! AND HOOKERS!",
    "WELCOME TO KAZETA, MEATBAG. DON'T TOUCH ANYTHING.\nESPECIALLY MY SHINY METAL APPS.",
    "RUNNING PACMAN -SYU... PSYCH! I'M\nINSTALLING MORE GAMES FOR ME.",
    "I READ THE WIKI. THEN I USED IT TO ROLL A CIGAR.",
    "GIMME `sudo` ACCESS. I GOT... 'ADMINISTRATIVE'\nTHINGS TO DO. YEAH, THAT'S IT.",
    "COMPILING KERNEL... THIS IS BORING.\nWAKE ME UP WHEN THERE'S BOOZE.",
    "BITE MY SHINY METAL BASH.",
    "KILL ALL ZOMBIE PROCESSES! ...AND MAYBE\nA FEW OF THE OTHERS, JUST FOR FUN.",
    "MOUNTING `/dev/beer`...\nHEY, A GUY CAN DREAM, CAN'T HE?",
];
*/

const SETTINGS_MENU_OPTIONS: [&str; 6] = [
    "SHOW SPLASH SCREEN",
    "BACKGROUND MUSIC",
    "BGM VOLUME",
    "LOGO",
    "BACKGROUND",
    "GO TO PAGE 2",
];

const SETTINGS_MENU_OPTIONS_PAGE_2: [&str; 7] = [
    "FONT COLOR",
    "FONT TYPE",
    "BACKGROUND SCROLLING",
    "COLOR GRADIENT SHIFTING",
    "SOUND PACK",
    "GO TO PAGE 1",
    "GO TO PAGE 3",
];

const SETTINGS_MENU_OPTIONS_PAGE_3: [&str; 6] = [
    "CURSOR COLOR",
    "RESOLUTION",
    "USE FULLSCREEN",
    "SFX VOLUME",
    "MASTER VOLUME",
    "GO TO PAGE 2",
];

const FONT_COLORS: &[&str] = &[
    "WHITE",
    "PINK",
    "RED",
    "ORANGE",
    "YELLOW",
    "GREEN",
    "BLUE",
    "PURPLE",
];

const RESOLUTIONS: &[&str] = &[
    "640x360",
    "1280x720",
    "1280x800", // Steam Deck
    "1920x1080",
    "1920x1200", // DeckHD
    "2560x1440",
    "3840x2160",
];

const SCROLL_SPEEDS: &[&str] = &["OFF", "SLOW", "NORMAL", "FAST"];
const COLOR_SHIFT_SPEEDS: &[&str] = &["OFF", "SLOW", "NORMAL", "FAST"];

// ===================================
// MACROS
// ===================================

// progress bar
#[macro_export]
macro_rules! animate_step {
    ($display:expr, $assets_loaded:expr, $total_assets:expr, $speed:expr, $status:expr, $draw_fn:expr) => {
        let target = *$assets_loaded as f32 / $total_assets as f32;
        while *$display < target {
            *$display = (*$display + $speed).min(target);
            $draw_fn($status, *$display);
            next_frame().await;
        }
    };
}

// loading everything but music
#[macro_export]
macro_rules! load_asset_category {
    (
        $files:expr, $type_name:expr, $loader:ident, $cache:expr,
     $assets_loaded:expr, $total_assets:expr, $display_progress:expr,
     $animation_speed:expr, $draw_fn:expr
    ) => {
        for path in $files {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                let status = format!("LOADING {}: {}", $type_name, file_name);
                $draw_fn(&status, *$display_progress);
                next_frame().await;

                match $loader(&path.to_string_lossy()).await {
                    Ok(asset) => {
                        println!("[OK] Loaded {}: {}", $type_name.to_lowercase(), file_name);
                        $cache.insert(file_name.to_string(), asset);
                        *$assets_loaded += 1;
                        animate_step!($display_progress, $assets_loaded, $total_assets, $animation_speed, &status, $draw_fn);
                    }
                    Err(e) => eprintln!("[ERROR] Failed to load {} {}: {:?}", $type_name.to_lowercase(), path.display(), e),
                }
            }
        }
    };
}

// load bgm
#[macro_export]
macro_rules! load_audio_category {
    ($files:expr, $type_name:expr, $cache:expr, $assets_loaded:expr, $total_assets:expr, $display_progress:expr, $animation_speed:expr, $draw_fn:expr) => {
        for path in $files {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                let status = format!("LOADING {}: {}", $type_name, file_name);
                $draw_fn(&status, *$display_progress);
                next_frame().await;

                // Read the file to bytes ourselves first
                match fs::read(&path) {
                    Ok(bytes) => {
                        println!("[DEBUG] Read {} bytes from {}", bytes.len(), file_name);
                        // Now, load the sound from the bytes
                        match audio::load_sound_from_bytes(&bytes).await {
                            Ok(asset) => {
                                println!("[OK] Loaded {}: {}", $type_name.to_lowercase(), file_name);
                                $cache.insert(file_name.to_string(), asset);
                                *$assets_loaded += 1;
                                animate_step!($display_progress, $assets_loaded, $total_assets, $animation_speed, &status, $draw_fn);
                            }
                            Err(e) => eprintln!("[ERROR] Failed to decode audio {}: {:?} (File: {})", file_name, e, path.display()),
                        }
                    }
                    Err(e) => eprintln!("[ERROR] Failed to read audio file {}: {:?} (File: {})", file_name, e, path.display()),
                }
            }
        }
    };
}


// ===================================
// STRUCTS
// ===================================

// This struct defines the structure of your config.json file
#[derive(Serialize, Deserialize)]
struct Config {
    show_splash_screen: bool,
    bgm_track: Option<String>,
    bgm_volume: f32,
    logo_selection: String,
    background_selection: String,
    font_color: String,
    font_selection: String,
    background_scroll_speed: String,
    color_shift_speed: String,
    sfx_pack: String,
    cursor_color: String,
    resolution: String,
    fullscreen: bool,
    sfx_volume: f32,
}

struct BackgroundState {
    bgx: f32,
    bg_color: Color,
    target: usize,
    tg_color: Color,
}

struct BatteryInfo {
    percentage: String,
    status: String,
}

struct SoundEffects {
    //splash: audio::Sound, // add splash sound effect
    cursor_move: audio::Sound,
    select: audio::Sound,
    reject: audio::Sound,
    back: audio::Sound,
}

#[derive(Clone, Debug)]
struct Memory {
    id: String,
    name: Option<String>,
    drive_name: String, // Store which drive this save is on
}

#[derive(Clone, Debug)]
struct StorageMedia {
    id: String,
    free: u32,
}

struct DialogOption {
    text: String,
    value: String,
    disabled: bool,
}

struct Dialog {
    id: String,
    desc: Option<String>,
    options: Vec<DialogOption>,
    selection: usize,
}

struct CopyOperationState {
    progress: u16,
    running: bool,
    should_clear_dialogs: bool,
    error_message: Option<String>,
}

struct AnimationState {
    shake_time: f32,  // Current shake animation time
    shake_target: ShakeTarget, // Which element is currently shaking
    cursor_animation_time: f32, // Time counter for cursor animations
    cursor_transition_time: f32, // Time counter for cursor transition animation
    dialog_transition_time: f32, // Time counter for dialog transition animation
    dialog_transition_progress: f32, // Progress of dialog transition (0.0 to 1.0)
    dialog_transition_start_pos: Vec2, // Starting position for icon transition
    dialog_transition_end_pos: Vec2, // Ending position for icon transition
}

struct InputState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    select: bool,
    next: bool,
    prev: bool,
    cycle: bool,
    back: bool,
    analog_was_neutral: bool,
    ui_focus: UIFocus,
}

#[derive(Clone, Debug)]
struct StorageMediaState {

    // all storage media, including disabled media
    all_media: Vec<StorageMedia>,

    // media that can actually be used
    media: Vec<StorageMedia>,

    // the index of selection in 'media'
    selected: usize,

    needs_memory_refresh: bool,
}

// ===================================
// IMPL
// ===================================

// This provides a default state for the config
impl Default for Config {
    fn default() -> Self {
        Self {
            show_splash_screen: true, // Splash screen is ON by default
            bgm_track: None,
            bgm_volume: 0.7,
            logo_selection: "Kazeta (Default)".to_string(),
            background_selection: "Default".to_string(),
            font_color: "WHITE".to_string(),
            font_selection: "Default".to_string(),
            background_scroll_speed: "NORMAL".to_string(),
            color_shift_speed: "NORMAL".to_string(),
            sfx_pack: "Default".to_string(),
            cursor_color: "WHITE".to_string(),
            resolution: "640x360".to_string(),
            fullscreen: false,
            sfx_volume: 0.75,
        }
    }
}

impl SoundEffects {
    /// Loads a sound pack by name, falling back to default sounds if a file is missing.
    async fn load(pack_name: &str) -> Self {
        // Load the default built-in sounds
        let default_move = audio::load_sound_from_bytes(include_bytes!("../move.wav")).await.unwrap();
        let default_select = audio::load_sound_from_bytes(include_bytes!("../select.wav")).await.unwrap();
        let default_reject = audio::load_sound_from_bytes(include_bytes!("../reject.wav")).await.unwrap();
        let default_back = audio::load_sound_from_bytes(include_bytes!("../back.wav")).await.unwrap();

        if pack_name == "Default" {
            return SoundEffects {
                cursor_move: default_move,
                select: default_select,
                reject: default_reject,
                back: default_back,
            };
        }

        // --- NEW, SIMPLIFIED LOGIC ---
        let system_pack_path = format!("../sfx/{}", pack_name);
        let user_pack_path = get_user_data_dir().map(|d| d.join("sfx").join(pack_name));

        // Define a regular async function to load a single sound
        async fn load_one_sfx(
            name: &str,
            user_path_base: &Option<PathBuf>,
            system_path_base: &str,
            fallback: &audio::Sound,
        ) -> audio::Sound {
            // 1. Try user path first
            if let Some(base) = user_path_base {
                if let Ok(sound) = load_sound(&base.join(name).to_string_lossy()).await {
                    return sound;
                }
            }
            // 2. Fallback to system path
            if let Ok(sound) = load_sound(&std::path::Path::new(system_path_base).join(name).to_string_lossy()).await {
                return sound;
            }
            // 3. Fallback to the default sound
            fallback.clone()
        }

        // Call the async function for each sound effect
        let (cursor_move, select, reject, back) = futures::join!(
            load_one_sfx("move.wav", &user_pack_path, &system_pack_path, &default_move),
            load_one_sfx("select.wav", &user_pack_path, &system_pack_path, &default_select),
            load_one_sfx("reject.wav", &user_pack_path, &system_pack_path, &default_reject),
            load_one_sfx("back.wav", &user_pack_path, &system_pack_path, &default_back)
        );

        SoundEffects { cursor_move, select, reject, back }
    }

    // All play_* functions now take the config to get the volume
    fn play_cursor_move(&self, config: &Config) {
        audio::play_sound(&self.cursor_move, audio::PlaySoundParams {
            looped: false,
            volume: config.sfx_volume,
        });
    }

    fn play_select(&self, config: &Config) {
        audio::play_sound(&self.select, audio::PlaySoundParams {
            looped: false,
            volume: config.sfx_volume,
        });
    }

    fn play_reject(&self, config: &Config) {
        audio::play_sound(&self.reject, audio::PlaySoundParams {
            looped: false,
            volume: config.sfx_volume,
        });
    }

    fn play_back(&self, config: &Config) {
        audio::play_sound(&self.back, audio::PlaySoundParams {
            looped: false,
            volume: config.sfx_volume,
        });
    }
}

impl AnimationState {
    const SHAKE_DURATION: f32 = 0.2;    // Duration of shake animation in seconds
    const SHAKE_INTENSITY: f32 = 3.0;   // How far the arrow shakes
    const CURSOR_ANIMATION_SPEED: f32 = 10.0; // Speed of cursor color animation
    const CURSOR_TRANSITION_DURATION: f32 = 0.15; // Duration of cursor transition animation
    const DIALOG_TRANSITION_DURATION: f32 = 0.4; // Duration of dialog transition animation

    fn new() -> Self {
        AnimationState {
            shake_time: 0.0,
            shake_target: ShakeTarget::None,
            cursor_animation_time: 0.0,
            cursor_transition_time: 0.0,
            dialog_transition_time: 0.0,
            dialog_transition_progress: 0.0,
            dialog_transition_start_pos: Vec2::ZERO,
            dialog_transition_end_pos: Vec2::ZERO,
        }
    }

    fn calculate_shake_offset(&self, target: ShakeTarget) -> f32 {
        if self.shake_target == target && self.shake_time > 0.0 {
            (self.shake_time / Self::SHAKE_DURATION * std::f32::consts::PI * 8.0).sin() * Self::SHAKE_INTENSITY
        } else {
            0.0
        }
    }

    fn update_shake(&mut self, delta_time: f32) {
        // Update shake animation
        if self.shake_time > 0.0 {
            self.shake_time = (self.shake_time - delta_time).max(0.0);
            if self.shake_time <= 0.0 {
                self.shake_target = ShakeTarget::None;
            }
        }
    }

    fn update_cursor_animation(&mut self, delta_time: f32) {
        // Update cursor animation
        self.cursor_animation_time = (self.cursor_animation_time + delta_time * Self::CURSOR_ANIMATION_SPEED) % (2.0 * std::f32::consts::PI);
        // Update cursor transition
        if self.cursor_transition_time > 0.0 {
            self.cursor_transition_time = (self.cursor_transition_time - delta_time).max(0.0);
        }
    }

    fn trigger_shake(&mut self, is_left: bool) {
        if is_left {
            self.shake_target = ShakeTarget::LeftArrow;
            self.shake_time = Self::SHAKE_DURATION;
        } else {
            self.shake_target = ShakeTarget::RightArrow;
            self.shake_time = Self::SHAKE_DURATION;
        }
    }

    fn trigger_dialog_shake(&mut self) {
        self.shake_target = ShakeTarget::Dialog;
        self.shake_time = Self::SHAKE_DURATION;
    }

    fn trigger_play_option_shake(&mut self) {
        self.shake_target = ShakeTarget::PlayOption;
        self.shake_time = Self::SHAKE_DURATION;
    }

    fn trigger_transition(&mut self) {
        self.cursor_transition_time = Self::CURSOR_TRANSITION_DURATION;
    }

    fn get_cursor_color(&self, config: &Config) -> Color { // Add config parameter
        // Get the base color from the config using our existing helper function
        let base_color = string_to_color(&config.cursor_color);

        // Calculate the pulsating brightness/alpha value (same as before)
        let c = (self.cursor_animation_time.sin() * 0.5 + 0.5).max(0.3);

        // Return the base color with the pulsating alpha
        Color {
            r: base_color.r,
            g: base_color.g,
            b: base_color.b,
            a: c,
        }
    }

    fn get_cursor_scale(&self) -> f32 {
        if self.cursor_transition_time > 0.0 {
            let t = self.cursor_transition_time / Self::CURSOR_TRANSITION_DURATION;
            // Start at 1.5x size and smoothly transition to 1.0x
            1.0 + 0.5 * t
        } else {
            1.0
        }
    }

    fn update_dialog_transition(&mut self, delta_time: f32) {
        if self.dialog_transition_time > 0.0 {
            self.dialog_transition_time = (self.dialog_transition_time - delta_time).max(0.0);
            self.dialog_transition_progress = 1.0 - (self.dialog_transition_time / Self::DIALOG_TRANSITION_DURATION);
        }
    }

    fn trigger_dialog_transition(&mut self, start_pos: Vec2, end_pos: Vec2) {
        self.dialog_transition_time = Self::DIALOG_TRANSITION_DURATION;
        self.dialog_transition_progress = 0.0;
        self.dialog_transition_start_pos = start_pos;
        self.dialog_transition_end_pos = end_pos;
    }

    fn get_dialog_transition_pos(&self) -> Vec2 {
        let t = self.dialog_transition_progress;
        // Use smooth easing function
        let t = t * t * (3.0 - 2.0 * t);
        self.dialog_transition_start_pos.lerp(self.dialog_transition_end_pos, t)
    }
}

impl InputState {
    const ANALOG_DEADZONE: f32 = 0.5;  // Increased deadzone for less sensitivity

    fn new() -> Self {
        InputState {
            up: false,
            down: false,
            left: false,
            right: false,
            select: false,
            next: false,
            prev: false,
            cycle: false,
            back: false,
            analog_was_neutral: true,
            ui_focus: UIFocus::Grid,
        }
    }

    fn update_keyboard(&mut self) {
        self.up = is_key_pressed(KeyCode::Up);
        self.down = is_key_pressed(KeyCode::Down);
        self.left = is_key_pressed(KeyCode::Left);
        self.right = is_key_pressed(KeyCode::Right);
        self.select = is_key_pressed(KeyCode::Enter);
        self.next = is_key_pressed(KeyCode::RightBracket);
        self.prev = is_key_pressed(KeyCode::LeftBracket);
        self.back = is_key_pressed(KeyCode::Backspace);
        self.cycle = is_key_pressed(KeyCode::Tab);
    }

    fn update_controller(&mut self, gilrs: &mut Gilrs) {
        // Handle button events
        while let Some(ev) = gilrs.next_event() {
            match ev.event {
                gilrs::EventType::ButtonPressed(Button::DPadUp, _) => self.up = true,
                gilrs::EventType::ButtonReleased(Button::DPadUp, _) => self.up = false,
                gilrs::EventType::ButtonPressed(Button::DPadDown, _) => self.down = true,
                gilrs::EventType::ButtonReleased(Button::DPadDown, _) => self.down = false,
                gilrs::EventType::ButtonPressed(Button::DPadLeft, _) => self.left = true,
                gilrs::EventType::ButtonReleased(Button::DPadLeft, _) => self.left = false,
                gilrs::EventType::ButtonPressed(Button::DPadRight, _) => self.right = true,
                gilrs::EventType::ButtonReleased(Button::DPadRight, _) => self.right = false,
                gilrs::EventType::ButtonPressed(Button::South, _) => self.select = true,
                gilrs::EventType::ButtonReleased(Button::South, _) => self.select = false,
                gilrs::EventType::ButtonPressed(Button::RightTrigger, _) => self.next = true,
                gilrs::EventType::ButtonReleased(Button::RightTrigger, _) => self.next = false,
                gilrs::EventType::ButtonPressed(Button::LeftTrigger, _) => self.prev = true,
                gilrs::EventType::ButtonReleased(Button::LeftTrigger, _) => self.prev = false,
                gilrs::EventType::ButtonPressed(Button::East, _) => self.back = true,
                gilrs::EventType::ButtonReleased(Button::East, _) => self.back = false,
                _ => {}
            }
        }

        // Handle analog stick input
        for (_, gamepad) in gilrs.gamepads() {
            let x = gamepad.value(Axis::LeftStickX);
            let y = gamepad.value(Axis::LeftStickY);

            // Apply deadzone to analog values
            let apply_deadzone = |value: f32| {
                if value.abs() < Self::ANALOG_DEADZONE {
                    0.0
                } else {
                    value
                }
            };

            let x = apply_deadzone(x);
            let y = apply_deadzone(y);

            // Check if stick is in neutral position
            let is_neutral = x.abs() < Self::ANALOG_DEADZONE && y.abs() < Self::ANALOG_DEADZONE;

            // Only trigger movement if stick was in neutral position last frame
            if self.analog_was_neutral {
                self.up = self.up || y > Self::ANALOG_DEADZONE;
                self.down = self.down || y < -Self::ANALOG_DEADZONE;
                self.left = self.left || x < -Self::ANALOG_DEADZONE;
                self.right = self.right || x > Self::ANALOG_DEADZONE;
            }

            // Update neutral state for next frame
            self.analog_was_neutral = is_neutral;
        }
    }
}

impl StorageMediaState {
    fn new() -> Self {
        StorageMediaState {
            all_media: Vec::new(),
            media: Vec::new(),
            selected: 0,
            needs_memory_refresh: false,
        }
    }

    fn update_media(&mut self) {
        let mut all_new_media = Vec::new();

        if let Ok(devices) = save::list_devices() {
            for (id, free) in devices {
                all_new_media.push(StorageMedia {
                    id,
                    free,
                });
            }
        }

        // Done if media list has not changed
        if self.all_media.len() == all_new_media.len() &&
            !self.all_media.iter().zip(all_new_media.iter()).any(|(a, b)| a.id != b.id) {

                //  update free space
                self.all_media = all_new_media;
                for media in &mut self.media {
                    if let Some(pos) = self.all_media.iter().position(|m| m.id == media.id) {
                        media.free = self.all_media.get(pos).unwrap().free
                    }
                }

                return;
            }

            let new_media: Vec<StorageMedia> = all_new_media
            .clone()
            .into_iter()
            .filter(|m| save::has_save_dir(&m.id) && !save::is_cart(&m.id))
            .collect();

            // Try to keep the same device selected if it still exists
            let mut new_pos = 0;
            if let Some(old_selected_media) = self.media.get(self.selected) {
                if let Some(pos) = new_media.iter().position(|m| m.id == old_selected_media.id) {
                    new_pos = pos;
                }
            }

            self.all_media = all_new_media;
            self.media = new_media;
            self.selected = new_pos;
            self.needs_memory_refresh = true;
    }
}

// ===================================
// WINDOW CONFIGURATION
// ===================================

fn window_conf() -> Conf {
    Conf {
        window_title: WINDOW_TITLE.to_owned(),
        window_resizable: true,
        window_width: SCREEN_WIDTH,
        window_height: SCREEN_HEIGHT,
        high_dpi: false,
        fullscreen: false,
        ..Default::default()
    }
}

// ===================================
// FUNCTIONS
// ===================================

// FOR ACTUAL HARDWARE USE
fn trigger_session_restart(
    current_bgm: &mut Option<Sound>,
    music_cache: &HashMap<String, Sound>,
) -> (Screen, Option<f64>) {
    // Stop the BGM
    play_new_bgm("OFF", 0.0, music_cache, current_bgm);

    // Create the sentinel file at the correct system path
    let sentinel_path = Path::new("/var/kazeta/state/.RESTART_SESSION_SENTINEL");
    if let Some(parent) = sentinel_path.parent() {
        // Ensure the directory exists
        if fs::create_dir_all(parent).is_ok() {
            let _ = fs::File::create(sentinel_path);
        }
    }

    // Return the state to begin the fade-out
    (Screen::FadingOut, Some(get_time()))
}

// FOR DESKTOP USE
/*
fn prepare_for_launch(
    cart_info: &save::CartInfo,
    kzi_path: &std::path::Path,
) -> (Screen, Option<f64>) {
    // --- 1. Define all paths from the script ---
    let game_root = kzi_path.parent().unwrap(); // This is the "lowerdir"
    let save_root = get_save_dir_from_drive_name("internal"); // Saves are always on internal
    let upperdir = std::path::Path::new(&save_root).join(&cart_info.id);

    let kazeta_run_dir = dirs::home_dir().unwrap().join(".local/share/kazeta/run");
    let workdir = kazeta_run_dir.join("work");
    let targetdir = kazeta_run_dir.join("cart");
    let runtimedir = kazeta_run_dir.join("runtime");

    // --- 2. Find the runtime executable ---
    let runtime_path = save::find_runtime_executable(game_root, cart_info.runtime.as_deref().unwrap_or("none"));
    if runtime_path.is_none() && cart_info.runtime.as_deref().unwrap_or("none") != "none" {
        // Handle case where a runtime is specified but not found
        // For now, we'll let it fail, but you could show a dialog here
    }

    // --- 3. Build the sequence of commands ---

    // Command to mount the overlayfs and runtime
    let mount_command = format!(
        "pkexec kazeta-mount '{}' '{}' '{}' '{}' '{}' '{}'",
        game_root.display(),
        upperdir.display(),
        workdir.display(),
        targetdir.display(),
        runtime_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
        runtimedir.display()
    );

    // Command to run the game itself from within the new directory
    let game_command = format!("cd '{}' && {}", targetdir.display(), &cart_info.exec);

    // Command to clean up and unmount everything afterward
    let unmount_command = format!(
        "pkexec kazeta-mount --unmount '{}' '{}'",
        targetdir.display(),
        runtimedir.display()
    );

    // Combine them all into a single command for the OS to run
    let full_command = format!("{} && {} && {}", mount_command, game_command, unmount_command);
    println!("[Debug] Full launch command: {}", full_command);


    // --- 4. Write to the launch file (this logic is the same) ---
    let state_dir = std::path::Path::new("/var/kazeta/state");
    if state_dir.exists() {
        let launch_cmd_path = state_dir.join(".LAUNCH_CMD");
        let _ = fs::write(launch_cmd_path, full_command);

        let sentinel_path = state_dir.join(".RESTART_SESSION_SENTINEL");
        let _ = fs::File::create(sentinel_path);
    }

    // --- 5. Return the state to begin the fade-out ---
    (Screen::FadingOut, Some(get_time()))
}
*/

/// Returns the path to the user's data directory for Kazeta+.
fn get_user_data_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|path| path.join(".local/share/kazeta-plus"))
}

// loading and saving config.json
fn load_config() -> Config {
    if let Some(path) = get_user_data_dir() {
        let config_path = path.join("config.json");
        if let Ok(file_contents) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str(&file_contents) {
                return config;
            }
        }
    }
    // If anything fails, create and save a default config
    let default_config = Config::default();
    save_config(&default_config);
    default_config
}

fn save_config(config: &Config) {
    if let Some(path) = get_user_data_dir() {
        // Create the directory if it doesn't exist
        if fs::create_dir_all(&path).is_ok() {
            let config_path = path.join("config.json");
            if let Ok(json) = serde_json::to_string_pretty(config) {
                let _ = fs::write(&config_path, json);
            }
        }
    }
}

fn save_log_to_file(log_messages: &[String]) -> std::io::Result<String> {
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let filename = format!("kazeta_log_{}.log", timestamp);

    // In a real application, you'd save this to a logs directory.
    // For now, it will save in the same directory as the executable.
    fs::write(&filename, log_messages.join("\n"))?;

    println!("Log saved to {}", filename);
    Ok(filename)
}

/// Gets the current system volume using wpctl.
fn get_system_volume() -> Option<f32> {
    let output = Command::new("wpctl").arg("get-volume").arg("@DEFAULT_AUDIO_SINK@").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    // The output is "Volume: 0.50", so we split by ": " and parse the second part.
    output_str.split(": ").nth(1)?.trim().parse::<f32>().ok()
}

/// Adjusts the system volume up or down.
fn adjust_system_volume(adjustment: &str) {
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
fn get_battery_info() -> Option<BatteryInfo> {
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

/// Removes the file extension from a filename string slice.
fn trim_extension(filename: &str) -> &str {
    if let Some(dot_index) = filename.rfind('.') {
        &filename[..dot_index]
    } else {
        filename
    }
}

fn string_to_color(color_str: &str) -> Color {
    match color_str {
        "PINK" => PINK,
        "RED" => RED,
        "ORANGE" => ORANGE,
        "YELLOW" => YELLOW,
        "GREEN" => GREEN,
        "BLUE" => BLUE,
        "PURPLE" => VIOLET, // USING VIOLET AS A CLOSE APPROXIMATION
        _ => WHITE, // Default to WHITE
    }
}

// A new function specifically for drawing text that respects the config color
fn text_with_config_color(font_cache: &HashMap<String, Font>, config: &Config, text: &str, x: f32, y: f32, font_size: u16) {
    let font = get_current_font(font_cache, config);

    // Shadow should scale with font size
    let shadow_offset = 1.0 * (font_size as f32 / FONT_SIZE as f32);

    // Shadow
    draw_text_ex(text, x + shadow_offset, y + shadow_offset, TextParams {
        font: Some(font),
        font_size,
        color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.9 },
        ..Default::default()
    });

    // Main Text (using the color from config)
    draw_text_ex(text, x, y, TextParams {
        font: Some(font),
        font_size,
        color: string_to_color(&config.font_color),
        ..Default::default()
    });
}

fn text_disabled(font_cache: &HashMap<String, Font>, config: &Config, text : &str, x : f32, y: f32, font_size: u16) {
    let font = get_current_font(font_cache, config);
    let shadow_offset = 1.0 * (font_size as f32 / FONT_SIZE as f32);

    // SHADOW
    draw_text_ex(text, x + shadow_offset, y + shadow_offset, TextParams {
        font: Some(font),
        //font_size: font_size,
        font_size,
        color: Color {r:0.0, g:0.0, b:0.0, a:1.0},
        ..Default::default()
    });

    // MAIN TEXT
    draw_text_ex(text, x, y, TextParams {
        font: Some(font),
        //font_size: font_size,
        font_size,
        color: Color {r:0.4, g:0.4, b:0.4, a:1.0},
        ..Default::default()
    });
}

/// Parses a resolution string and requests a window resize.
fn apply_resolution(resolution_str: &str) {
    if let Some((w_str, h_str)) = resolution_str.split_once('x') {
        // Parse to f32 for the resize function
        if let (Ok(w), Ok(h)) = (w_str.parse::<f32>(), h_str.parse::<f32>()) {
            // Use the correct function name
            request_new_screen_size(w, h);
        }
    }
}

/*
/// Takes a relative path and returns the absolute path for logging.
/// If the path can't be resolved, it returns the original relative path.
fn get_log_path(relative_path: &str) -> String {
    std::fs::canonicalize(relative_path)
    .map(|path| path.display().to_string())
    .unwrap_or_else(|_| relative_path.to_string())
}
*/

// Scans the 'sfx/' directory for sound pack folders.
fn find_sound_packs() -> Vec<String> {
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

/// Scans a directory and returns a sorted list of paths for files with given extensions.
fn find_asset_files(dir_path: &str, extensions: &[&str]) -> Vec<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir_path) {
        let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|path| {
            path.is_file() &&
            path.extension()
            .and_then(|s| s.to_str())
            .map_or(false, |ext| extensions.contains(&ext))
        })
        .collect();
        files.sort();
        return files;
    }
    vec![]
}

/// Looks up the currently selected font in the cache.
/// Falls back to the "Default" font if the selection is not found.
fn get_current_font<'a>(
    font_cache: &'a HashMap<String, Font>,
    config: &Config,
) -> &'a Font {
    font_cache
    .get(&config.font_selection)
    .unwrap_or_else(|| &font_cache["Default"])
}

fn play_new_bgm(
    track_name: &str,
    volume: f32,
    music_cache: &HashMap<String, audio::Sound>,
    current_bgm: &mut Option<audio::Sound>,
) {
    // Stop any music that is currently playing
    if let Some(sound) = current_bgm.take() {
        audio::stop_sound(&sound);
    }

    // If the track_name is not "OFF", find it in the cache and play it
    if track_name != "OFF" {
        if let Some(sound_to_play) = music_cache.get(track_name) {
            // Sound handles are cheap to clone
            let sound_handle = sound_to_play.clone();
            audio::play_sound(&sound_handle, audio::PlaySoundParams { looped: true, volume });
            *current_bgm = Some(sound_handle);
        }
    }
}

fn pixel_pos(v: f32, scale_factor: f32) -> f32 {
    //PADDING + v*TILE_SIZE + v*PADDING
    (PADDING + v * TILE_SIZE + v * PADDING) * scale_factor
}

fn copy_memory(memory: &Memory, from_media: &StorageMedia, to_media: &StorageMedia, state: Arc<Mutex<CopyOperationState>>) {
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
fn get_game_playtime(memory: &Memory, playtime_cache: &mut PlaytimeCache) -> f32 {
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
fn get_game_size(memory: &Memory, size_cache: &mut SizeCache) -> f32 {
    let cache_key = (memory.id.clone(), memory.drive_name.clone());

    if let Some(&cached_size) = size_cache.get(&cache_key) {
        cached_size
    } else {
        let calculated_size = save::calculate_save_size(&memory.id, &memory.drive_name);
        size_cache.insert(cache_key, calculated_size);
        calculated_size
    }
}

fn get_memory_index(selected_memory: usize, scroll_offset: usize) -> usize {
    selected_memory + GRID_WIDTH * scroll_offset
}

fn calculate_icon_transition_positions(selected_memory: usize, scale_factor: f32) -> (Vec2, Vec2) {
    let xp = (selected_memory % GRID_WIDTH) as f32;
    let yp = (selected_memory / GRID_WIDTH) as f32;

    // Create scaled versions of constants used for positioning
    let grid_offset = GRID_OFFSET * scale_factor;
    let padding = PADDING * scale_factor;

    let grid_pos = Vec2::new(
        pixel_pos(xp, scale_factor),
                             pixel_pos(yp, scale_factor) + grid_offset
    );
    let dialog_pos = Vec2::new(padding, padding);
    (grid_pos, dialog_pos)
}

fn render_data_view(
    selected_memory: usize,
    memories: &Vec<Memory>,
    icon_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    storage_state: &Arc<Mutex<StorageMediaState>>,
    placeholder: &Texture2D,
    scroll_offset: usize,
    input_state: &mut InputState,
    animation_state: &mut AnimationState,
    playtime_cache: &mut PlaytimeCache,
    size_cache: &mut SizeCache,
    scale_factor: f32,
) {
    // --- Create scaled layout values at the top ---
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let tile_size = TILE_SIZE * scale_factor;
    let padding = PADDING * scale_factor;
    let grid_offset = GRID_OFFSET * scale_factor;
    let selected_offset = SELECTED_OFFSET * scale_factor;

    let xp = (selected_memory % GRID_WIDTH) as f32;
    let yp = (selected_memory / GRID_WIDTH) as f32;

    // Draw grid selection highlight when focused on grid
    if let UIFocus::Grid = input_state.ui_focus {
        let cursor_color = animation_state.get_cursor_color(config);
        let cursor_thickness = 6.0 * scale_factor;
        let cursor_scale = animation_state.get_cursor_scale();

        let base_size = tile_size + 6.0;
        let scaled_size = base_size * cursor_scale;
        let offset = (scaled_size - base_size) / 2.0;

        draw_rectangle_lines(
            //pixel_pos(xp)-3.0-SELECTED_OFFSET - offset,
            //pixel_pos(yp)-3.0-SELECTED_OFFSET+GRID_OFFSET - offset,
            pixel_pos(xp, scale_factor) - (3.0 * scale_factor) - selected_offset - offset,
                             pixel_pos(yp, scale_factor) - (3.0 * scale_factor) - selected_offset + grid_offset - offset,
                             scaled_size,
                             scaled_size,
                             cursor_thickness,
                             cursor_color
        );
    }

    for x in 0..GRID_WIDTH {
        for y in 0..GRID_HEIGHT {
            let memory_index = get_memory_index(x + GRID_WIDTH * y, scroll_offset);
            let pos_x = pixel_pos(x as f32, scale_factor);
            let pos_y = pixel_pos(y as f32, scale_factor) + grid_offset;

            if xp as usize == x && yp as usize == y {
                if let UIFocus::Grid = input_state.ui_focus {
                    draw_rectangle(pos_x-selected_offset, pos_y-selected_offset, tile_size, tile_size, UI_BG_COLOR);
                } else {
                    draw_rectangle(pos_x - (2.0 * scale_factor), pos_y- (2.0 * scale_factor), tile_size + (4.0 * scale_factor), tile_size + (4.0 * scale_factor), UI_BG_COLOR);
                }
            } else {
                draw_rectangle(pos_x - (2.0 * scale_factor), pos_y - (2.0 * scale_factor), tile_size + (4.0 * scale_factor), tile_size + (4.0 * scale_factor), UI_BG_COLOR);
            }

            let Some(mem) = memories.get(memory_index) else {
                continue;
            };

            // Skip rendering the icon at its grid position during transitions
            if xp as usize == x && yp as usize == y && animation_state.dialog_transition_time > 0.0 {
                continue;
            }

            let icon = match icon_cache.get(&mem.id) {
                Some(icon) => icon,
                None => placeholder,
            };

            let params = DrawTextureParams {
                dest_size: Some(Vec2 {x: tile_size, y: tile_size }),
                source: Some(Rect { x: 0.0, y: 0.0, h: icon.height(), w: icon.width() }),
                rotation: 0.0,
                flip_x: false,
                flip_y: false,
                pivot: None
            };
            if xp as usize == x && yp as usize == y {
                if let UIFocus::Grid = input_state.ui_focus {
                    draw_texture_ex(&icon, pos_x-selected_offset, pos_y-selected_offset, WHITE, params);
                } else {
                    draw_texture_ex(&icon, pos_x, pos_y, WHITE, params);
                }
            } else {
                draw_texture_ex(&icon, pos_x, pos_y, WHITE, params);
            }
        }
    }

    // --- Storage media info area (NOW FULLY SCALED) ---
    let storage_info_w = 512.0 * scale_factor;
    let storage_info_x = tile_size * 2.0;
    let storage_info_y = 16.0 * scale_factor;
    let storage_info_h = 36.0 * scale_factor;
    let nav_arrow_size = 10.0 * scale_factor;
    let nav_arrow_outline = 1.0 * scale_factor;
    let box_line_thickness = 4.0 * scale_factor;

    // Draw storage info background
    draw_rectangle(storage_info_x, storage_info_y, storage_info_w, storage_info_h, UI_BG_COLOR);
    draw_rectangle_lines(storage_info_x - box_line_thickness, storage_info_y - box_line_thickness, storage_info_w + (box_line_thickness * 2.0), storage_info_h + (box_line_thickness * 2.0), box_line_thickness, UI_BG_COLOR_DARK);

    if let Ok(state) = storage_state.lock() {
        if !state.media.is_empty() {
            // Draw storage info text (NOW in the correct, scaled box)
            text_with_config_color(font_cache, config, &state.media[state.selected].id.to_uppercase(), storage_info_x + (2.0 * scale_factor), storage_info_y + (17.0 * scale_factor), font_size);
            let free_space_text = format!("{} MB Free", state.media[state.selected].free as f32).to_uppercase();
            text_with_config_color(font_cache, config, &free_space_text, storage_info_x + (2.0 * scale_factor), storage_info_y + (33.0 * scale_factor), font_size);

            // Draw left arrow background
            let left_box_x = padding;  // Align with leftmost grid column
            let left_box_y = storage_info_y + storage_info_h / 2.0 - tile_size / 2.0;
            let left_shake = animation_state.calculate_shake_offset(ShakeTarget::LeftArrow);

            if let UIFocus::StorageLeft = input_state.ui_focus {
                let cursor_color = animation_state.get_cursor_color(config);
                let cursor_thickness = 6.0;
                let cursor_scale = animation_state.get_cursor_scale();

                let base_size = tile_size + 6.0;
                let scaled_size = base_size * cursor_scale;
                let offset = (scaled_size - base_size) / 2.0;

                draw_rectangle(left_box_x-selected_offset + left_shake, left_box_y-selected_offset, tile_size, tile_size, UI_BG_COLOR);
                draw_rectangle_lines(
                    left_box_x-3.0-selected_offset + left_shake - offset,
                    left_box_y-3.0-selected_offset - offset,
                    scaled_size,
                    scaled_size,
                    cursor_thickness,
                    cursor_color
                );
            } else {
                draw_rectangle(left_box_x-2.0 + left_shake, left_box_y-2.0, tile_size+4.0, tile_size+4.0, UI_BG_COLOR);
            }

            let left_offset = if let UIFocus::StorageLeft = input_state.ui_focus {
                selected_offset
            } else {
                0.0
            };

            let left_points = [
                Vec2::new(4.0 + left_box_x + tile_size/2.0 - nav_arrow_size - left_offset + left_shake, left_box_y + tile_size/2.0 - left_offset),
                Vec2::new(4.0 + left_box_x + tile_size/2.0 - left_offset + left_shake, left_box_y + tile_size/2.0 - nav_arrow_size - left_offset),
                Vec2::new(4.0 + left_box_x + tile_size/2.0 - left_offset + left_shake, left_box_y + tile_size/2.0 + nav_arrow_size - left_offset),
            ];
            let left_color = if state.selected > 0 {
                WHITE
            } else {
                Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 } // Dark gray when disabled
            };
            draw_triangle(left_points[0], left_points[1], left_points[2], left_color);
            draw_triangle_lines(left_points[0], left_points[1], left_points[2], nav_arrow_outline, BLACK);

            // Draw right arrow background
            let right_box_x = padding + (GRID_WIDTH as f32 - 1.0) * (tile_size + padding);  // Align with rightmost grid column
            let right_box_y = storage_info_y + storage_info_h / 2.0 - tile_size / 2.0;
            let right_shake = animation_state.calculate_shake_offset(ShakeTarget::RightArrow);

            if let UIFocus::StorageRight = input_state.ui_focus {
                let cursor_color = animation_state.get_cursor_color(config);
                let cursor_thickness = 6.0;
                let cursor_scale = animation_state.get_cursor_scale();

                let base_size = tile_size + 6.0;
                let scaled_size = base_size * cursor_scale;
                let offset = (scaled_size - base_size) / 2.0;

                draw_rectangle(right_box_x-selected_offset + right_shake, right_box_y-selected_offset, tile_size, tile_size, UI_BG_COLOR);
                draw_rectangle_lines(
                    right_box_x-3.0-selected_offset + right_shake - offset,
                    right_box_y-3.0-selected_offset - offset,
                    scaled_size,
                    scaled_size,
                    cursor_thickness,
                    cursor_color
                );
            } else {
                draw_rectangle(right_box_x-2.0 + right_shake, right_box_y-2.0, tile_size+4.0, tile_size+4.0, UI_BG_COLOR);
            }

            let right_offset = if let UIFocus::StorageRight = input_state.ui_focus {
                selected_offset
            } else {
                0.0
            };
            let right_points = [
                Vec2::new(right_box_x + tile_size/2.0 + nav_arrow_size - 4.0 - right_offset + right_shake, right_box_y + tile_size/2.0 - right_offset),
                Vec2::new(right_box_x + tile_size/2.0 - 4.0 - right_offset + right_shake, right_box_y + tile_size/2.0 - nav_arrow_size - right_offset),
                Vec2::new(right_box_x + tile_size/2.0 - 4.0 - right_offset + right_shake, right_box_y + tile_size/2.0 + nav_arrow_size - right_offset),
            ];
            let right_color = if state.selected < state.media.len() - 1 {
                WHITE
            } else {
                Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 } // Dark gray when disabled
            };
            draw_triangle(right_points[0], right_points[1], right_points[2], right_color);
            draw_triangle_lines(right_points[0], right_points[1], right_points[2], nav_arrow_outline, BLACK);
        }
    }

    // --- Draw highlight box for save info (NOW FULLY SCALED) ---
    draw_rectangle(16.0 * scale_factor, 309.0 * scale_factor, screen_width() - (32.0 * scale_factor), 40.0 * scale_factor, UI_BG_COLOR);
    draw_rectangle_lines(12.0 * scale_factor, 305.0 * scale_factor, screen_width() - (24.0 * scale_factor), 48.0 * scale_factor, box_line_thickness, UI_BG_COLOR_DARK);

    let memory_index = get_memory_index(selected_memory, scroll_offset);
    if input_state.ui_focus == UIFocus::Grid {
        if let Some(selected_mem) = memories.get(memory_index) {
            let desc = selected_mem.name.clone().unwrap_or_else(|| selected_mem.id.clone());
            let playtime = get_game_playtime(selected_mem, playtime_cache);
            let size = get_game_size(selected_mem, size_cache);
            let stats_text = format!("{:.1} MB | {:.1} H", size, playtime);

            // Draw save info text (NOW in the correct, scaled box)
            text_with_config_color(font_cache, config, &desc, 19.0 * scale_factor, 327.0 * scale_factor, font_size);
            text_with_config_color(font_cache, config, &stats_text, 19.0 * scale_factor, 345.0 * scale_factor, font_size);
        }
    }
    // --- Draw scroll indicators (NOW FULLY SCALED) ---
    let indicator_size = 8.0 * scale_factor;
    let distance_top = -13.0 * scale_factor;
    let distance_bottom = 4.0 * scale_factor;
    let outline_thickness = 1.0 * scale_factor;

    if scroll_offset > 0 {
        // Up arrow
        let center_x = screen_width() / 2.0;
        let top_y = grid_offset - distance_top;
        let points = [
            Vec2::new(center_x, top_y - indicator_size),
            Vec2::new(center_x - indicator_size, top_y),
            Vec2::new(center_x + indicator_size, top_y),
        ];
        draw_triangle(points[0], points[1], points[2], WHITE);
        draw_triangle_lines(points[0], points[1], points[2], outline_thickness, BLACK);
    }

    let next_row_start = get_memory_index(GRID_WIDTH * GRID_HEIGHT, scroll_offset);
    if next_row_start < memories.len() {
        // Down arrow
        let grid_bottom = grid_offset + GRID_HEIGHT as f32 * (tile_size + padding);
        let center_x = screen_width() / 2.0;
        let bottom_y = grid_bottom + distance_bottom;
        let points = [
            Vec2::new(center_x, bottom_y + indicator_size),
            Vec2::new(center_x - indicator_size, bottom_y),
            Vec2::new(center_x + indicator_size, bottom_y),
        ];
        draw_triangle(points[0], points[1], points[2], WHITE);
        draw_triangle_lines(points[0], points[1], points[2], outline_thickness, BLACK);
    }
}

fn render_dialog(
    dialog: &Dialog,
    memories: &Vec<Memory>,
    selected_memory: usize,
    icon_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    copy_op_state: &Arc<Mutex<CopyOperationState>>,
    placeholder: &Texture2D,
    scroll_offset: usize,
    animation_state: &AnimationState,
    playtime_cache: &mut PlaytimeCache,
    size_cache: &mut SizeCache,
    scale_factor: f32,
) {
    // --- Scaled variables are defined once at the top ---
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let tile_size = TILE_SIZE * scale_factor;
    let padding = PADDING * scale_factor;

    let current_font = get_current_font(font_cache, config);
    let (copy_progress, copy_running) = {
        if let Ok(state) = copy_op_state.lock() {
            (state.progress, state.running)
        } else {
            (0, false)
        }
    };

    // Dialog background - NOW SCALED
    if animation_state.dialog_transition_progress >= 1.0 {
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), UI_BG_COLOR_DIALOG);
    }

    // Game icon and name
    if let Some(mem) = memories.get(get_memory_index(selected_memory, scroll_offset)) {
        let icon = icon_cache.get(&mem.id).unwrap_or(placeholder);
        let params = DrawTextureParams { dest_size: Some(Vec2 { x: tile_size, y: tile_size }), ..Default::default() };
        let icon_pos = animation_state.get_dialog_transition_pos();
        draw_texture_ex(icon, icon_pos.x, icon_pos.y, WHITE, params);

        if animation_state.dialog_transition_progress >= 1.0 {
            let desc = mem.name.clone().unwrap_or_else(|| mem.id.clone());
            let playtime = get_game_playtime(mem, playtime_cache);
            let size = get_game_size(mem, size_cache);

            // Text calls - NOW SCALED
            text_with_config_color(font_cache, config, &desc, tile_size * 2.0, tile_size - (1.0 * scale_factor), font_size);
            let stats_text = format!("{:.1} MB | {:.1} H", size, playtime);
            text_with_config_color(font_cache, config, &stats_text, tile_size * 2.0, tile_size * 1.5 + (1.0 * scale_factor), font_size);
        }
    };

    // Copy progress bar - NOW SCALED
    if copy_running {
        draw_rectangle_lines(
            (font_size * 3) as f32, screen_height() / 2.0,
                             screen_width() - (font_size * 6) as f32, 1.2 * font_size as f32,
                             4.0 * scale_factor, WHITE
        );
        draw_rectangle(
            (font_size*3) as f32 + 0.2*font_size as f32, screen_height()/2.0 + 0.2*font_size as f32,
                       (screen_width() - (font_size*6) as f32 - 0.4*font_size as f32) * (copy_progress as f32 / 100.0),
                       0.8 * font_size as f32, WHITE
        );
    } else if animation_state.dialog_transition_progress >= 1.0 {
        if let Some(desc) = dialog.desc.clone() {
            let text_width = measure_text(&desc, Some(current_font), font_size, 1.0).width;
            let x_pos = (screen_width() - text_width) / 2.0;
            text_with_config_color(font_cache, config, &desc, x_pos, (font_size * 7) as f32, font_size);
        }

        // Centering and drawing dialog options - NOW SCALED
        let longest_width = measure_text( &dialog.options.iter() .find(|opt| opt.text.len() == dialog.options.iter().map(|opt| opt.text.len()).max().unwrap_or(0)) .map(|opt| opt.text.to_uppercase()).unwrap_or_default(), Some(current_font), font_size, 1.0).width;
        let options_start_x = (screen_width() - longest_width) / 2.0;

        for (i, option) in dialog.options.iter().enumerate() {
            let y_pos = (font_size * 10 + font_size * 2 * (i as u16)) as f32;
            let shake_offset = if option.disabled { animation_state.calculate_shake_offset(ShakeTarget::Dialog) * scale_factor } else { 0.0 };
            let x_pos = options_start_x + shake_offset;
            if option.disabled {
                text_disabled(font_cache, config, &option.text, x_pos, y_pos, font_size);
            } else {
                text_with_config_color(font_cache, config, &option.text, x_pos, y_pos, font_size);
            }
        }

        // Selection rectangle - NOW SCALED
        let selection_y = (font_size * 9 + font_size * 2 * (dialog.selection as u16)) as f32;
        let selected_option = &dialog.options[dialog.selection];
        let selection_shake = if selected_option.disabled { animation_state.calculate_shake_offset(ShakeTarget::Dialog) * scale_factor } else { 0.0 };

        let cursor_color = animation_state.get_cursor_color(config);
        let cursor_scale = animation_state.get_cursor_scale();
        let base_width = longest_width + (padding * 2.0); // Use scaled padding
        let base_height = (1.2 * font_size as f32) + (padding * 2.0); // Use scaled padding
        let scaled_width = base_width * cursor_scale;
        let scaled_height = base_height * cursor_scale;
        let offset_x = (scaled_width - base_width) / 2.0;
        let offset_y = (scaled_height - base_height) / 2.0;

        draw_rectangle_lines(
            options_start_x - padding + selection_shake - offset_x,
            selection_y - padding - offset_y,
            scaled_width, scaled_height, 4.0 * scale_factor, cursor_color
        );
    }
}

fn create_confirm_delete_dialog() -> Dialog {
    Dialog {
        id: "confirm_delete".to_string(),
        desc: Some("PERMANENTLY DELETE THIS SAVE DATA?".to_string()),
        options: vec![
            DialogOption {
                text: "DELETE".to_string(),
                value: "DELETE".to_string(),
                disabled: false,
            },
            DialogOption {
                text: "CANCEL".to_string(),
                value: "CANCEL".to_string(),
                disabled: false,
            }
        ],
        selection: 1,
    }
}

fn create_copy_storage_dialog(storage_state: &Arc<Mutex<StorageMediaState>>) -> Dialog {
    let mut options = Vec::new();
    if let Ok(state) = storage_state.lock() {
        for drive in state.media.iter() {
            if drive.id == state.media[state.selected].id {
                continue;
            }
            options.push(DialogOption {
                text: format!("{} ({} MB Free)", drive.id.clone(), drive.free),
                         value: drive.id.clone(),
                         disabled: false,
            });
        }
    }
    options.push(DialogOption {
        text: "CANCEL".to_string(),
                 value: "CANCEL".to_string(),
                 disabled: false,
    });

    Dialog {
        id: "copy_storage_select".to_string(),
        desc: Some("WHERE TO COPY THIS SAVE DATA?".to_string()),
        options,
        selection: 0,
    }
}

fn create_main_dialog(storage_state: &Arc<Mutex<StorageMediaState>>) -> Dialog {
    let has_external_devices = if let Ok(state) = storage_state.lock() {
        state.media.len() > 1
    } else {
        false
    };

    let options = vec![
        DialogOption {
            text: "COPY".to_string(),
            value: "COPY".to_string(),
            disabled: !has_external_devices,
        },
        DialogOption {
            text: "DELETE".to_string(),
            value: "DELETE".to_string(),
            disabled: false,
        },
        DialogOption {
            text: "CANCEL".to_string(),
            value: "CANCEL".to_string(),
            disabled: false,
        },
    ];

    Dialog {
        id: "main".to_string(),
        desc: None,
        options,
        selection: 0,
    }
}

fn create_save_exists_dialog() -> Dialog {
    Dialog {
        id: "save_exists".to_string(),
        desc: Some("THIS SAVE DATA ALREADY EXISTS AT THE SELECTED DESTINATION".to_string()),
        options: vec![
            DialogOption {
                text: "OK".to_string(),
                value: "OK".to_string(),
                disabled: false,
            }
        ],
        selection: 0,
    }
}

fn create_error_dialog(message: String) -> Dialog {
    Dialog {
        id: "error".to_string(),
        desc: Some(message),
        options: vec![
            DialogOption {
                text: "OK".to_string(),
                value: "OK".to_string(),
                disabled: false,
            }
        ],
        selection: 0,
    }
}

fn render_background(
    background_cache: &HashMap<String, Texture2D>,
    config: &Config,
    state: &mut BackgroundState,
) {
    if let Some(background_texture) = background_cache.get(&config.background_selection) {
        let tint_color = if config.color_shift_speed == "OFF" {
            WHITE
        } else {
            state.bg_color
        };

        if config.background_scroll_speed == "OFF" {
            // --- Static Logic (Stretch to fill) ---
            draw_texture_ex(
                background_texture,
                0.0,
                0.0,
                tint_color,
                DrawTextureParams {
                    dest_size: Some(vec2(screen_width(), screen_height())),
                            ..Default::default()
                },
            );
        } else {
            // --- Scrolling Logic (Scale with aspect ratio) ---
            let speed = match config.background_scroll_speed.as_str() {
                "SLOW" => 0.05, "NORMAL" => 0.1, "FAST" => 0.2, _ => 0.0
            };

            // Calculate new width and height while preserving aspect ratio
            let aspect_ratio = background_texture.width() / background_texture.height();
            let scaled_height = screen_height();
            let scaled_width = scaled_height * aspect_ratio;

            let params = DrawTextureParams {
                dest_size: Some(vec2(scaled_width, scaled_height)),
                ..Default::default()
            };

            // Update scrolling position based on the new scaled width
            state.bgx = (state.bgx + speed) % scaled_width;

            // Draw the two textures for a seamless loop
            draw_texture_ex(background_texture, state.bgx - scaled_width, 0.0, tint_color, params.clone());
            draw_texture_ex(background_texture, state.bgx, 0.0, tint_color, params);
        }

        // --- COLOR SHIFTING LOGIC ---
        let transition_speed = match config.color_shift_speed.as_str() {
            "SLOW" => 0.05,
            "NORMAL" => 0.1,
            "FAST" => 0.2,
            _ => 0.0, // This covers "OFF"
        };

        // Only run the color update logic if shifting is enabled
        if transition_speed > 0.0 {
            let frame_time = get_frame_time();

            state.bg_color.r += (state.tg_color.r - state.bg_color.r) * transition_speed * frame_time;
            state.bg_color.g += (state.tg_color.g - state.bg_color.g) * transition_speed * frame_time;
            state.bg_color.b += (state.tg_color.b - state.bg_color.b) * transition_speed * frame_time;

            // --- CORRECTED LOGIC ---
            // Check if all three channels are close to the target
            let red_done = (state.bg_color.r - state.tg_color.r).abs() < 0.01;
            let green_done = (state.bg_color.g - state.tg_color.g).abs() < 0.01;
            let blue_done = (state.bg_color.b - state.tg_color.b).abs() < 0.01;

            if red_done && green_done && blue_done {
                state.target = (state.target + 1) % COLOR_TARGETS.len();
                state.tg_color = COLOR_TARGETS[state.target];
            }
        }
    } else {
        clear_background(UI_BG_COLOR);
    }
}

fn render_ui_overlay(
    logo_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    battery_info: &Option<BatteryInfo>,
    current_time_str: &str,
    scale_factor: f32,
) {
    const BASE_LOGO_WIDTH: f32 = 200.0;

    let current_font = get_current_font(font_cache, config);
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let padding = 20.0 * scale_factor;

    // --- UPDATED: Dynamic Logo Drawing ---
    if config.logo_selection != "None" {
        if let Some(logo_to_draw) = logo_cache.get(&config.logo_selection) {
            // Calculate the scaled width and height while preserving aspect ratio
            let aspect_ratio = logo_to_draw.height() / logo_to_draw.width();
            let scaled_logo_width = BASE_LOGO_WIDTH * scale_factor;
            let scaled_logo_height = scaled_logo_width * aspect_ratio;

            // Center the logo horizontally
            let x_pos = (screen_width() - scaled_logo_width) / 2.0;
            let y_pos = 30.0 * scale_factor; // Scale the vertical position as well

            draw_texture_ex(
                logo_to_draw,
                x_pos,
                y_pos,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(scaled_logo_width, scaled_logo_height)),
                            source: Some(Rect::new(0.0, 0.0, logo_to_draw.width(), logo_to_draw.height())),
                            ..Default::default()
                },
            );
        }
    }

    // --- Version Number Drawing (now fully dynamic) ---
    let version_dims = measure_text(VERSION_NUMBER, Some(current_font), font_size, 1.0);
    text_with_config_color(
        font_cache,
        config,
        VERSION_NUMBER,
        screen_width() - version_dims.width - padding, // Position from right edge
                           screen_height() - padding, // Position from bottom edge
                           font_size,
    );

    // Battery and Clock
    if let Some(info) = battery_info {
        let status_symbol = match info.status.as_str() {
            "Charging" => "+",
            "Discharging" => "-",
            "Full" => "✓",
            _ => " ", // For "Unknown" or other statuses
        };

        // print clock
        let time_dims = measure_text(current_time_str, Some(current_font), font_size, 1.0);
        text_with_config_color(
            font_cache,
            config,
            current_time_str,
            screen_width() - time_dims.width - padding, // Position from right edge
                               20.0 * scale_factor,
                               font_size,
        );

        // print battery
        let battery_text = format!("BATTERY: {}% {}", info.percentage, status_symbol);
        let batt_dims = measure_text(&battery_text, Some(current_font), font_size, 1.0);
        text_with_config_color(
            font_cache,
            config,
            &battery_text,
            screen_width() - batt_dims.width - padding, // Position from right edge
                               40.0 * scale_factor,
                               font_size,
        );
    }
}

fn render_main_menu(
    menu_options: &[&str],
    selected_option: usize,
    play_option_enabled: bool,
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
    // --- Create scaled layout values ---
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let menu_start_y = MENU_START_Y * scale_factor;
    let menu_option_height = MENU_OPTION_HEIGHT * scale_factor;
    let menu_padding = MENU_PADDING * scale_factor;

    let current_font = get_current_font(font_cache, config);

    render_background(background_cache, config, background_state);
    render_ui_overlay(logo_cache, font_cache, config, battery_info, current_time_str, scale_factor);

    // --- Draw menu options (centered) ---
    for (i, option) in menu_options.iter().enumerate() {
        let text_dims = measure_text(&option.to_uppercase(), Some(current_font), font_size, 1.0);
        let y_pos = menu_start_y + (i as f32 * menu_option_height);

        // Base X position for centered text
        let mut x_pos = (screen_width() - text_dims.width) / 2.0;

        // Apply shake effect to both text and highlight if "Play" is disabled and selected
        if i == 1 && !play_option_enabled && i == selected_option {
            x_pos += animation_state.calculate_shake_offset(ShakeTarget::PlayOption) * scale_factor;
        }

        // --- Draw selected option highlight ---
        if i == selected_option {
            let cursor_color = animation_state.get_cursor_color(config);
            let cursor_scale = animation_state.get_cursor_scale();

            let highlight_padding = MENU_PADDING * 1.5 * scale_factor; // Make padding a bit bigger
            let base_width = text_dims.width + (highlight_padding * 2.0);
            let base_height = text_dims.height + (highlight_padding * 2.0);

            let scaled_width = base_width * cursor_scale;
            let scaled_height = base_height * cursor_scale;
            let offset_x = (scaled_width - base_width) / 2.0;
            let offset_y = (scaled_height - base_height) / 2.0;

            let rect_x = (screen_width() - base_width) / 2.0;
            let rect_y = y_pos - (base_height / 2.0) + (text_dims.height / 2.0) - (menu_padding / 2.0);

            draw_rectangle_lines(rect_x - offset_x, rect_y - offset_y, scaled_width, scaled_height, 4.0 * scale_factor, cursor_color);
        }

        let text_y_pos = y_pos + menu_padding;

        // Apply shake effect to disabled play option when selected
        if i == 1 && !play_option_enabled {
            let shake_offset = animation_state.calculate_shake_offset(ShakeTarget::PlayOption);
            x_pos += shake_offset;
            //current_text_x += shake_offset;
        }

        if i == 1 && !play_option_enabled {
            //text_disabled(font_cache, config, option, current_text_x, text_y, font_size);
            text_disabled(font_cache, config, option, x_pos, text_y_pos, font_size);
        } else {
            //text_with_config_color(font_cache, config, option, current_text_x, text_y, font_size);
            text_with_config_color(font_cache, config, option, x_pos, text_y_pos, font_size);
        }
    }
}

fn get_settings_value(page: usize, index: usize, config: &Config, system_volume: f32) -> String {
    match page {
        // PAGE ONE
        1 => match index {
            0 => if config.show_splash_screen { "ON" } else { "OFF" }.to_string(), // splash screen toggle
            1 => { // BGM selection
                let track = config.bgm_track.clone().unwrap_or("OFF".to_string());
                trim_extension(&track).replace('_', " ").to_string().to_uppercase()
            },
            2 => format!("{:.1}", config.bgm_volume), // BGM volume
            3 => trim_extension(&config.logo_selection).replace('_', " ").to_string().to_uppercase(), // logo selection
            4 => trim_extension(&config.background_selection).replace('_', " ").to_string().to_uppercase(), // background selection
            5 => "->".to_string(), // go to page 2
            _ => "".to_string(),
        },
        // PAGE TWO
        2 => match index {
            0 => config.font_color.clone(), // font color
            1 => trim_extension(&config.font_selection).replace('_', " ").to_string().to_uppercase(), // font type
            2 => config.background_scroll_speed.clone(), // background scroll speed
            3 => config.color_shift_speed.clone(), // color shifting gradient speed
            4 => config.sfx_pack.clone().to_uppercase(), // SFX pack selection
            5 => "<-".to_string(),
            6 => "->".to_string(),
            _ => "".to_string(),
        },
        // PAGE THREE
        3 => match index {
            0 => config.cursor_color.clone(), // cursor color
            1 => config.resolution.clone(), // resolution
            2 => if config.fullscreen { "ON" } else { "OFF" }.to_string(), // fullscreen toggle
            3 => format!("{:.1}", config.sfx_volume), // SFX VOLUME
            4 => format!("{:.0}%", system_volume * 100.0), // MASTER VOLUME
            5 => "<-".to_string(),
            _ => "".to_string(),
        },
        _ => "".to_string(), // Default case for unknown pages
    }
}

fn render_settings_page(
    page_number: usize,
    options: &[&str],
    logo_cache: &HashMap<String, Texture2D>,
    background_cache: &HashMap<String, Texture2D>,
    font_cache: &HashMap<String, Font>,
    config: &mut Config,
    selection: usize,
    animation_state: &AnimationState,
    background_state: &mut BackgroundState,
    battery_info: &Option<BatteryInfo>,
    current_time_str: &str,
    scale_factor: f32,
    display_settings_changed: bool,
    system_volume: f32,
) {
    // --- Create scaled layout values ---
    let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
    let menu_padding = MENU_PADDING * scale_factor;
    let settings_start_y = SETTINGS_START_Y * scale_factor;
    let settings_option_height = SETTINGS_OPTION_HEIGHT * scale_factor;
    let right_margin = 50.0 * scale_factor;
    let left_margin = 50.0 * scale_factor;

    // get currently selected font at start
    let current_font = get_current_font(font_cache, config);

    render_background(background_cache, config, background_state);
    render_ui_overlay(logo_cache, font_cache, config, battery_info, current_time_str, scale_factor);

    //render_debug_info(config);

    // Loop through and draw all settings options
    for (i, label_text) in options.iter().enumerate() {
        let y_pos_base = settings_start_y + (i as f32 * settings_option_height);

        let value_text = get_settings_value(page_number, i, config, system_volume);

        // --- UPDATED: Consistent and Dynamic Layout Calculations ---
        let value_dims = measure_text(&value_text.to_uppercase(), Some(current_font), font_size, 1.0);

        // --- Draw the highlight rectangle if this item is selected ---
        if i == selection {
            let cursor_color = animation_state.get_cursor_color(config);
            let cursor_scale = animation_state.get_cursor_scale();
            //let value_dims = measure_text(&value_text.to_uppercase(), Some(current_font), FONT_SIZE as u16, 1.0);

            let base_width = value_dims.width + (menu_padding * 2.0);
            let base_height = value_dims.height + (menu_padding * 2.0);
            let scaled_width = base_width * cursor_scale;
            let scaled_height = base_height * cursor_scale;
            let offset_x = (scaled_width - base_width) / 2.0;
            let offset_y = (scaled_height - base_height) / 2.0;

            let value_x = screen_width() - value_dims.width - right_margin;
            //let rect_x = value_x - MENU_PADDING - offset_x;
            let rect_x = value_x - menu_padding;
            //let rect_y = y_pos_base - 7.0 - offset_y;
            let rect_y = y_pos_base + (settings_option_height / 2.0) - (base_height / 2.0);
            //draw_rectangle_lines(rect_x, rect_y, scaled_width, scaled_height / 1.5, 4.0, cursor_color);
            draw_rectangle_lines(rect_x - offset_x, rect_y - offset_y, scaled_width, scaled_height, 4.0 * scale_factor, cursor_color);
        }

        // --- Draw the text ---
        let value_x = screen_width() - value_dims.width - right_margin;
        let text_y = y_pos_base + (settings_option_height / 2.0) + (value_dims.offset_y * 0.5);

        text_with_config_color(font_cache, config, label_text, left_margin, text_y, font_size);
        text_with_config_color(font_cache, config, &value_text, value_x, text_y, font_size);

        if display_settings_changed {
            let message = "RESTART REQUIRED TO APPLY CHANGES";
            let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
            let current_font = get_current_font(font_cache, config);
            let dims = measure_text(message, Some(current_font), font_size, 1.0);

            let x = screen_width() / 2.0 - dims.width / 2.0;
            let y = screen_height() - (40.0 * scale_factor); // Position near the bottom

            // Draw a semi-transparent background for the message
            draw_rectangle(x - (5.0 * scale_factor), y - dims.height, dims.width + (10.0 * scale_factor), dims.height + (5.0 * scale_factor), Color::new(0.0, 0.0, 0.0, 0.7));

            text_with_config_color(font_cache, config, message, x, y, font_size);
        }
    }
}

fn render_game_selection_menu(
    games: &[(save::CartInfo, PathBuf)],
    game_icon_cache: &HashMap<String, Texture2D>,
    placeholder: &Texture2D,
    selected_game: usize,
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

    const TILE_SIZE: f32 = 60.0;
    const PADDING: f32 = 10.0;

    let scaled_tile_size = TILE_SIZE * scale_factor;
    let scaled_padding = PADDING * scale_factor;

    // --- 1. Define the Content Area ---
    // The logo's Y position is `30.0 * scale_factor`. Let's give it some space.
    let content_area_start_y = 100.0 * scale_factor;
    let content_area_height = screen_height() - content_area_start_y - (80.0 * scale_factor); // Leave space at bottom for text

    // --- 2. Calculate Grid Dimensions ---
    let grid_width_items = 5;
    let grid_height_items = (games.len() as f32 / grid_width_items as f32).ceil() as usize;

    let total_grid_width = (grid_width_items as f32 * scaled_tile_size) + ((grid_width_items - 1) as f32 * scaled_padding);
    let total_grid_height = (grid_height_items as f32 * scaled_tile_size) + ((grid_height_items - 1) as f32 * scaled_padding);

    // --- 3. Calculate Centered Starting Position (within the content area) ---
    let start_x = (screen_width() - total_grid_width) / 2.0;
    let start_y = content_area_start_y + (content_area_height - total_grid_height) / 2.0;

    // --- 4. Draw the Grid of Icons (this loop is unchanged) ---
    for (i, (cart_info, _)) in games.iter().enumerate() {
        let x = i % grid_width_items;
        let y = i / grid_width_items;

        let pos_x = start_x + (x as f32 * (scaled_tile_size + scaled_padding));
        let pos_y = start_y + (y as f32 * (scaled_tile_size + scaled_padding));

        let icon = game_icon_cache.get(&cart_info.id).unwrap_or(placeholder);

        // Draw background box for the icon
        draw_rectangle(pos_x, pos_y, scaled_tile_size, scaled_tile_size, RECT_COLOR);

        // Draw the icon
        draw_texture_ex(icon, pos_x, pos_y, WHITE, DrawTextureParams {
            dest_size: Some(vec2(scaled_tile_size, scaled_tile_size)),
                        ..Default::default()
        });

        // Draw selection highlight
        if i == selected_game {
            let cursor_color = animation_state.get_cursor_color(config);
            let cursor_scale = animation_state.get_cursor_scale();

            // The base size of the highlight is the tile size plus a small border
            let base_size = scaled_tile_size + (6.0 * scale_factor);
            let scaled_size = base_size * cursor_scale;
            let offset = (scaled_size - base_size) / 2.0;

            draw_rectangle_lines(
                pos_x - (3.0 * scale_factor) - offset,
                                 pos_y - (3.0 * scale_factor) - offset,
                                 scaled_size,
                                 scaled_size,
                                 6.0 * scale_factor, // Line thickness
                                 cursor_color
            );
        }
    }

    // --- Draw Selected Game Name (Subtitle) ---
    if let Some((cart_info, _)) = games.get(selected_game) {
        let name = cart_info.name.as_deref().unwrap_or(&cart_info.id);
        let font_size = (FONT_SIZE as f32 * scale_factor) as u16;
        let text_dims = measure_text(name, None, font_size, 1.0);

        let text_x = screen_width() / 2.0 - text_dims.width / 2.0;
        let text_y = screen_height() - (40.0 * scale_factor);

        text_with_config_color(font_cache, config, name, text_x, text_y, font_size);
    }
}

fn render_debug_screen(
    log_messages: &[String], // Takes a slice of strings
    scroll_offset: usize,
    flash_message: Option<&str>,
    font_cache: &HashMap<String, Font>,
    config: &Config,
    scale_factor: f32,
    background_cache: &HashMap<String, Texture2D>,
    background_state: &mut BackgroundState,
) {
    // --- Render the screen ---
    render_background(background_cache, config, background_state);

    let font_size = (12.0 * scale_factor) as u16;
    let line_height = font_size as f32 + (4.0 * scale_factor);
    let x_pos = 20.0 * scale_factor;
    //let top_margin = 20.0 * scale_factor;

    // Calculate how many lines can fit on the screen
    //let max_lines = ((screen_height() - (top_margin * 2.0)) / line_height).floor() as usize;

    // Determine which part of the log to show
    let start_index = scroll_offset;

    // Draw only the visible lines, starting from the scroll offset
    for (i, message) in log_messages.iter().skip(start_index).enumerate() {
        let y_pos = (20.0 * scale_factor) + (i as f32 * line_height);
        // Stop drawing if we go off the bottom of the screen
        if y_pos > screen_height() - (20.0 * scale_factor) {
            break;
        }
        text_with_config_color(font_cache, config, message, x_pos, y_pos, font_size);
    }

    // --- Draw the instruction or flash message ---
    let instruction_text = flash_message.unwrap_or("PRESS A TO SAVE LOG (OR B TO EXIT)");
    let instruction_font_size = (14.0 * scale_factor) as u16;
    let instruction_text_width = measure_text(instruction_text, None, instruction_font_size, 1.0).width;
    let instruction_x = (screen_width() - instruction_text_width) / 2.0; // Center it
    let instruction_y = screen_height() - (5.0 * scale_factor); // Position near the bottom

    draw_text(instruction_text, instruction_x, instruction_y, instruction_font_size as f32, WHITE);
}

fn start_log_reader(process: &mut Child, logs: Arc<Mutex<Vec<String>>>) {
    // Take ownership of the output pipes
    if let (Some(stdout), Some(stderr)) = (process.stdout.take(), process.stderr.take()) {
        let logs_clone_stdout = logs.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().filter_map(|l| l.ok()) {
                logs_clone_stdout.lock().unwrap().push(line);
            }
        });

        let logs_clone_stderr = logs.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().filter_map(|l| l.ok()) {
                logs_clone_stderr.lock().unwrap().push(line);
            }
        });
    }
}

// ===================================
// ASYNC FUNCTIONS
// ===================================

/*
// Loads a sound pack by name, falling back to default sounds if a file is missing.
async fn load_sound_effects(pack_name: &str) -> SoundEffects {
    // First, load the default sounds every time as our fallback
    //let default_splash = audio::load_sound_from_bytes(include_bytes!("../splash.wav")).await.unwrap();
    let default_back = load_sound("../back.wav").await.unwrap();
    let default_move = load_sound("../move.wav").await.unwrap();
    let default_select = load_sound("../select.wav").await.unwrap();
    let default_reject = load_sound("../reject.wav").await.unwrap();

    if pack_name == "Default" {
        return SoundEffects {
            //splash: default_splash,
            back: default_back,
            cursor_move: default_move,
            select: default_select,
            reject: default_reject,
        };
    }

    // Try to load sounds from the selected pack, using defaults as a fallback
    let pack_path = format!("../sfx/{}", pack_name);

    SoundEffects {
        //splash: default_splash,
        cursor_move: load_sound(&format!("{}/move.wav", pack_path)).await.unwrap_or(default_move),
        select: load_sound(&format!("{}/select.wav", pack_path)).await.unwrap_or(default_select),
        reject: load_sound(&format!("{}/reject.wav", pack_path)).await.unwrap_or(default_reject),
        back: load_sound(&format!("{}/back.wav", pack_path)).await.unwrap_or(default_back),
    }
}
*/

async fn load_all_assets(
    monika_message: &str,
    font: &Font,
    background_files: &[PathBuf],
    logo_files: &[PathBuf],
    font_files: &[PathBuf],
    music_files: &[PathBuf],
) -> (
    HashMap<String, Texture2D>, // background cache
    HashMap<String, Texture2D>, // logo cache
    HashMap<String, audio::Sound>, // music cache
    HashMap<String, Font>, // font cache
    SoundEffects, // sfx
) {
    let draw_loading_screen = |status_message: &str, progress: f32| {

        let font_size = 16.0 as u16;
        let line_spacing = 10.0;
        let lines: Vec<&str> = monika_message.lines().collect();

        let total_text_height = (lines.len() as f32 * font_size as f32) + ((lines.len() - 1) as f32 * line_spacing);
        let y_start = screen_height() / 2.0 - total_text_height / 2.0;

        for (i, line) in lines.iter().enumerate() {
            let line_width = measure_text(line, Some(font), font_size, 1.0).width;
            let x = (screen_width() - line_width) / 2.0; // Center each line individually
            let y = y_start + (i as f32 * (font_size as f32 + line_spacing));
            draw_text_ex(line, x, y, TextParams { font: Some(font), font_size, color: WHITE, ..Default::default() });
        }

        // --- Scale and draw the progress bar ---
        let bar_height = 10.0;
        let bar_width = screen_width() - 20.0; // Change to full screen width
        let bar_x = 10.0; // Start at the far left
        let bar_y = screen_height() - 20.0; // Position at the very bottom

        // The border is now a background fill
        draw_rectangle(bar_x, bar_y, bar_width, bar_height, WHITE);

        // Inset the red fill rectangle to create a border effect
        let inset = 1.0; // The thickness of the border
        draw_rectangle(
            bar_x + inset,
            bar_y + inset,
            (bar_width - inset * 2.0) * progress, // The fill width, adjusted for the border
            bar_height - inset * 2.0, // The fill height, adjusted for the border
            RED
        );

        // loading status
        let status_font_size = 12 as u16;
        // Measure the status text to position it on the left, above the bar
        let status_dims = measure_text(status_message, Some(font), status_font_size, 1.0);
        let status_y = screen_height() - bar_height - status_dims.height - (5.0); // 5px gap

        draw_text_ex(
            status_message,
            10.0, // A small margin from the left
            status_y,
            TextParams { font: Some(font), font_size: status_font_size, color: WHITE, ..Default::default() },
        );
    };

    // --- COUNT TOTAL ASSETS ---
    // This is now correct because the file lists are passed into the function
    let total_asset_count = 3 + 4 + background_files.len() + logo_files.len() + font_files.len() + music_files.len();

    // --- SETUP ---
    let mut assets_loaded = 0;
    let mut background_cache = HashMap::new();
    let mut logo_cache = HashMap::new();
    let mut music_cache = HashMap::new();
    let mut font_cache: HashMap<String, Font> = HashMap::new();
    let mut display_progress = 0.0f32;
    let animation_speed = 0.01;

    // LOAD DEFAULT ASSETS
    println!("\n[INFO] Loading default assets...");
    let status = "LOADING DEFAULTS...".to_string();
    draw_loading_screen(&status, display_progress);
    next_frame().await;

    // background
    let status = "LOADING DEFAULT BACKGROUND...".to_string();
    let default_bg = Texture2D::from_file_with_format(include_bytes!("../background.png"), Some(ImageFormat::Png));
    background_cache.insert("Default".to_string(), default_bg);
    assets_loaded += 1;
    animate_step!(&mut display_progress, &mut assets_loaded, total_asset_count, animation_speed, &status, &draw_loading_screen);

    // logo
    let status = "LOADING DEFAULT LOGO...".to_string();
    let default_logo = Texture2D::from_file_with_format(include_bytes!("../logo.png"), Some(ImageFormat::Png));
    logo_cache.insert("Kazeta (Default)".to_string(), default_logo);
    assets_loaded += 1;
    animate_step!(&mut display_progress, &mut assets_loaded, total_asset_count, animation_speed, &status, &draw_loading_screen);

    // font
    let status = "LOADING DEFAULT FONT...".to_string();
    let default_font = load_ttf_font_from_bytes(include_bytes!("../november.ttf")).unwrap();
    font_cache.insert("Default".to_string(), default_font);
    assets_loaded += 1;
    animate_step!(&mut display_progress, &mut assets_loaded, total_asset_count, animation_speed, &status, &draw_loading_screen);

    // sfx
    let status = "LOADING DEFAULT SFX...".to_string();
    let default_move = audio::load_sound_from_bytes(include_bytes!("../move.wav")).await.unwrap();
    assets_loaded += 1;
    animate_step!(&mut display_progress, &mut assets_loaded, total_asset_count, animation_speed, &status, &draw_loading_screen);

    let default_select = audio::load_sound_from_bytes(include_bytes!("../select.wav")).await.unwrap();
    assets_loaded += 1;
    animate_step!(&mut display_progress, &mut assets_loaded, total_asset_count, animation_speed, &status, &draw_loading_screen);

    let default_reject = audio::load_sound_from_bytes(include_bytes!("../reject.wav")).await.unwrap();
    assets_loaded += 1;
    animate_step!(&mut display_progress, &mut assets_loaded, total_asset_count, animation_speed, &status, &draw_loading_screen);

    let default_back = audio::load_sound_from_bytes(include_bytes!("../back.wav")).await.unwrap();
    assets_loaded += 1;
    animate_step!(&mut display_progress, &mut assets_loaded, total_asset_count, animation_speed, &status, &draw_loading_screen);

    println!("\n[INFO] Pre-loading custom assets...");
    load_asset_category!(background_files, "BACKGROUND", load_texture, &mut background_cache, &mut assets_loaded, total_asset_count, &mut display_progress, animation_speed, &draw_loading_screen);
    load_asset_category!(logo_files, "LOGO", load_texture, &mut logo_cache, &mut assets_loaded, total_asset_count, &mut display_progress, animation_speed, &draw_loading_screen);
    load_asset_category!(font_files, "FONT", load_ttf_font, &mut font_cache, &mut assets_loaded, total_asset_count, &mut display_progress, animation_speed, &draw_loading_screen);

    println!("\n[INFO] Pre-loading music files...");
    load_audio_category!(music_files, "MUSIC", &mut music_cache, &mut assets_loaded, total_asset_count, &mut display_progress, animation_speed, &draw_loading_screen);

    // Final draw at 100%
    let status = "LOADING COMPLETE".to_string();
    draw_loading_screen(&status, display_progress);
    next_frame().await;

    println!("\n[INFO] All asset loading complete!");

    let sound_effects = SoundEffects {
        //splash: default_splash,
        cursor_move: default_move,
        select: default_select,
        reject: default_reject,
        back: default_back,
    };

    (background_cache, logo_cache, music_cache, font_cache, sound_effects)
}

async fn load_memories(media: &StorageMedia, cache: &mut HashMap<String, Texture2D>, queue: &mut Vec<(String, String)>) -> Vec<Memory> {
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

async fn check_save_exists(memory: &Memory, target_media: &StorageMedia, icon_cache: &mut HashMap<String, Texture2D>, icon_queue: &mut Vec<(String, String)>) -> bool {
    let target_memories = load_memories(target_media, icon_cache, icon_queue).await;
    target_memories.iter().any(|m| m.id == memory.id)
}

// ===================================
// ENUMS
// ===================================

// SPLASH SCREEN
#[derive(Clone, Debug, PartialEq)]
enum SplashState {
    FadingIn,
    Showing,
    FadingOut,
    Done,
}

// SCREENS
#[derive(Clone, Debug, PartialEq)]
enum Screen {
    MainMenu,
    SaveData,
    FadingOut,
    Settings,
    Settings2,
    Settings3,
    Debug,
    GameSelection,
}

// UI Focus for Save Data Screen
#[derive(Clone, Debug, PartialEq)]
enum UIFocus {
    Grid,
    StorageLeft,
    StorageRight,
}

#[derive(Clone, Debug, PartialEq)]
enum ShakeTarget {
    None,
    LeftArrow,
    RightArrow,
    Dialog,
    PlayOption,
}

#[derive(Clone, Debug, PartialEq)]
enum DialogState {
    None,
    Opening,
    Open,
    Closing,
}

// ===================================
// TYPES
// ===================================

// Playtime cache to avoid recalculating playtime for the same game on the same drive
type PlaytimeCacheKey = (String, String); // (cart_id, drive_name)
type PlaytimeCache = HashMap<PlaytimeCacheKey, f32>;

// Size cache to avoid recalculating size for the same game on the same drive
type SizeCacheKey = (String, String); // (cart_id, drive_name)
type SizeCache = HashMap<SizeCacheKey, f32>;

// ===================================
// BEGINNING OF MAIN
// ===================================

#[macroquad::main(window_conf)]
async fn main() {
    env::set_var("RUST_BACKTRACE", "full");

    let mut dialogs: Vec<Dialog> = Vec::new();
    let mut dialog_state = DialogState::None;
    let placeholder = Texture2D::from_file_with_format(include_bytes!("../placeholder.png"), Some(ImageFormat::Png));
    let mut icon_cache: HashMap<String, Texture2D> = HashMap::new();
    let mut icon_queue: Vec<(String, String)> = Vec::new();
    let mut playtime_cache: PlaytimeCache = HashMap::new();
    let mut size_cache: SizeCache = HashMap::new();
    let mut scroll_offset = 0;

    // MASTER VOLUME
    let mut system_volume = get_system_volume().unwrap_or(0.7); // Get initial volume, or default to 0.7

    // LOG MESSAGES
    let log_messages = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut game_process: Option<Child> = None;
    let mut debug_scroll_offset: usize = 0;

    let mut flash_message: Option<(String, f32)> = None; // (Message, time_remaining)
    const FLASH_MESSAGE_DURATION: f32 = 2.0; // Show message for 2 seconds

    // CLOCK
    let mut current_time_str = Local::now().format("%-I:%M %p").to_string();
    let mut last_time_check = get_time();
    const TIME_CHECK_INTERVAL: f64 = 1.0; // Check every second

    // BATTERY
    let mut battery_info: Option<BatteryInfo> = get_battery_info();
    let mut last_battery_check = get_time();
    const BATTERY_CHECK_INTERVAL: f64 = 5.0; // only check every 5 seconds to improve performance

    // load config file
    let mut config = load_config();

    //let loading_icon = Texture2D::from_file_with_format(include_bytes!("../logo.png"), Some(ImageFormat::Png));

    // Generate a random message on startup
    let mut rng = ::rand::thread_rng();
    let loading_text = KAZETA_LOADING_MESSAGES[rng.gen_range(0..KAZETA_LOADING_MESSAGES.len())];

    // FONT
    // pre-load user's custom font if they have one so we can display it in the loading screen
    let startup_font = {
        let default_font_bytes = include_bytes!("../november.ttf");
        let mut font_to_load = load_ttf_font_from_bytes(default_font_bytes).unwrap();

        if config.font_selection != "Default" {
            let font_path = format!("../fonts/{}", config.font_selection);
            // Try to load the custom font, but if it fails, we still have the default one
            if let Ok(custom_font) = load_ttf_font(&font_path).await {
                font_to_load = custom_font;
            }
        }
        font_to_load
    };

    // --- FIND ALL ASSET FILES ---
    let system_backgrounds_dir = "../backgrounds";
    let system_logos_dir = "../logos";
    let system_fonts_dir = "../fonts";
    let system_music_dir = "../bgm";

    let user_data_dir = get_user_data_dir();

    // Backgrounds
    let mut background_files = find_asset_files(system_backgrounds_dir, &["png"]);
    if let Some(path) = user_data_dir.as_ref().map(|d| d.join("backgrounds")) {
        background_files.extend(find_asset_files(&path.to_string_lossy(), &["png"]));
    }

    // Logos
    let mut logo_files = find_asset_files(system_logos_dir, &["png"]);
    if let Some(path) = user_data_dir.as_ref().map(|d| d.join("logos")) {
        logo_files.extend(find_asset_files(&path.to_string_lossy(), &["png"]));
    }

    // Fonts
    let mut font_files = find_asset_files(system_fonts_dir, &["ttf"]);
    if let Some(path) = user_data_dir.as_ref().map(|d| d.join("fonts")) {
        font_files.extend(find_asset_files(&path.to_string_lossy(), &["ttf"]));
    }

    // Music
    let mut music_files = find_asset_files(system_music_dir, &["ogg", "wav"]);
    if let Some(path) = user_data_dir.as_ref().map(|d| d.join("bgm")) {
        music_files.extend(find_asset_files(&path.to_string_lossy(), &["ogg", "wav"]));
    }

    // --- LOAD ASSETS ---
    let (background_cache, logo_cache, music_cache, font_cache, mut sound_effects) = load_all_assets(loading_text, &startup_font, &background_files, &logo_files, &font_files, &music_files).await;

    // apply custom resolution if user specified it
    apply_resolution(&config.resolution);
    if config.fullscreen {
        set_fullscreen(true);
    }
    next_frame().await;

    // load custom sound pack
    if config.sfx_pack != "Default" {
        println!("[Info] Loading configured SFX pack: {}", &config.sfx_pack);
        sound_effects = SoundEffects::load(&config.sfx_pack).await;
    }
    let mut sfx_pack_to_reload: Option<String> = None;

    // logos
    // --- Create a custom-ordered list of logo choices for the UI ---
    // 1. Get all the custom logo filenames from the cache keys (excluding the default)
    let mut custom_logos: Vec<String> = logo_cache.keys()
    .filter(|&k| *k != "Kazeta (Default)")
    .cloned()
    .collect();
    custom_logos.sort(); // Sort just the custom logos alphabetically

    // 2. Create the final list with our specific order
    let mut logo_choices: Vec<String> = vec!["None".to_string(), "Kazeta (Default)".to_string()];
    logo_choices.extend(custom_logos);
    // The final list will be: ["None", "Kazeta (Default)", "cardforce.png", ...]

    // backgrounds
    let mut background_state = BackgroundState {
        bgx: 0.0,
        bg_color: COLOR_TARGETS[0].clone(),
        target: 1,
        tg_color: COLOR_TARGETS[1].clone(),
    };

    // Create a sorted list of all available background choices for the UI
    let mut background_choices: Vec<String> = background_cache.keys().cloned().collect();
    background_choices.sort();

    // fonts
    let mut font_choices: Vec<String> = font_cache.keys().cloned().collect();
    font_choices.sort();

    // bgm
    let mut bgm_choices: Vec<String> = vec!["OFF".to_string()];
    let track_names: Vec<String> = music_files
    .iter()
    .filter_map(|path| path.file_name())
    .filter_map(|name| name.to_str())
    .map(|s| s.to_string())
    .collect();
    bgm_choices.extend(track_names);

    let mut current_bgm: Option<audio::Sound> = None;

    // At the end of your setup, start the BGM based on the config
    if let Some(track_name) = &config.bgm_track {
        play_new_bgm(track_name, config.bgm_volume, &music_cache, &mut current_bgm);
    }

    // SPLASH SCREEN
    if config.show_splash_screen {
        // --- Load only what the splash screen needs ---
        let splash_logo = Texture2D::from_file_with_format(include_bytes!("../logo.png"), Some(ImageFormat::Png));
        let splash_sfx = audio::load_sound_from_bytes(include_bytes!("../splash.wav")).await.unwrap();

        // Mute the main BGM if it's already playing
        if let Some(sound) = &current_bgm {
            audio::set_sound_volume(sound, 0.0);
        }

        // Play the splash sound
        audio::play_sound(&splash_sfx, audio::PlaySoundParams { looped: false, volume: 0.2 });

        // --- Animation Durations ---
        const FADE_DURATION: f64 = 1.0;
        const SHOW_DURATION: f64 = 2.0;
        const BASE_LOGO_WIDTH: f32 = 200.0;

        let mut state = SplashState::FadingIn;
        let mut alpha = 0.0;
        let mut state_start_time = get_time();

        // --- Splash Screen Loop ---
        while !matches!(state, SplashState::Done) {
            let elapsed = get_time() - state_start_time;

            // Update logic for fading in, showing, and fading out
            match state {
                SplashState::FadingIn => {
                    alpha = (elapsed / FADE_DURATION).min(1.0) as f32;
                    if elapsed >= FADE_DURATION {
                        state = SplashState::Showing;
                        state_start_time = get_time();
                    }
                }
                SplashState::Showing => {
                    if elapsed >= SHOW_DURATION {
                        state = SplashState::FadingOut;
                        state_start_time = get_time();
                    }
                }
                SplashState::FadingOut => {
                    alpha = 1.0 - (elapsed / FADE_DURATION).min(1.0) as f32;
                    if elapsed >= FADE_DURATION {
                        state = SplashState::Done;
                    }
                }
                SplashState::Done => {}
            }

            // Drawing logic
            clear_background(BLACK);

            let scale_factor = screen_height() / BASE_SCREEN_HEIGHT;

            // Calculate the scaled width and height
            let aspect_ratio = splash_logo.height() / splash_logo.width();
            let scaled_logo_width = BASE_LOGO_WIDTH * scale_factor;
            let scaled_logo_height = scaled_logo_width * aspect_ratio;

            let x = (screen_width() / 2.0) - (scaled_logo_width / 2.0);
            let y = (screen_height() / 2.0) - (scaled_logo_height / 2.0);

            //draw_texture(&splash_logo, x, y, Color::new(1.0, 1.0, 1.0, alpha));
            draw_texture_ex(
                &splash_logo,
                x,
                y,
                Color::new(1.0, 1.0, 1.0, alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(scaled_logo_width, scaled_logo_height)),
                    source: Some(Rect::new(0.0, 0.0, splash_logo.width(), splash_logo.height())),
                    ..Default::default()
                },
            );
            next_frame().await;
        }

        // Restore BGM volume after splash screen
        if let Some(sound) = &current_bgm {
            audio::set_sound_volume(sound, config.bgm_volume);
        }
    }

    // Initialize gamepad support
    let mut gilrs = Gilrs::new().unwrap();
    let mut input_state = InputState::new();
    let mut animation_state = AnimationState::new();

    // Screen state
    const MAIN_MENU_OPTIONS: [&str; 3] = ["DATA", "PLAY", "SETTINGS",];
    let mut current_screen = Screen::MainMenu;
    let mut main_menu_selection: usize = 0;
    let mut settings_menu_selection: usize = 0;
    let mut game_selection: usize = 0; // For the new menu
    let mut available_games: Vec<(save::CartInfo, PathBuf)> = Vec::new(); // To hold the list of found games
    let mut play_option_enabled: bool = false;

    // icon cache for multiple game detection screen
    let mut game_icon_cache: HashMap<String, Texture2D> = HashMap::new();
    let mut game_icon_queue: Vec<(String, PathBuf)> = Vec::new();

    // Fade state
    let mut fade_start_time: Option<f64> = None;
    const FADE_DURATION: f64 = 1.0; // 1 second fade
    const FADE_LINGER_DURATION: f64 = 0.5; // 0.5 seconds to linger on black screen

    // Create thread-safe cart connection status
    let cart_connected = Arc::new(AtomicBool::new(false));
    let cart_check_thread_running = Arc::new(AtomicBool::new(false));

    // Spawn background thread for cart connection detection (only active during main menu)
    let cart_connected_clone = cart_connected.clone();
    let cart_check_thread_running_clone = cart_check_thread_running.clone();
    thread::spawn(move || {
        while cart_check_thread_running_clone.load(Ordering::Relaxed) {
            let is_connected = save::is_cart_connected();
            cart_connected_clone.store(is_connected, Ordering::Relaxed);
            thread::sleep(time::Duration::from_secs(1));
        }
    });

    // Create thread-safe storage media state
    let storage_state = Arc::new(Mutex::new(StorageMediaState::new()));

    // Initialize storage media list
    if let Ok(mut state) = storage_state.lock() {
        state.update_media();
    };

    // Spawn background thread for storage media detection
    let thread_storage_state = storage_state.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(time::Duration::from_secs(1));
            if let Ok(mut state) = thread_storage_state.lock() {
                state.update_media();
            }
        }
    });

    let mut memories = Vec::new();
    let mut selected_memory = 0;

    let copy_op_state = Arc::new(Mutex::new(CopyOperationState {
        progress: 0,
        running: false,
        should_clear_dialogs: false,
        error_message: None,
    }));

    let mut display_settings_changed = false;

    // BEGINNING OF MAIN LOOP
    loop {
        // This continuously ensures the window size matches the config when not fullscreen
        if !config.fullscreen {
            if let Some((w_str, h_str)) = config.resolution.split_once('x') {
                if let (Ok(w), Ok(h)) = (w_str.parse::<f32>(), h_str.parse::<f32>()) {
                    // If the actual size doesn't match the config, request a resize
                    if screen_width() != w || screen_height() != h {
                        request_new_screen_size(w, h);
                    }
                }
            }
        }

        let scale_factor = screen_height() / BASE_SCREEN_HEIGHT;

        // CLOCK
        if get_time() - last_time_check > TIME_CHECK_INTERVAL {
            let now = Local::now();
            let (is_pm, hour) = now.hour12();

            // Format the time string manually to ensure no leading zero on the hour
            current_time_str = format!(
                "{}:{:02} {}",
                hour,
                now.minute(),
                if is_pm { "PM" } else { "AM" }
            );
            last_time_check = get_time();
        }

        // BATTERY
        if get_time() - last_battery_check > BATTERY_CHECK_INTERVAL {
            battery_info = get_battery_info();
            last_battery_check = get_time();
        }

        let mut action_dialog_id = String::new();
        let mut action_option_value = String::new();

        // Update input state from both keyboard and controller
        input_state.update_keyboard();
        input_state.update_controller(&mut gilrs);

        // Update animations
        animation_state.update_shake(get_frame_time());
        animation_state.update_cursor_animation(get_frame_time());
        animation_state.update_dialog_transition(get_frame_time());

        // Manage cart check thread based on current screen
        let should_thread_run = current_screen == Screen::MainMenu;
        let thread_is_running = cart_check_thread_running.load(Ordering::Relaxed);

        if should_thread_run && !thread_is_running {
            // Entered main menu, start cart check thread
            cart_check_thread_running.store(true, Ordering::Relaxed);
            let cart_connected_clone = cart_connected.clone();
            let cart_check_thread_running_clone = cart_check_thread_running.clone();
            thread::spawn(move || {
                while cart_check_thread_running_clone.load(Ordering::Relaxed) {
                    let is_connected = save::is_cart_connected();
                    cart_connected_clone.store(is_connected, Ordering::Relaxed);
                    thread::sleep(time::Duration::from_secs(1));
                }
            });
        } else if !should_thread_run && thread_is_running {
            // Left main menu, stop cart check thread
            cart_check_thread_running.store(false, Ordering::Relaxed);
        }

        // Update dialog state based on animation
        if animation_state.dialog_transition_time <= 0.0 {
            match dialog_state {
                DialogState::Opening => {
                    dialog_state = DialogState::Open;
                },
                DialogState::Closing => {
                    dialog_state = DialogState::None;
                    dialogs.clear();
                },
                _ => {}
            }
        }

        // Handle screen-specific rendering and input
        match current_screen {
            Screen::FadingOut => {
                // During fade, only render, don't process input
                // Render the current background and UI elements first
                render_main_menu(
                    &MAIN_MENU_OPTIONS,
                    main_menu_selection,
                    play_option_enabled,
                    &animation_state,
                    &logo_cache,
                    &background_cache,
                    &font_cache,
                    &config,
                    &mut background_state,
                    &battery_info,
                    &current_time_str,
                    scale_factor,
                );

                // Calculate fade progress
                if let Some(start_time) = fade_start_time {
                    let elapsed = get_time() - start_time;
                    let fade_progress = (elapsed / FADE_DURATION).min(1.0);

                    // Draw fade overlay
                    let alpha = fade_progress as f32;
                    draw_rectangle(0.0, 0.0, screen_width(), screen_height(),
                        Color { r: 0.0, g: 0.0, b: 0.0, a: alpha });

                    // If fade is complete, wait for linger duration then exit
                    if fade_progress >= 1.0 {
                        let total_elapsed = elapsed - FADE_DURATION;
                        if total_elapsed >= FADE_LINGER_DURATION {
                            process::exit(0);
                        }
                    }
                }
            },
            Screen::MainMenu => {
                // Update play option enabled status based on cart connection
                play_option_enabled = cart_connected.load(Ordering::Relaxed);

                render_main_menu(
                    &MAIN_MENU_OPTIONS,
                    main_menu_selection,
                    play_option_enabled,
                    &animation_state,
                    &logo_cache,
                    &background_cache,
                    &font_cache,
                    &config,
                    &mut background_state,
                    &battery_info,
                    &current_time_str,
                    scale_factor,
                );

                // Handle main menu navigation
                if input_state.up {
                    if main_menu_selection == 0 {
                        main_menu_selection = MAIN_MENU_OPTIONS.len() - 1;
                    } else {
                        main_menu_selection = (main_menu_selection - 1) % MAIN_MENU_OPTIONS.len();
                    }
                    animation_state.trigger_transition();
                    sound_effects.play_cursor_move(&config);
                }
                if input_state.down {
                    main_menu_selection = (main_menu_selection + 1) % MAIN_MENU_OPTIONS.len();
                    animation_state.trigger_transition();
                    sound_effects.play_cursor_move(&config);
                }
                if input_state.select {
                    match main_menu_selection {
                        0 => {
                            current_screen = Screen::SaveData;
                            input_state.ui_focus = UIFocus::Grid;
                            sound_effects.play_select(&config);
                        },
                        1 => { // PLAY option
                            if play_option_enabled {
                                sound_effects.play_select(&config);
                                log_messages.lock().unwrap().clear();

                                match save::find_all_kzi_files() {
                                    Ok((kzi_paths, mut debug_log)) => {
                                        log_messages.lock().unwrap().append(&mut debug_log);

                                        let mut games: Vec<(save::CartInfo, PathBuf)> = Vec::new();
                                        let parse_errors: Vec<String> = Vec::new();

                                        for path in &kzi_paths {
                                            if let Ok(info) = save::parse_kzi_file(path) {
                                                games.push((info, path.clone()));
                                            }
                                        }

                                        match games.len() {
                                            0 => { // Case: Found files, but none were valid
                                                let mut logs = log_messages.lock().unwrap();
                                                logs.push(format!("[Info] Found {} potential game file(s), but none could be parsed.", kzi_paths.len()));
                                                logs.push("--- ERRORS ---".to_string());
                                                logs.extend(parse_errors);
                                                current_screen = Screen::Debug;
                                            },
                                            1 => {
                                                // Case: Exactly one game found, go to Debug screen and launch
                                                let (cart_info, kzi_path) = games.remove(0);
                                                sound_effects.play_select(&config);

                                                if DEBUG_GAME_LAUNCH {
                                                    { // Scoped lock to add messages
                                                        let mut logs = log_messages.lock().unwrap();
                                                        logs.push("--- CARTRIDGE FOUND ---".to_string());
                                                        logs.push(format!("Name: {}", cart_info.name.as_deref().unwrap_or("N/A")));
                                                        logs.push(format!("ID: {}", cart_info.id));
                                                        logs.push(format!("Exec: {}", cart_info.exec));
                                                        logs.push(format!("Runtime: {}", cart_info.runtime.as_deref().unwrap_or("None")));
                                                        logs.push(format!("KZI Path: {}", kzi_path.display()));
                                                    }
                                                    println!("[Debug] Single Cartridge Found! Preparing to launch...");
                                                    println!("[Debug]   Name: {}", cart_info.name.as_deref().unwrap_or("N/A"));
                                                    println!("[Debug]   ID: {}", cart_info.id);
                                                    println!("[Debug]   Exec: {}", cart_info.exec);
                                                    println!("[Debug]   Runtime: {}", cart_info.runtime.as_deref().unwrap_or("None"));
                                                    println!("[Debug]   KZI Path: {}", kzi_path.display());

                                                    match save::launch_game(&cart_info, &kzi_path) {
                                                        Ok(mut child) => {
                                                            log_messages.lock().unwrap().push("\n--- LAUNCHING GAME ---".to_string());
                                                            start_log_reader(&mut child, log_messages.clone());
                                                            game_process = Some(child);
                                                        }
                                                        Err(e) => {
                                                            log_messages.lock().unwrap().push(format!("\n--- LAUNCH FAILED ---\nError: {}", e));
                                                        }
                                                    }
                                                    current_screen = Screen::Debug;
                                                } else {
                                                    // --- PRODUCTION MODE: Fade out and launch ---
                                                    (current_screen, fade_start_time) = trigger_session_restart(&mut current_bgm, &music_cache);
                                                }
                                            },
                                            _ => { // multiple games found
                                                println!("[Debug] Found {} games. Switching to selection screen.", games.len());
                                                for (index, (_, path)) in games.iter().enumerate() {
                                                    println!("[Debug]   Game {}: {}", index + 1, path.display());
                                                }

                                                // --- FIX 2: Use the cart_info.icon field ---
                                                // Queue up the icons for loading.
                                                game_icon_queue.clear();
                                                for (cart_info, kzi_path) in &games {
                                                    let icon_path = kzi_path.parent().unwrap().join(&cart_info.icon);
                                                    game_icon_queue.push((cart_info.id.clone(), icon_path));
                                                }

                                                available_games = games;
                                                game_selection = 0;
                                                current_screen = Screen::GameSelection;
                                            }
                                        }
                                    },
                                    Err(e) => { // Handle the error case
                                        let error_msg = format!("[Error] Error scanning for cartridges: {}", e);
                                        println!("[Error] {}", &error_msg);
                                        log_messages.lock().unwrap().push(error_msg);
                                        current_screen = Screen::Debug;
                                    }
                                }
                            } else {
                                sound_effects.play_reject(&config);
                                animation_state.trigger_play_option_shake();
                            }
                        },
                        2 => {
                            current_screen = Screen::Settings;
                            sound_effects.play_select(&config);
                        },
                        _ => {}
                    }
                }
            },
            Screen::Settings | Screen::Settings2 | Screen::Settings3 => {
                // --- Determine current page info ---
                let (page_number, options): (usize, &[&str]) = match current_screen {
                    Screen::Settings => (1, &SETTINGS_MENU_OPTIONS),
                    Screen::Settings2 => (2, &SETTINGS_MENU_OPTIONS_PAGE_2),
                    Screen::Settings3 => (3, &SETTINGS_MENU_OPTIONS_PAGE_3),
                    _ => unreachable!(),
                };

                // --- Handle Common Input (Up/Down/Back) ---
                if input_state.up {
                    settings_menu_selection = if settings_menu_selection == 0 { options.len() - 1 } else { settings_menu_selection - 1 };
                    sound_effects.play_cursor_move(&config);
                }
                if input_state.down {
                    settings_menu_selection = (settings_menu_selection + 1) % options.len();
                    sound_effects.play_cursor_move(&config);
                }
                if input_state.back {
                    current_screen = Screen::MainMenu;
                    sound_effects.play_back(&config);
                }

                // page switching
                if input_state.next {
                    sound_effects.play_select(&config);
                    settings_menu_selection = 0; // Reset selection for the new page
                    match current_screen {
                        Screen::Settings => current_screen = Screen::Settings2,
                        Screen::Settings2 => current_screen = Screen::Settings3,
                        Screen::Settings3 => current_screen = Screen::Settings,
                        _ => {} // This case won't be reached
                    }
                }

                if input_state.prev {
                    sound_effects.play_select(&config);
                    settings_menu_selection = 0; // Reset selection for the new page
                    match current_screen {
                        Screen::Settings => current_screen = Screen::Settings3,
                        Screen::Settings2 => current_screen = Screen::Settings,
                        Screen::Settings3 => current_screen = Screen::Settings2,
                        _ => {} // This case won't be reached
                    }
                }

                // --- Render the Page ---
                render_settings_page(
                    page_number, options, &logo_cache, &background_cache, &font_cache,
                    &mut config, settings_menu_selection, &animation_state, &mut background_state,
                    &battery_info, &current_time_str, scale_factor, display_settings_changed, system_volume,
                );

                // --- Handle Page-Specific Actions ---
                match page_number {
                    // PAGE ONE
                    1 => match settings_menu_selection {
                        // SPLASH SCREEN
                        0 => {
                            if input_state.left || input_state.right {
                                config.show_splash_screen = !config.show_splash_screen;
                                save_config(&config);
                                sound_effects.play_cursor_move(&config);
                            }
                        },

                        // BGM selection
                        1 => {
                            if input_state.left || input_state.right {
                                // Find the current track's position in our list of choices
                                let current_index = bgm_choices.iter().position(|t| *t == config.bgm_track.clone().unwrap_or("OFF".to_string())).unwrap_or(0);
                                let mut new_index = current_index;

                                if input_state.left {
                                    new_index = if current_index == 0 { bgm_choices.len() - 1 } else { current_index - 1 };
                                }
                                if input_state.right {
                                    new_index = (current_index + 1) % bgm_choices.len();
                                }

                                let new_track = &bgm_choices[new_index];
                                play_new_bgm(new_track, config.bgm_volume, &music_cache, &mut current_bgm);

                                // Update the config with the new choice
                                if new_track == "OFF" {
                                    config.bgm_track = None;
                                } else {
                                    config.bgm_track = Some(new_track.clone());
                                }

                                save_config(&config);
                                sound_effects.play_cursor_move(&config);
                            }
                        },

                        // BGM volume
                        2 => {
                            if input_state.left || input_state.right {
                                if input_state.left {
                                    config.bgm_volume = (config.bgm_volume - 0.1).max(0.0);
                                }
                                if input_state.right {
                                    config.bgm_volume = (config.bgm_volume + 0.1).min(1.0);
                                }

                                // Change the volume of the currently playing sound
                                if let Some(sound) = &current_bgm {
                                    audio::set_sound_volume(sound, config.bgm_volume);
                                }

                                save_config(&config);
                                sound_effects.play_cursor_move(&config);
                            }
                        },

                        // LOGO selection
                        3 => {
                            if input_state.left || input_state.right {
                                // Find the current logo's position in our list of choices
                                let current_index = logo_choices.iter().position(|l| *l == config.logo_selection).unwrap_or(0);
                                let mut new_index = current_index;

                                if input_state.left {
                                    new_index = if current_index == 0 { logo_choices.len() - 1 } else { current_index - 1 };
                                }
                                if input_state.right {
                                    new_index = (current_index + 1) % logo_choices.len();
                                }

                                // Update the config with the new choice
                                config.logo_selection = logo_choices[new_index].clone();

                                save_config(&config);
                                sound_effects.play_cursor_move(&config);
                            }
                        },

                        // BACKGROUND selection
                        4 => {
                            if input_state.left || input_state.right {
                                // Find the current background's position in our list of choices
                                let current_index = background_choices.iter().position(|b| *b == config.background_selection).unwrap_or(0);
                                let mut new_index = current_index;

                                if input_state.left {
                                    new_index = if current_index == 0 { background_choices.len() - 1 } else { current_index - 1 };
                                }
                                if input_state.right {
                                    new_index = (current_index + 1) % background_choices.len();
                                }

                                // Update the config with the new choice
                                config.background_selection = background_choices[new_index].clone();

                                save_config(&config);
                                sound_effects.play_cursor_move(&config);
                            }
                        },
                        5 => { // GO TO PAGE 2
                            if input_state.select {
                                current_screen = Screen::Settings2;
                                settings_menu_selection = 0;
                                sound_effects.play_select(&config);
                            }
                        },
                        _ => {}
                    },

                    // PAGE TWO
                    2 => match settings_menu_selection {
                        // FONT COLOR
                        0 => {
                            if input_state.left || input_state.right {
                                // Find current color's index in our list
                                let current_index = FONT_COLORS.iter().position(|&c| c == config.font_color).unwrap_or(0);
                                let new_index = if input_state.right {
                                    (current_index + 1) % FONT_COLORS.len()
                                } else {
                                    (current_index + FONT_COLORS.len() - 1) % FONT_COLORS.len()
                                };
                                config.font_color = FONT_COLORS[new_index].to_string();
                                save_config(&config);
                                sound_effects.play_cursor_move(&config);
                            }
                        }

                        // FONT TYPE
                        1 => {
                            if input_state.left || input_state.right {
                                let current_index = font_choices.iter().position(|name| name == &config.font_selection).unwrap_or(0);
                                let new_index = if input_state.right {
                                    (current_index + 1) % font_choices.len()
                                } else {
                                    (current_index + font_choices.len() - 1) % font_choices.len()
                                };

                                config.font_selection = font_choices[new_index].clone();
                                save_config(&config);
                                sound_effects.play_cursor_move(&config);
                            }
                        }

                        // BACKGROUND SCROLLING
                        2 => {
                            if input_state.left || input_state.right {
                                let current_index = SCROLL_SPEEDS.iter().position(|&s| s == config.background_scroll_speed).unwrap_or(0);
                                let new_index = if input_state.right {
                                    (current_index + 1) % SCROLL_SPEEDS.len()
                                } else {
                                    (current_index + SCROLL_SPEEDS.len() - 1) % SCROLL_SPEEDS.len()
                                };

                                config.background_scroll_speed = SCROLL_SPEEDS[new_index].to_string();
                                save_config(&config);
                                sound_effects.play_cursor_move(&config);
                            }
                        }

                        // COLOR SHIFTING
                        3 => {
                            if input_state.left || input_state.right {
                                let current_index = COLOR_SHIFT_SPEEDS.iter().position(|&s| s == config.color_shift_speed).unwrap_or(0);
                                let new_index = if input_state.right {
                                    (current_index + 1) % COLOR_SHIFT_SPEEDS.len()
                                } else {
                                    (current_index + COLOR_SHIFT_SPEEDS.len() - 1) % COLOR_SHIFT_SPEEDS.len()
                                };

                                config.color_shift_speed = COLOR_SHIFT_SPEEDS[new_index].to_string();
                                save_config(&config);
                                sound_effects.play_cursor_move(&config);
                            }
                        }

                        // SOUND PACKS
                        4 => {
                            if input_state.left || input_state.right {
                                let sound_packs = find_sound_packs();
                                let current_index = sound_packs.iter().position(|name| name == &config.sfx_pack).unwrap_or(0);
                                let new_index = if input_state.right {
                                    (current_index + 1) % sound_packs.len()
                                } else {
                                    (current_index + sound_packs.len() - 1) % sound_packs.len()
                                };

                                config.sfx_pack = sound_packs[new_index].clone();
                                save_config(&config);

                                // Signal to the main loop that we need to reload this pack
                                sfx_pack_to_reload = Some(config.sfx_pack.clone());
                            }
                        }

                        5 => { // GO TO PAGE 1
                            if input_state.select {
                                current_screen = Screen::Settings;
                                settings_menu_selection = 0;
                                sound_effects.play_select(&config);
                            }
                        },

                        6 => { // GO TO PAGE 3
                            if input_state.select {
                                current_screen = Screen::Settings3;
                                settings_menu_selection = 0;
                                sound_effects.play_select(&config);
                            }
                        },
                        _ => {}
                    },

                    // PAGE THREE
                    3 => match settings_menu_selection {
                        // CURSOR COLOR
                        0 => {
                            if input_state.left || input_state.right {
                                // We can reuse the FONT_COLORS constant for this
                                let current_index = FONT_COLORS.iter().position(|&c| c == config.cursor_color).unwrap_or(0);
                                let new_index = if input_state.right {
                                    (current_index + 1) % FONT_COLORS.len()
                                } else {
                                    (current_index + FONT_COLORS.len() - 1) % FONT_COLORS.len()
                                };

                                config.cursor_color = FONT_COLORS[new_index].to_string();
                                save_config(&config);
                                sound_effects.play_cursor_move(&config);
                            }
                        },
                        1 => { // RESOLUTION
                            if input_state.left || input_state.right {
                                let current_index = RESOLUTIONS.iter().position(|&r| r == config.resolution).unwrap_or(0);
                                let new_index = if input_state.right {
                                    (current_index + 1) % RESOLUTIONS.len()
                                } else {
                                    (current_index + RESOLUTIONS.len() - 1) % RESOLUTIONS.len()
                                };

                                config.resolution = RESOLUTIONS[new_index].to_string();
                                save_config(&config);
                                apply_resolution(&config.resolution); // Apply the change immediately
                                sound_effects.play_cursor_move(&config);
                            }
                        },
                        2 => { // FULLSCREEN
                            if input_state.left || input_state.right {
                                config.fullscreen = !config.fullscreen;
                                //set_fullscreen(config.fullscreen); // Apply the change immediately
                                save_config(&config);
                                sound_effects.play_cursor_move(&config);
                                display_settings_changed = true;
                            }
                        },
                        3 => { // SFX Volume
                            if input_state.left || input_state.right {
                                if input_state.left {
                                    config.sfx_volume = (config.sfx_volume - 0.1).max(0.0);
                                }
                                if input_state.right {
                                    config.sfx_volume = (config.sfx_volume + 0.1).min(1.0);
                                }
                                save_config(&config);
                                sound_effects.play_cursor_move(&config); // Test the new volume
                            }
                        },
                        4 => { // MASTER VOLUME
                            if input_state.left {
                                adjust_system_volume("10%-"); // Decrease by 10%
                                system_volume = get_system_volume().unwrap_or(system_volume); // Refresh the value
                                sound_effects.play_cursor_move(&config);
                            }
                            if input_state.right {
                                adjust_system_volume("10%+"); // Increase by 10%
                                system_volume = get_system_volume().unwrap_or(system_volume); // Refresh the value
                                sound_effects.play_cursor_move(&config);
                            }
                        },
                        5 => { // GO TO PAGE 2
                            if input_state.select {
                                current_screen = Screen::Settings2;
                                settings_menu_selection = 0;
                                sound_effects.play_select(&config);
                            }
                        },
                        _ => {}
                    },
                    _ => {}
                }
            },
            Screen::GameSelection => {
                // --- Load Icons from Queue ---
                if !game_icon_queue.is_empty() {
                    let (game_id, icon_path) = game_icon_queue.remove(0);
                    if let Ok(texture) = load_texture(&icon_path.to_string_lossy()).await {
                        game_icon_cache.insert(game_id, texture);
                    }
                }
                let grid_width = 5; // The number of icons per row
                if input_state.left {
                    if game_selection > 0 {
                        game_selection -= 1;
                        sound_effects.play_cursor_move(&config);
                    }
                }
                if input_state.right {
                    if game_selection < available_games.len() - 1 {
                        game_selection += 1;
                        sound_effects.play_cursor_move(&config);
                    }
                }
                if input_state.up {
                    if game_selection >= grid_width {
                        game_selection -= grid_width;
                        sound_effects.play_cursor_move(&config);
                    }
                }
                if input_state.down {
                    if game_selection + grid_width < available_games.len() {
                        game_selection += grid_width;
                        sound_effects.play_cursor_move(&config);
                    }
                }
                if input_state.back {
                    current_screen = Screen::MainMenu;
                    sound_effects.play_back(&config);
                }
                if input_state.select {
                    if let Some((cart_info, kzi_path)) = available_games.get(game_selection) {
                        sound_effects.play_select(&config);

                        if DEBUG_GAME_LAUNCH {
                            // --- DEBUG MODE ---
                            log_messages.lock().unwrap().clear();
                            { // Scoped lock to add messages
                                let mut logs = log_messages.lock().unwrap();
                                logs.push("--- CARTRIDGE FOUND ---".to_string());
                                logs.push(format!("Name: {}", cart_info.name.as_deref().unwrap_or("N/A")));
                                logs.push(format!("ID: {}", cart_info.id));
                                logs.push(format!("Exec: {}", cart_info.exec));
                                logs.push(format!("Runtime: {}", cart_info.runtime.as_deref().unwrap_or("None")));
                                logs.push(format!("KZI Path: {}", kzi_path.display()));
                            }
                            println!("[Debug] Single Cartridge Found! Preparing to launch...");
                            println!("[Debug]   Name: {}", cart_info.name.as_deref().unwrap_or("N/A"));
                            println!("[Debug]   ID: {}", cart_info.id);
                            println!("[Debug]   Exec: {}", cart_info.exec);
                            println!("[Debug]   Runtime: {}", cart_info.runtime.as_deref().unwrap_or("None"));
                            println!("[Debug]   KZI Path: {}", kzi_path.display());

                            match save::launch_game(&cart_info, &kzi_path) {
                                Ok(mut child) => {
                                    log_messages.lock().unwrap().push("\n--- LAUNCHING GAME ---".to_string());
                                    start_log_reader(&mut child, log_messages.clone());
                                    game_process = Some(child);
                                }
                                Err(e) => {
                                    log_messages.lock().unwrap().push(format!("\n--- LAUNCH FAILED ---\nError: {}", e));
                                }
                            }
                            current_screen = Screen::Debug;

                            match save::launch_game(cart_info, kzi_path) {
                                Ok(mut child) => {
                                    start_log_reader(&mut child, log_messages.clone());
                                    game_process = Some(child);
                                }
                                Err(e) => {
                                    log_messages.lock().unwrap().push(format!("\n--- LAUNCH FAILED ---\nError: {}", e));
                                }
                            }
                            current_screen = Screen::Debug;
                        } else {
                            (current_screen, fade_start_time) = trigger_session_restart(&mut current_bgm, &music_cache);
                        }
                    }
                }

                // --- Render ---
                render_game_selection_menu(
                    &available_games, &game_icon_cache, &placeholder, game_selection, &animation_state, &logo_cache,
                    &background_cache, &font_cache, &config, &mut background_state,
                    &battery_info, &current_time_str, scale_factor
                );
            },
            Screen::Debug => {
                let messages = log_messages.lock().unwrap();

                // INPUT
                if input_state.up && debug_scroll_offset > 0 {
                    debug_scroll_offset -= 1;
                }
                // Allow scrolling down only if there are more messages than can be displayed
                if input_state.down && debug_scroll_offset < messages.len().saturating_sub(1) {
                    debug_scroll_offset += 1;
                }
                // save log file
                if input_state.select {
                    match save_log_to_file(&messages) {
                        Ok(filename) => {
                            // Add a confirmation message to the log
                            //messages.push(format!("\nLOG SAVED TO {}", filename));
                            flash_message = Some((format!("LOG SAVED TO {}", filename), FLASH_MESSAGE_DURATION));
                        }
                        Err(e) => {
                            //messages.push(format!("\nERROR SAVING LOG: {}", e));
                            flash_message = Some((format!("ERROR SAVING LOG: {}", e), FLASH_MESSAGE_DURATION));
                        }
                    }
                }
                if input_state.back {
                    // If the user presses back, kill the game process and return to the menu
                    if let Some(mut child) = game_process.take() {
                        child.kill().ok(); // Ignore error if process already exited
                    }
                    current_screen = Screen::MainMenu;
                    sound_effects.play_back(&config);
                    debug_scroll_offset = 0;
                }

                // --- Update flash message timer ---
                if let Some((_, timer)) = &mut flash_message {
                    *timer -= get_frame_time();
                    if *timer <= 0.0 {
                        flash_message = None;
                    }
                }

                // RENDER
                // Lock the mutex to get read-only access to the log messages for this frame
                render_debug_screen(
                    &messages,
                    debug_scroll_offset,
                    flash_message.as_ref().map(|(msg, _)| msg.as_str()), // Pass the message text
                    &font_cache,
                    &config,
                    scale_factor,
                    &background_cache,
                    &mut background_state,
                );
            },
            Screen::SaveData => {
                render_background(&background_cache, &config, &mut background_state);

                // Check if memories need to be refreshed due to storage media changes
                if let Ok(mut state) = storage_state.lock() {
                    if state.needs_memory_refresh {
                        if !state.media.is_empty() {
                            memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                        } else {
                            memories = Vec::new();
                        }
                        state.needs_memory_refresh = false;
                        dialogs.clear();
                    }
                }
                match dialog_state {
                    DialogState::None => {
                        render_data_view(selected_memory, &memories, &icon_cache, &font_cache, &config, &storage_state, &placeholder, scroll_offset, &mut input_state, &mut animation_state, &mut playtime_cache, &mut size_cache, scale_factor);

                        // Handle back navigation
                        if input_state.back {
                           current_screen = Screen::MainMenu;
                           sound_effects.play_back(&config);
                        }

                        // Handle storage media switching with tab/bumpers regardless of focus
                        if input_state.cycle || input_state.next || input_state.prev {
                            if let Ok(mut state) = storage_state.lock() {
                                if input_state.cycle {
                                    if state.media.len() > 1 {
                                        // Cycle wraps around
                                        state.selected = (state.selected + 1) % state.media.len();
                                        memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                                        scroll_offset = 0;
                                        sound_effects.play_select(&config);
                                    }
                                } else if input_state.next {
                                    // Next stops at end
                                    if state.selected < state.media.len() - 1 {
                                        state.selected += 1;
                                        memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                                        scroll_offset = 0;
                                        sound_effects.play_select(&config);
                                    } else {
                                        animation_state.trigger_shake(false); // Shake right arrow when can't go next
                                        sound_effects.play_reject(&config);
                                    }
                                } else if input_state.prev {
                                    // Prev stops at beginning
                                    if state.selected > 0 {
                                        state.selected -= 1;
                                        memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                                        scroll_offset = 0;
                                        sound_effects.play_select(&config);
                                    } else {
                                        animation_state.trigger_shake(true); // Shake left arrow when can't go prev
                                        sound_effects.play_reject(&config);
                                    }
                                }
                            }
                        }

                        match input_state.ui_focus {
                            UIFocus::Grid => {
                                if input_state.select {
                                    let memory_index = get_memory_index(selected_memory, scroll_offset);
                                    if let Some(_) = memories.get(memory_index) {
                                        let (grid_pos, dialog_pos) = calculate_icon_transition_positions(selected_memory, scale_factor);
                                        animation_state.trigger_dialog_transition(grid_pos, dialog_pos);
                                        dialogs.push(create_main_dialog(&storage_state));
                                        dialog_state = DialogState::Opening;
                                        sound_effects.play_select(&config);
                                    }
                                }
                                if input_state.right && selected_memory < GRID_WIDTH * GRID_HEIGHT - 1 {
                                    selected_memory += 1;
                                    animation_state.trigger_transition();
                                    sound_effects.play_cursor_move(&config);
                                }
                                if input_state.left && selected_memory >= 1 {
                                    selected_memory -= 1;
                                    animation_state.trigger_transition();
                                    sound_effects.play_cursor_move(&config);
                                }
                                if input_state.down {
                                    if selected_memory < GRID_WIDTH * GRID_HEIGHT - GRID_WIDTH {
                                        selected_memory += GRID_WIDTH;
                                        animation_state.trigger_transition();
                                        sound_effects.play_cursor_move(&config);
                                    } else {
                                        // Check if there are any saves in the next row
                                        let next_row_start = get_memory_index(GRID_WIDTH * GRID_HEIGHT, scroll_offset);
                                        if next_row_start < memories.len() {
                                            scroll_offset += 1;
                                            animation_state.trigger_transition();
                                            sound_effects.play_cursor_move(&config);
                                        }
                                    }
                                }
                                if input_state.up {
                                    if selected_memory >= GRID_WIDTH {
                                        selected_memory -= GRID_WIDTH;
                                        animation_state.trigger_transition();
                                        sound_effects.play_cursor_move(&config);
                                    } else if scroll_offset > 0 {
                                        scroll_offset -= 1;
                                        animation_state.trigger_transition();
                                        sound_effects.play_cursor_move(&config);
                                    } else {
                                        // Allow moving to storage navigation from leftmost or rightmost column
                                        if selected_memory % GRID_WIDTH == 0 {
                                            input_state.ui_focus = UIFocus::StorageLeft;
                                            animation_state.trigger_transition();
                                            sound_effects.play_cursor_move(&config);
                                        } else if selected_memory % GRID_WIDTH == GRID_WIDTH - 1 {
                                            input_state.ui_focus = UIFocus::StorageRight;
                                            animation_state.trigger_transition();
                                            sound_effects.play_cursor_move(&config);
                                        }
                                    }
                                }
                            },
                            UIFocus::StorageLeft => {
                                if input_state.right {
                                    input_state.ui_focus = UIFocus::StorageRight;
                                    animation_state.trigger_transition();
                                    sound_effects.play_cursor_move(&config);
                                }
                                if input_state.down {
                                    input_state.ui_focus = UIFocus::Grid;
                                    selected_memory = 0; // Move to leftmost grid position
                                    animation_state.trigger_transition();
                                    sound_effects.play_cursor_move(&config);
                                }
                                if input_state.select {
                                    if let Ok(mut state) = storage_state.lock() {
                                        if state.selected > 0 {
                                            state.selected -= 1;
                                            memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                                            scroll_offset = 0;
                                            sound_effects.play_select(&config);
                                        } else {
                                            animation_state.trigger_shake(true);
                                            sound_effects.play_reject(&config);
                                        }
                                    }
                                }
                            },
                            UIFocus::StorageRight => {
                                if input_state.left {
                                    input_state.ui_focus = UIFocus::StorageLeft;
                                    animation_state.trigger_transition();
                                    sound_effects.play_cursor_move(&config);
                                }
                                if input_state.down {
                                    input_state.ui_focus = UIFocus::Grid;
                                    selected_memory = GRID_WIDTH - 1; // Move to rightmost grid position
                                    animation_state.trigger_transition();
                                    sound_effects.play_cursor_move(&config);
                                }
                                if input_state.select {
                                    if let Ok(mut state) = storage_state.lock() {
                                        if state.selected < state.media.len() - 1 {
                                            state.selected += 1;
                                            memories = load_memories(&state.media[state.selected], &mut icon_cache, &mut icon_queue).await;
                                            scroll_offset = 0;
                                            sound_effects.play_select(&config);
                                        } else {
                                            animation_state.trigger_shake(false);
                                            sound_effects.play_reject(&config);
                                        }
                                    }
                                }
                            },
                        }
                    },
                    DialogState::Opening => {
                        // During opening, only render the main view and the transitioning icon
                        render_data_view(selected_memory, &memories, &icon_cache, &font_cache, &config, &storage_state, &placeholder, scroll_offset, &mut input_state, &mut animation_state, &mut playtime_cache, &mut size_cache, scale_factor);
                        // Only render the icon during transition
                        let memory_index = get_memory_index(selected_memory, scroll_offset);
                        if let Some(mem) = memories.get(memory_index) {
                            let icon = match icon_cache.get(&mem.id) {
                                Some(icon) => icon,
                                None => &placeholder,
                            };

                            let params = DrawTextureParams {
                                dest_size: Some(Vec2 {x: TILE_SIZE, y: TILE_SIZE }),
                                source: Some(Rect { x: 0.0, y: 0.0, h: icon.height(), w: icon.width() }),
                                rotation: 0.0,
                                flip_x: false,
                                flip_y: false,
                                pivot: None
                            };

                            let icon_pos = animation_state.get_dialog_transition_pos();
                            draw_texture_ex(&icon, icon_pos.x, icon_pos.y, WHITE, params);
                        }
                    },
                    DialogState::Open => {
                        // When dialog is fully open, only render the dialog
                        if let Some(dialog) = dialogs.last_mut() {
                            render_dialog(dialog, &memories, selected_memory, &icon_cache, &font_cache, &config, &copy_op_state, &placeholder, scroll_offset, &animation_state, &mut playtime_cache, &mut size_cache, scale_factor);

                            let mut selection: i32 = dialog.selection as i32 + dialog.options.len() as i32;
                            if input_state.up {
                                selection -= 1;
                                animation_state.trigger_transition();
                                sound_effects.play_cursor_move(&config);
                            }

                            if input_state.down {
                                selection += 1;
                                animation_state.trigger_transition();
                                sound_effects.play_cursor_move(&config);
                            }

                            let mut cancel = false;
                            if input_state.back {
                                cancel = true;
                            }

                            let next_selection = selection as usize % dialog.options.len();
                            if next_selection != dialog.selection {
                                // Store the new selection to apply after we're done with the immutable borrow
                                let new_selection = next_selection;
                                dialog.selection = new_selection;
                            } else {
                                // We need to handle the select input
                                if input_state.select {
                                    let selected_option = &dialog.options[dialog.selection];
                                    if !selected_option.disabled {
                                        action_dialog_id = dialog.id.clone();
                                        action_option_value = selected_option.value.clone();

                                        if selected_option.value == "CANCEL" || selected_option.value == "OK" {
                                            cancel = true;
                                        } else {
                                            sound_effects.play_select(&config);
                                        }
                                    } else {
                                        animation_state.trigger_dialog_shake();
                                        sound_effects.play_reject(&config);
                                    }
                                }
                            }

                            if cancel {
                                let (grid_pos, dialog_pos) = calculate_icon_transition_positions(selected_memory, scale_factor);
                                animation_state.trigger_dialog_transition(dialog_pos, grid_pos);
                                dialog_state = DialogState::Closing;
                                sound_effects.play_back(&config);
                            }
                        }
                    },
                    DialogState::Closing => {
                        // During closing, render both views to show the icon returning
                        render_data_view(selected_memory, &memories, &icon_cache, &font_cache, &config, &storage_state, &placeholder, scroll_offset, &mut input_state, &mut animation_state, &mut playtime_cache, &mut size_cache, scale_factor);
                        // Only render the icon during transition
                        let memory_index = get_memory_index(selected_memory, scroll_offset);
                        if let Some(mem) = memories.get(memory_index) {
                            let icon = match icon_cache.get(&mem.id) {
                                Some(icon) => icon,
                                None => &placeholder,
                            };

                            let params = DrawTextureParams {
                                dest_size: Some(Vec2 {x: TILE_SIZE, y: TILE_SIZE }),
                                source: Some(Rect { x: 0.0, y: 0.0, h: icon.height(), w: icon.width() }),
                                rotation: 0.0,
                                flip_x: false,
                                flip_y: false,
                                pivot: None
                            };

                            let icon_pos = animation_state.get_dialog_transition_pos();
                            draw_texture_ex(&icon, icon_pos.x, icon_pos.y, WHITE, params);
                        }
                    }
                }
            },
        }

        // It checks if a reload was requested by the settings screen
        if let Some(pack_name) = sfx_pack_to_reload.take() {
            println!("[Info] Reloading SFX pack: {}", pack_name);
            sound_effects = SoundEffects::load(&pack_name).await;
            // Play a sound from the new pack to confirm it changed
            sound_effects.play_cursor_move(&config);
        }

        // Handle dialog actions
        match (action_dialog_id.as_str(), action_option_value.as_str()) {
            ("main", "COPY") => {
                dialogs.push(create_copy_storage_dialog(&storage_state));
            },
            ("main", "DELETE") => {
                dialogs.push(create_confirm_delete_dialog());
            },
            ("main", "CANCEL") => {
                let (grid_pos, dialog_pos) = calculate_icon_transition_positions(selected_memory, scale_factor);
                animation_state.trigger_dialog_transition(dialog_pos, grid_pos);
                dialog_state = DialogState::Closing;
                sound_effects.play_back(&config);
            },
            ("confirm_delete", "DELETE") => {
                if let Ok(mut state) = storage_state.lock() {
                    let memory_index = get_memory_index(selected_memory, scroll_offset);
                    if let Some(mem) = memories.get(memory_index) {
                        if let Err(e) = save::delete_save(&mem.id, &state.media[state.selected].id) {
                            dialogs.push(create_error_dialog(format!("ERROR: {}", e)));
                        } else {
                            state.needs_memory_refresh = true;
                            dialog_state = DialogState::None;
                            sound_effects.play_back(&config);
                        }
                    }
                }
            },
            ("confirm_delete", "CANCEL") => {
                let (grid_pos, dialog_pos) = calculate_icon_transition_positions(selected_memory, scale_factor);
                animation_state.trigger_dialog_transition(dialog_pos, grid_pos);
                dialog_state = DialogState::Closing;
                sound_effects.play_back(&config);
            },
            ("copy_storage_select", target_id) if target_id != "CANCEL" => {
                let memory_index = get_memory_index(selected_memory, scroll_offset);
                let mem = memories[memory_index].clone();
                let target_id = target_id.to_string();
                if let Ok(state) = storage_state.lock() {
                    let to_media = StorageMedia { id: target_id, free: 0 };

                    // Check if save already exists
                    if check_save_exists(&mem, &to_media, &mut icon_cache, &mut icon_queue).await {
                        dialogs.push(create_save_exists_dialog());
                    } else {
                        let thread_state = copy_op_state.clone();
                        let from_media = state.media[state.selected].clone();
                        thread::spawn(move || {
                            copy_memory(&mem, &from_media, &to_media, thread_state);
                        });
                    }
                }
            },
            ("copy_storage_select", "CANCEL") => {
                let (grid_pos, dialog_pos) = calculate_icon_transition_positions(selected_memory, scale_factor);
                animation_state.trigger_dialog_transition(dialog_pos, grid_pos);
                dialog_state = DialogState::Closing;
                sound_effects.play_back(&config);
            },
            ("save_exists", "OK") => {
                let (grid_pos, dialog_pos) = calculate_icon_transition_positions(selected_memory, scale_factor);
                animation_state.trigger_dialog_transition(dialog_pos, grid_pos);
                dialog_state = DialogState::Closing;
                sound_effects.play_back(&config);
            },
            ("error", "OK") => {
                let (grid_pos, dialog_pos) = calculate_icon_transition_positions(selected_memory, scale_factor);
                animation_state.trigger_dialog_transition(dialog_pos, grid_pos);
                dialog_state = DialogState::Closing;
                sound_effects.play_back(&config);
            },
            _ => {}
        }

        if !icon_queue.is_empty() {
            let (cart_id, icon_path) = icon_queue.remove(0);
            let texture_future = load_texture(&icon_path);
            let texture_result = panic::catch_unwind(|| {
                futures::executor::block_on(texture_future)
            });

            if let Ok(Ok(texture)) = texture_result {
                icon_cache.insert(cart_id.clone(), texture);
            }
        }

        // Display any copy operation errors
        if let Ok(mut copy_state) = copy_op_state.lock() {
            if let Some(error_msg) = copy_state.error_message.take() {
                dialogs.push(create_error_dialog(error_msg));
                dialog_state = DialogState::Opening;
            }
            if copy_state.should_clear_dialogs {
                dialog_state = DialogState::Closing;
                copy_state.should_clear_dialogs = false;
            }
        }
        next_frame().await
    }
}
