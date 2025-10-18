use macroquad::prelude::*;
use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, PlaySoundParams, Sound};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
use std::collections::{HashMap, HashSet};
use gilrs::Gilrs;
use std::sync::atomic::{Ordering, AtomicBool};
use std::fs;
use std::process;
use std::process::Child;

// extra stuff I'm using
use std::path::PathBuf; // for loading assets
use std::io::BufReader; // logger
use std::env; // backtracing
use ::rand::Rng; // for selecting a random message on startup
use chrono::Local; // for getting clock
use regex::Regex; // fetching audio sinks

// Import our new modules
mod audio;
mod config;
mod input;
mod memory;
mod save;
mod system;
mod theme;
mod types;
mod ui;
mod utils;

use crate::audio::{SoundEffects, play_new_bgm};
use crate::config::{Config, get_user_data_dir};
use crate::dialog::Dialog;
use crate::input::InputState;
use crate::system::*; // Wildcard to get all system functions
use crate::ui::main_menu::MAIN_MENU_OPTIONS;
use crate::ui::theme_downloader::ThemeDownloaderState;
use crate::ui::update_checker::UpdateCheckerState;
use crate::ui::wifi::WifiState;
use crate::ui::*;
use crate::utils::*; // Wildcard to get all utility functions
use crate::save::StorageMediaState;
use crate::settings::GENERAL_SETTINGS;
use crate::settings::render_settings_page;

pub use types::*;

/*
// ===================================
// TO-DO LIST
// ===================================
- gamepad tester
- add system debugger in the event the game crashed
- fix D-pad reversal with some games (Godot-based games in particular)
- per-game keyboard to gamepad mapping
- make the multi-cart selector UI similar to that of the SM3D All Stars Deluxe
- use OSK in-game
- Bluetooth controller support -- IN PROGRESS
- add option to safely unmount cart in main menu -- IN PROGRESS

Hard
- DVD functionality?
- MP4 support for background videos?

Unnecessary but cool
- GCC overclocking support?
- Slippi/Project+ support

// ===================================
// NOTES
// ===================================
- setting brightness needs the brightnessctl package -- this has been added to the manifest
- Steam Deck volume/brightness controls requires the keyd package -- this has been added to the manifest
- support for multiple audio sinks requires us to replace the wireplumber file in /var/kazeta/state/ to .AUDIO_PREFERENCE_SET, as specified in the kazeta-session script
- multi-cart support requires us to have a LAUNCH_CMD_FILE, as specified in kazeta-session, and we also have to check if a specific .kzi file was passed as an argument in "kazeta"
- we add a "steam-deck.yaml" device profile for InputPlumber in /usr/share/inputplumber/profiles/ and map two of the back buttons to F13 and F14 so keyd can recognize them as keyboard inputs. These then get loaded into /etc/keyd/default.conf and control the brightness level
*/

// ===================================
// CONSTANTS
// ===================================

const DEBUG_GAME_LAUNCH: bool = false;
const DEV_MODE: bool = false;

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

const WINDOW_TITLE: &str = "Kazeta+ BIOS";
const VERSION_NUMBER: &str = "V1.3.KAZETA+";

const MENU_OPTION_HEIGHT: f32 = 30.0;
const MENU_PADDING: f32 = 8.0;
const RECT_COLOR: Color = Color::new(0.15, 0.15, 0.15, 1.0);

