use macroquad::prelude::*;
use gilrs::{Gilrs, Button, Axis};
use crate::types::UIFocus; // Assuming UIFocus is in types.rs

pub struct InputState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub select: bool,
    pub next: bool,
    pub prev: bool,
    pub cycle: bool,
    pub back: bool,
    pub analog_was_neutral: bool,
    pub ui_focus: UIFocus,
}

impl InputState {
    const ANALOG_DEADZONE: f32 = 0.5;  // Increased deadzone for less sensitivity

    pub fn new() -> Self {
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

    pub fn update_keyboard(&mut self) {
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

    pub fn update_controller(&mut self, gilrs: &mut Gilrs) {
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
