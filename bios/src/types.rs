use crate::{Color, Vec2, Config, string_to_color, HashMap};
use macroquad::prelude::*;
use serde::{Serialize, Deserialize};
use std::str::FromStr;

// ===================================
// TYPES
// ===================================

// Playtime cache to avoid recalculating playtime for the same game on the same drive
pub type PlaytimeCacheKey = (String, String); // (cart_id, drive_name)
pub type PlaytimeCache = HashMap<PlaytimeCacheKey, f32>;

// Size cache to avoid recalculating size for the same game on the same drive
pub type SizeCacheKey = (String, String); // (cart_id, drive_name)
pub type SizeCache = HashMap<SizeCacheKey, f32>;

// ===================================
// ENUMS
// ===================================

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MenuPosition {
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShakeTarget {
    None,
    LeftArrow,
    RightArrow,
    Dialog,
    PlayOption,
    CopyLogOption,
    UnmountOption,
}

// SPLASH SCREEN
#[derive(Clone, Debug, PartialEq)]
pub enum SplashState {
    FadingIn,
    Showing,
    FadingOut,
    Done,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DialogState {
    None,
    Opening,
    Open,
    Closing,
}

// SCREENS
#[derive(Clone, Debug, PartialEq)]
pub enum Screen {
    MainMenu,
    SaveData,
    FadingOut,
    GeneralSettings,
    AudioSettings,
    GuiSettings,
    AssetSettings,
    ConfirmReset,
    ResetComplete,
    Extras,
    Wifi,
    Bluetooth,
    ThemeDownloader,
    ReloadingThemes,
    RuntimeDownloader,
    UpdateChecker,
    Debug,
    GameSelection,
    CdPlayer,
    About,
}

// UI Focus for Save Data Screen
#[derive(Clone, Debug, PartialEq)]
pub enum UIFocus {
    Grid,
    StorageLeft,
    StorageRight,
}

// A simple message for our new thread
pub enum GccMessage {
    RateUpdate(u32),
    Disconnected,
}

// ===================================
// STRUCTS
// ===================================

pub struct AppState {
    pub gcc_adapter_poll_rate: Option<u32>, // Store rate in Hz
}

pub struct CopyOperationState {
    pub progress: u16,
    pub running: bool,
    pub should_clear_dialogs: bool,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AudioSink {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os_name: String,
    pub kernel: String,
    pub cpu: String,
    pub gpu: String,
    pub ram_total: String,
}

pub struct BatteryInfo {
    pub percentage: String,
    pub status: String,
}

// color shifting background
pub struct BackgroundState {
    pub bgx: f32,
    pub bg_color: Color,
    pub target: usize,
    pub tg_color: Color,
}

#[derive(Clone, Debug)]
pub struct Memory {
    pub id: String,
    pub name: Option<String>,
    pub drive_name: String, // Store which drive this save is on
}

#[derive(Clone, Debug)]
pub struct StorageMedia {
    pub id: String,
    pub free: u32,
}

pub struct AnimationState {
    pub shake_time: f32,  // Current shake animation time
    pub shake_target: ShakeTarget, // Which element is currently shaking
    pub cursor_animation_time: f32, // Time counter for cursor animations
    pub cursor_transition_time: f32, // Time counter for cursor transition animation
    pub current_transition_duration: f32,
    pub dialog_transition_time: f32, // Time counter for dialog transition animation
    pub dialog_transition_progress: f32, // Progress of dialog transition (0.0 to 1.0)
    pub dialog_transition_start_pos: Vec2, // Starting position for icon transition
    pub dialog_transition_end_pos: Vec2, // Ending position for icon transition
}

// ===================================
// IMPL
// ===================================

// 1. Teach MenuPosition what its "default" value is.
impl Default for MenuPosition {
    fn default() -> Self {
        MenuPosition::Center // You can choose any default you like
    }
}

// 2. Teach MenuPosition how to be created from a string.
impl FromStr for MenuPosition {
    type Err = (); // We don't need a complex error type

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "TopLeft" => Ok(MenuPosition::TopLeft),
            "TopRight" => Ok(MenuPosition::TopRight),
            "BottomLeft" => Ok(MenuPosition::BottomLeft),
            "BottomRight" => Ok(MenuPosition::BottomRight),
            _ => Err(()), // If the string is anything else, it's an error
        }
    }
}

impl MenuPosition {
    // Helper function to easily cycle through the options in the settings menu
    pub fn next(&self) -> Self {
        match self {
            Self::Center => Self::TopLeft,
            Self::TopLeft => Self::TopRight,
            Self::TopRight => Self::BottomLeft,
            Self::BottomLeft => Self::BottomRight,
            Self::BottomRight => Self::Center,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Center => Self::BottomRight,
            Self::TopLeft => Self::Center,
            Self::TopRight => Self::TopLeft,
            Self::BottomLeft => Self::TopRight,
            Self::BottomRight => Self::BottomLeft,
        }
    }
}

impl AnimationState {
    const SHAKE_DURATION: f32 = 0.2;    // Duration of shake animation in seconds
    const SHAKE_INTENSITY: f32 = 3.0;   // How far the arrow shakes
    //const CURSOR_TRANSITION_DURATION: f32 = 0.15; // Duration of cursor transition animation
    const DIALOG_TRANSITION_DURATION: f32 = 0.4; // Duration of dialog transition animation

    pub fn new() -> Self {
        AnimationState {
            shake_time: 0.0,
            shake_target: ShakeTarget::None,
            cursor_animation_time: 0.0,
            cursor_transition_time: 0.0,
            current_transition_duration: 0.15,
            dialog_transition_time: 0.0,
            dialog_transition_progress: 0.0,
            dialog_transition_start_pos: Vec2::ZERO,
            dialog_transition_end_pos: Vec2::ZERO,
        }
    }

    pub fn calculate_shake_offset(&self, target: ShakeTarget) -> f32 {
        if self.shake_target == target && self.shake_time > 0.0 {
            (self.shake_time / Self::SHAKE_DURATION * std::f32::consts::PI * 8.0).sin() * Self::SHAKE_INTENSITY
        } else {
            0.0
        }
    }

    pub fn update_shake(&mut self, delta_time: f32) {
        // Update shake animation
        if self.shake_time > 0.0 {
            self.shake_time = (self.shake_time - delta_time).max(0.0);
            if self.shake_time <= 0.0 {
                self.shake_target = ShakeTarget::None;
            }
        }
    }

    pub fn update_cursor_animation(&mut self, delta_time: f32, speed_setting: &str) {

        // Determine numeric speed based on string setting
        let speed = match speed_setting {
            "FAST" => 15.0,
            "NORMAL" => 10.0,
            "SLOW" => 5.0,
            _ => 0.0, // "OFF"
        };

        if speed > 0.0 {
            // Standard animation
            self.cursor_animation_time = (self.cursor_animation_time + delta_time * speed) % (2.0 * std::f32::consts::PI);
        } else {
            // If OFF, lock time to PI/2.
            // sin(PI/2) = 1.0, ensuring the cursor stays fully lit/solid instead of freezing at a random dimness.
            self.cursor_animation_time = std::f32::consts::PI / 2.0;
        }

        // Update cursor transition (unchanged)
        if self.cursor_transition_time > 0.0 {
            self.cursor_transition_time = (self.cursor_transition_time - delta_time).max(0.0);
        }
    }

    pub fn trigger_shake(&mut self, is_left: bool) {
        if is_left {
            self.shake_target = ShakeTarget::LeftArrow;
            self.shake_time = Self::SHAKE_DURATION;
        } else {
            self.shake_target = ShakeTarget::RightArrow;
            self.shake_time = Self::SHAKE_DURATION;
        }
    }

    pub fn trigger_dialog_shake(&mut self) {
        self.shake_target = ShakeTarget::Dialog;
        self.shake_time = Self::SHAKE_DURATION;
    }

    pub fn trigger_play_option_shake(&mut self) {
        self.shake_target = ShakeTarget::PlayOption;
        self.shake_time = Self::SHAKE_DURATION;
    }

    pub fn trigger_copy_log_option_shake(&mut self) {
        self.shake_target = ShakeTarget::CopyLogOption;
        self.shake_time = Self::SHAKE_DURATION;
    }

    pub fn trigger_transition(&mut self, speed_setting: &str) {
        let duration = match speed_setting {
            "FAST" => 0.07,
            "NORMAL" => 0.15,
            "SLOW" => 0.30,
            _ => 0.0, // OFF
        };

        self.current_transition_duration = duration;
        self.cursor_transition_time = duration;
    }

    pub fn get_cursor_color(&self, config: &Config) -> Color { // Add config parameter
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

    pub fn get_cursor_scale(&self) -> f32 {
        // If duration is 0 (INSTANT) or time is 0, return 1.0 (no scale effect)
        if self.current_transition_duration <= 0.0 || self.cursor_transition_time <= 0.0 {
            return 1.0;
        }

        let t = self.cursor_transition_time / self.current_transition_duration;
        // Start at 1.5x size and smoothly transition to 1.0x
        1.0 + 0.5 * t
    }

    pub fn update_dialog_transition(&mut self, delta_time: f32) {
        if self.dialog_transition_time > 0.0 {
            self.dialog_transition_time = (self.dialog_transition_time - delta_time).max(0.0);
            self.dialog_transition_progress = 1.0 - (self.dialog_transition_time / Self::DIALOG_TRANSITION_DURATION);
        }
    }

    pub fn trigger_dialog_transition(&mut self, start_pos: Vec2, end_pos: Vec2) {
        self.dialog_transition_time = Self::DIALOG_TRANSITION_DURATION;
        self.dialog_transition_progress = 0.0;
        self.dialog_transition_start_pos = start_pos;
        self.dialog_transition_end_pos = end_pos;
    }

    pub fn get_dialog_transition_pos(&self) -> Vec2 {
        let t = self.dialog_transition_progress;
        // Use smooth easing function
        let t = t * t * (3.0 - 2.0 * t);
        self.dialog_transition_start_pos.lerp(self.dialog_transition_end_pos, t)
    }
}