const FLASH_MESSAGE_DURATION: f32 = 5.0; // Show message for 5 seconds

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
    "KAZETA IS CZECH FOR 'CASSETTE'.",
    "BLOWING DUST OFF THE CARTRIDGE...",
    "RUNNING `SUDO PACMAN -SYU`...\nJUST KIDDING ;-).",
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
    ($files:expr, $type_name:expr, $loader:ident, $cache:expr,
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
                        match load_sound_from_bytes(&bytes).await {
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

fn find_all_asset_files() -> (Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>) {
    println!("[INFO] Scanning for all asset files...");

    // 1. Create empty sets for each asset type
    let mut background_files_set = HashSet::new();
    let mut logo_files_set = HashSet::new();
    let mut font_files_set = HashSet::new();
    let mut music_files_set = HashSet::new();

    // 2. Gather system/default assets and add them to the sets
    background_files_set.extend(utils::find_asset_files("../backgrounds", &["png"]));
    logo_files_set.extend(utils::find_asset_files("../logos", &["png"]));
    font_files_set.extend(utils::find_asset_files("../fonts", &["ttf"]));
    music_files_set.extend(utils::find_asset_files("../music", &["ogg", "wav"]));

    // 3. Gather user-installed and theme assets
    if let Some(user_dir) = get_user_data_dir() {
        // Add assets from global user folders first
        background_files_set.extend(utils::find_asset_files(&user_dir.join("backgrounds").to_string_lossy(), &["png"]));
        logo_files_set.extend(utils::find_asset_files(&user_dir.join("logos").to_string_lossy(), &["png"]));
        font_files_set.extend(utils::find_asset_files(&user_dir.join("fonts").to_string_lossy(), &["ttf"]));
        music_files_set.extend(utils::find_asset_files(&user_dir.join("bgm").to_string_lossy(), &["ogg", "wav"]));

        // --- REVISED LOGIC for scanning theme folders ---
        let theme_dir = user_dir.join("themes");
        if let Ok(entries) = std::fs::read_dir(theme_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let theme_path = entry.path();

                    // Find all assets within this theme folder just ONCE
                    let theme_images = utils::find_asset_files(&theme_path.to_string_lossy(), &["png"]);
                    let theme_fonts = utils::find_asset_files(&theme_path.to_string_lossy(), &["ttf"]);
                    let theme_music = utils::find_asset_files(&theme_path.to_string_lossy(), &["wav", "ogg"]);

                    // Now, intelligently sort the images into the correct sets based on filename
                    for image_path in theme_images {
                        if let Some(filename) = image_path.file_name().and_then(|s| s.to_str()) {
                            if filename.ends_with("_logo.png") {
                                logo_files_set.insert(image_path);
                            } else if filename.ends_with("_background.png") {
                                background_files_set.insert(image_path);
                            }
                        }
                    }

                    // Add the fonts and music from the theme to their respective sets
                    font_files_set.extend(theme_fonts);
                    music_files_set.extend(theme_music);
                }
            }
        }
    }

    // 4. Convert the unique sets back into vectors for the loader
    let background_files: Vec<_> = background_files_set.into_iter().collect();
    let logo_files: Vec<_> = logo_files_set.into_iter().collect();
    let font_files: Vec<_> = font_files_set.into_iter().collect();
    let music_files: Vec<_> = music_files_set.into_iter().collect();

    // Return all the lists as a tuple
    (background_files, logo_files, font_files, music_files)
}

// ===================================
// ASYNC FUNCTIONS
// ===================================

async fn load_all_assets(
    config: &Config,
    display_message: &str,
    font: &Font,
    background_files: &[PathBuf],
    logo_files: &[PathBuf],
    font_files: &[PathBuf],
    music_files: &[PathBuf],
) -> (
    HashMap<String, Texture2D>, // background cache
    HashMap<String, Texture2D>, // logo cache
    HashMap<String, Sound>, // music cache
    HashMap<String, Font>, // font cache
    SoundEffects, // sfx
) {
    let draw_loading_screen = |status_message: &str, progress: f32| {
        let font_size = 16.0 as u16;
        let line_spacing = 10.0;
        let lines: Vec<&str> = display_message.lines().collect();

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
    logo_cache.insert("Kazeta+ (Default)".to_string(), default_logo);
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
    assets_loaded += 1;
    animate_step!(&mut display_progress, &mut assets_loaded, total_asset_count, animation_speed, &status, &draw_loading_screen);

    assets_loaded += 1;
    animate_step!(&mut display_progress, &mut assets_loaded, total_asset_count, animation_speed, &status, &draw_loading_screen);

    assets_loaded += 1;
    animate_step!(&mut display_progress, &mut assets_loaded, total_asset_count, animation_speed, &status, &draw_loading_screen);

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

    let sound_effects = audio::SoundEffects::load(&config.sfx_pack).await;

    (background_cache, logo_cache, music_cache, font_cache, sound_effects)
}

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

    // SYSTEM INFO
    let system_info = get_system_info();
    println!("[Debug] System Info Loaded: {:#?}", system_info); // Optional: for debugging

    // WI-FI
    //let mut wifi_state = WifiState::new().expect("Wi-Fi initialization failed. Ensure wlan0 is available.");
    let mut wifi_state = WifiState::new();

    // THEME DOWNLOADER
    let mut theme_downloader_state = ThemeDownloaderState::new();

    // BLUETOOTH CONTROLLER PAIRING
    let mut bluetooth_state = ui::bluetooth::BluetoothState::new();

    // UPDATE CHECKER
    let mut update_checker_state = UpdateCheckerState::new();

    // RESET SETTINGS CONFIRMATION
    let mut confirm_selection = 0; // 0 for YES, 1 for NO

    // MASTER VOLUME
    let mut system_volume = get_system_volume().unwrap_or(0.7); // Get initial volume, or default to 0.7

    // BRIGHTNESS
    let mut brightness = get_current_brightness().unwrap_or(0.5);

    // LOG MESSAGES
    let log_messages = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut game_process: Option<Child> = None;
    let mut debug_scroll_offset: usize = 0;

    // CLOCK
    let mut current_time_str = Local::now().format("%-I:%M %p").to_string();
    let mut last_time_check = get_time();
    const TIME_CHECK_INTERVAL: f64 = 1.0; // Check every second

    // BATTERY
    let mut battery_info: Option<BatteryInfo> = get_battery_info();
    let mut last_battery_check = get_time();
    const BATTERY_CHECK_INTERVAL: f64 = 5.0; // only check every 5 seconds to improve performance

    // load config file
    let mut config = Config::load();

    // AUDIO SINKS
    let available_sinks = get_available_sinks();
    println!("[Debug] Sinks loaded at startup: {:#?}", available_sinks); // <-- ADD THIS
    //let mut config: Config = load_config(); // Or your existing config loading

    // If the saved sink isn't available, reset to "Auto"
    if !available_sinks.iter().any(|s| s.name == config.audio_output) {
        config.audio_output = "Auto".to_string();
    }

    // FLASH MESSENGER
    let mut flash_message: Option<(String, f32)> = None; // (Message, time_remaining)

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

    // Load all themes ONCE at the start
    println!("[INFO] Pre-loading all themes...");
    let mut loaded_themes: HashMap<String, theme::Theme> = theme::load_all_themes().await;
    println!("[INFO] {} themes loaded successfully.", loaded_themes.len());

    let sound_pack_choices = audio::find_sound_packs();

    // find all asset files
    let (background_files, logo_files, font_files, music_files) = find_all_asset_files();

    // load them
    let (mut background_cache, mut logo_cache, mut music_cache, mut font_cache, mut sound_effects) =
    load_all_assets(
        &config,
        loading_text,
        &startup_font,
        &background_files,
        &logo_files,
        &font_files,
        &music_files
    ).await;

    // --- SET THE ACTIVE THEME ---
    let active_theme = loaded_themes.get(&config.theme).unwrap_or_else(|| {
        println!("[WARN] Active theme '{}' not found. Falling back to 'Default'.", &config.theme);
        loaded_themes.get("Default").expect("Default fallback theme is also missing!")
    });

    println!("[INFO] Using theme: {}", active_theme.name);

    // apply custom resolution if user specified it
    apply_resolution(&config.resolution);
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
    .filter(|k| *k != "Kazeta+ (Default)" && k.ends_with("_logo.png")) // Add this filter
    .cloned()
    .collect();
    custom_logos.sort(); // Sort just the custom logos alphabetically

    // 2. Create the final list with our specific order
    let mut logo_choices: Vec<String> = vec!["None".to_string(), "Kazeta+ (Default)".to_string()];
    logo_choices.extend(custom_logos);
    // The final list will be: ["None", "Kazeta (Default)", "cardforce.png", ...]

    // background state
    let mut background_state = BackgroundState {
        bgx: 0.0,
        bg_color: COLOR_TARGETS[0].clone(),
        target: 1,
        tg_color: COLOR_TARGETS[1].clone(),
    };

    // backgrounds
    let mut background_choices: Vec<String> = background_cache.keys()
    .filter(|k| k.ends_with("_background.png") || *k == "Default") // Add this filter
    .cloned()
    .collect();
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

    let mut current_bgm: Option<Sound> = None;

    // At the end of your setup, start the BGM based on the config
    if let Some(track_name) = &config.bgm_track {
        play_new_bgm(track_name, config.bgm_volume, &music_cache, &mut current_bgm);
    }

    // SPLASH SCREEN
    if config.show_splash_screen {
        // Mute the main BGM if it's already playing
        if let Some(sound) = &current_bgm {
            set_sound_volume(sound, 0.0);
        }

        // --- Load only what the splash screen needs ---
        let splash_logo = Texture2D::from_file_with_format(include_bytes!("../logo.png"), Some(ImageFormat::Png));
        let splash_sfx = load_sound_from_bytes(include_bytes!("../splash.wav")).await.unwrap();

        // Play the splash sound
        play_sound(&splash_sfx, PlaySoundParams { looped: false, volume: 0.2 });

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
            set_sound_volume(sound, config.bgm_volume);
        }
    }

    // Initialize gamepad support
    let mut gilrs = Gilrs::new().unwrap();
    let mut input_state = InputState::new();
    let mut animation_state = AnimationState::new();

    // Screen state
    let mut current_screen = Screen::MainMenu;
    let mut main_menu_selection: usize = 0;
    let mut settings_menu_selection: usize = 0;
    let mut extras_menu_selection: usize = 0;
    let mut game_selection: usize = 0; // For the new menu
    let mut available_games: Vec<(save::CartInfo, PathBuf)> = Vec::new(); // To hold the list of found games
    let mut play_option_enabled: bool = false;
    let mut copy_logs_option_enabled = false; // new button to copy session logs over to SD card

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

    // unmount cart
    let unmount_requested = Arc::new(AtomicBool::new(false));

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

    // BEGINNING OF MAIN LOOP
    loop {
        let scale_factor = screen_height() / BASE_SCREEN_HEIGHT;

        // FLASH TIMER
        if let Some((_message, timer)) = &mut flash_message {
            *timer -= get_frame_time(); // Decrease timer by the time elapsed since last frame
            if *timer <= 0.0 {
                flash_message = None; // Clear the message when timer runs out
            }
        }

        // CLOCK
        if get_time() - last_time_check > TIME_CHECK_INTERVAL {
            // Just call the new function to get the correct, formatted time string
            current_time_str = get_current_local_time_string(&config);
            last_time_check = get_time();
        }

        // BATTERY
        if get_time() - last_battery_check > BATTERY_CHECK_INTERVAL {
            battery_info = get_battery_info();
            last_battery_check = get_time();
        }

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
            Screen::About => {
                // Tell the about module to handle its own logic
                ui::about::update(&input_state, &mut current_screen, &sound_effects, &config);

                // Tell the about module to draw itself
                ui::about::draw(
                    &system_info,
                    &logo_cache,
                    &background_cache,
                    &font_cache,
                    &config,
                    &mut background_state,
                    &battery_info,
                    &current_time_str,
                    scale_factor,
                );
            }
            Screen::FadingOut => {
                // During fade, only render, don't process input
                // Render the current background and UI elements first
                ui::main_menu::update(
                    &mut current_screen,
                    &mut main_menu_selection,
                    &mut play_option_enabled,
                    &mut copy_logs_option_enabled,
                    &cart_connected,
                    &unmount_requested,
                    &mut input_state,
                    &mut animation_state,
                    &sound_effects,
                    &config,
                    &log_messages,
                    &storage_state,
                    &mut fade_start_time,
                    &mut current_bgm,
                    &music_cache,
                    &mut game_icon_queue,
                    &mut available_games,
                    &mut game_selection,
                    &mut flash_message,
                    &mut game_process,
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
                ui::main_menu::update(
                    &mut current_screen,
                    &mut main_menu_selection,
                    &mut play_option_enabled,
                    &mut copy_logs_option_enabled,
                    &cart_connected,
                    &unmount_requested,
                    &mut input_state,
                    &mut animation_state,
                    &sound_effects,
                    &config,
                    &log_messages,
                    &storage_state,
                    &mut fade_start_time,
                    &mut current_bgm,
                    &music_cache,
                    &mut game_icon_queue,
                    &mut available_games,
                    &mut game_selection,
                    &mut flash_message,
                    &mut game_process,
                );

                ui::main_menu::draw(
                    &MAIN_MENU_OPTIONS,
                    main_menu_selection,
                    play_option_enabled,
                    copy_logs_option_enabled,
                    &animation_state,
                    &logo_cache,
                    &background_cache,
                    &font_cache,
                    &config,
                    &mut background_state,
                    &battery_info,
                    &current_time_str,
                    scale_factor,
                    flash_message.as_ref().map(|(msg, _)| msg.as_str()),
                );
            },
            Screen::GeneralSettings | Screen::AudioSettings | Screen::GuiSettings | Screen::AssetSettings => {
                // --- Determine what to draw BEFORE updating state ---
                let (page_number, options) = match current_screen {
                    Screen::GeneralSettings => (1, ui::settings::GENERAL_SETTINGS),
                    Screen::AudioSettings => (2, ui::settings::AUDIO_SETTINGS),
                    Screen::GuiSettings => (3, ui::settings::GUI_CUSTOMIZATION_SETTINGS),
                    Screen::AssetSettings => (4, ui::settings::CUSTOM_ASSET_SETTINGS),
                    _ => (0, &[] as &[&str]),
                };

                // --- Handle input and state changes ---
                ui::settings::update(
                    &mut current_screen, &input_state, &mut config, &sound_pack_choices, &loaded_themes, &mut settings_menu_selection,
                    &mut sound_effects, &mut confirm_selection,
                    &mut brightness, &mut system_volume, &available_sinks, &mut current_bgm,
                    &bgm_choices, &music_cache, &mut sfx_pack_to_reload, &logo_choices,
                    &background_choices, &font_choices,
                );

                // --- Draw the UI ---
                if page_number > 0 {
                    ui::settings::render_settings_page(
                        page_number, options, &logo_cache, &background_cache, &font_cache,
                        &mut config, settings_menu_selection, &animation_state, &mut background_state,
                        &battery_info, &current_time_str, scale_factor, system_volume, brightness,
                    );
                }
            },
            Screen::Extras => {
                ui::extras_menu::update(
                    &mut current_screen,
                    &mut extras_menu_selection,
                    &input_state,
                    &mut animation_state,
                    &sound_effects,
                    &config,
                );

                ui::extras_menu::draw(
                    extras_menu_selection,
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
            }
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
                            // uncomment the line below when you are ready to debug
                            //current_screen = Screen::Debug;

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
                            // Instead of just restarting, we now trigger a specific game launch.
                            (current_screen, fade_start_time) = trigger_game_launch(
                                cart_info,
                                kzi_path,
                                &mut current_bgm,
                                &music_cache
                            );
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
                // Stop the BGM
                play_new_bgm("OFF", 0.0, &music_cache, &mut current_bgm);

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
            Screen::ConfirmReset => {
                // --- Input Handling ---
                if input_state.left || input_state.right {
                    confirm_selection = 1 - confirm_selection; // Flips between 0 and 1
                    sound_effects.play_cursor_move(&config);
                }
                if input_state.back {
                    current_screen = Screen::GeneralSettings; // Or whatever page you came from
                    sound_effects.play_back(&config);
                }
                if input_state.select {
                    if confirm_selection == 0 { // User selected YES
                        //if let Err(e) = delete_config_file() {
                        if let Err(e) = Config::delete() {
                            println!("[ERROR] Failed to delete config file: {}", e);
                        }
                        current_screen = Screen::ResetComplete;
                        sound_effects.play_select(&config);
                    } else { // User selected NO
                        current_screen = Screen::GeneralSettings;
                        sound_effects.play_back(&config);
                    }
                }

                // --- Render ---
                // First, render the settings page in the background
                render_settings_page(
                    1, &GENERAL_SETTINGS, &logo_cache, &background_cache, &font_cache,
                    &mut config, settings_menu_selection, &animation_state, &mut background_state,
                    &battery_info, &current_time_str, scale_factor, system_volume, brightness,
                );
                // Then, render the dialog box on top
                render_dialog_box(
                    "Reset all settings to default?\nThis cannot be undone.",
                    Some(("YES", "NO")), // Options to display
                    confirm_selection,  // Which option is selected
                    &font_cache, &config, scale_factor, &animation_state,
                );
            },
            Screen::ResetComplete => {
                // --- Input Handling ---
                if input_state.select || input_state.back {
                    // Use the restart function you already have
                    (current_screen, fade_start_time) = trigger_session_restart(&mut current_bgm, &music_cache);
                }

                // --- Render ---
                render_settings_page(
                    1, &GENERAL_SETTINGS, &logo_cache, &background_cache, &font_cache,
                    &mut config, settings_menu_selection, &animation_state, &mut background_state,
                    &battery_info, &current_time_str, scale_factor, system_volume, brightness
                );

                render_dialog_box(
                    "Settings have been reset.\nRestart required.",
                    None, // No YES/NO options needed
                    0,
                    &font_cache, &config, scale_factor, &animation_state,
                );
            },
            Screen::SaveData => {
                // Process one item from the icon queue each frame to prevent stuttering.
                if !icon_queue.is_empty() {
                    let (save_id, icon_path_str) = icon_queue.remove(0);
                    if let Ok(texture) = load_texture(&icon_path_str).await {
                        icon_cache.insert(save_id, texture);
                    }
                }

                ui::data::update(
                    &mut input_state, &mut current_screen, &sound_effects, &config,
                    &storage_state, &mut memories, &mut icon_cache, &mut icon_queue,
                    &mut selected_memory, &mut scroll_offset, &mut dialogs, &mut dialog_state, &mut animation_state,
                    scale_factor, &copy_op_state
                ).await;

                render_background(&background_cache, &config, &mut background_state);

                ui::data::draw(
                    selected_memory, &memories, &icon_cache, &font_cache,
                    &config, &storage_state, &placeholder, scroll_offset,
                    &input_state, &animation_state, &mut playtime_cache, &mut size_cache,
                    scale_factor, &dialog_state
                );

                // Draw dialogs on top if they are open
                if let Some(dialog) = dialogs.last_mut() {
                    if dialog_state == DialogState::Open {
                        ui::render_dialog(
                            dialog, &memories, selected_memory, &icon_cache, &font_cache,
                            &config, &copy_op_state, &placeholder, scroll_offset,
                            &animation_state, &mut playtime_cache, &mut size_cache, scale_factor
                        );
                    }
                }
            },
            Screen::Wifi => {
                ui::wifi::update(
                    &mut wifi_state,
                    &input_state,
                    &mut current_screen,
                    &sound_effects,
                    &config,
                );

                // Tell the about module to draw itself
                ui::wifi::draw(
                    &wifi_state,
                    &mut animation_state,
                    &logo_cache,
                    &background_cache,
                    &font_cache,
                    &config,
                    &mut background_state,
                    &battery_info,
                    &current_time_str,
                    scale_factor,
                );
            }
            Screen::Bluetooth => {
                ui::bluetooth::update(
                    &mut bluetooth_state,
                    &input_state,
                    &mut current_screen,
                    &sound_effects,
                    &config,
                );

                ui::bluetooth::draw(
                    &bluetooth_state,
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
            }
            Screen::ThemeDownloader => {
                ui::theme_downloader::update(
                    &mut theme_downloader_state,
                    &input_state,
                    &mut current_screen,
                    &sound_effects,
                    &config,
                );
                ui::theme_downloader::draw(
                    &theme_downloader_state,
                    &mut animation_state,
                    &background_cache,
                    &font_cache,
                    &config,
                    &mut background_state,
                    scale_factor,
                );
            }
            Screen::ReloadingThemes => {
                // ... (drawing "Reloading themes..." text)
                next_frame().await;

                // 1. Re-run the theme loading function
                loaded_themes = theme::load_all_themes().await;

                // 2. Re-scan all asset directories to find the new files
                let (background_files, logo_files, font_files, music_files) = find_all_asset_files();

                // --- Define a new message for reloading ---
                let reloading_text = "APPLYING NEW THEME ASSETS...";

                // 3. Re-load all assets and assign them to the original mutable caches
                (background_cache, logo_cache, music_cache, font_cache, sound_effects) =
                load_all_assets(
                    &config,
                    reloading_text,
                    &startup_font,
                    &background_files,
                    &logo_files,
                    &font_files,
                    &music_files
                ).await;

                // 4. After reloading, go back to the downloader screen
                current_screen = Screen::ThemeDownloader;
            }
            Screen::UpdateChecker => {
                ui::update_checker::update(
                    &mut update_checker_state,
                    &input_state,
                    &mut current_screen,
                    &sound_effects,
                    &config,
                );
                ui::update_checker::draw(
                    &mut update_checker_state,
                    &background_cache,
                    &font_cache,
                    &config,
                    &mut background_state,
                    scale_factor,
                );
            }
        }

        // This block checks if the settings screen requested an SFX reload
        if let Some(pack_name) = sfx_pack_to_reload.take() {
            println!("[Info] Reloading SFX pack: {}", pack_name);
            sound_effects = SoundEffects::load(&pack_name).await;
            // Play a sound from the new pack to confirm it changed
            sound_effects.play_cursor_move(&config);
        }
        next_frame().await
    }
}
