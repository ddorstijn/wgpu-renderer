use std::collections::HashSet;

pub use winit::{event::MouseButton, keyboard::KeyCode};

#[derive(Default, Debug)]
pub struct WreInput {
    pub pressed_keys: HashSet<KeyCode>,
    pub just_pressed_keys: HashSet<KeyCode>,
    pub just_released_keys: HashSet<KeyCode>,

    pub pressed_mouse_buttons: HashSet<MouseButton>,
    pub just_pressed_mouse_buttons: HashSet<MouseButton>,
    pub just_released_mouse_buttons: HashSet<MouseButton>,

    pub mouse_position: (f64, f64),     // (x, y)
    pub mouse_delta: (f64, f64),        // (dx, dy)
    pub mouse_scroll_delta: (f32, f32), // (dx, dy)

    // Store previous frame's state to detect "just pressed/released"
    previous_pressed_keys: HashSet<KeyCode>,
    previous_pressed_mouse_buttons: HashSet<MouseButton>,
}

impl WreInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn was_key_just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed_keys.contains(&key)
    }

    pub fn was_key_just_released(&self, key: KeyCode) -> bool {
        self.just_released_keys.contains(&key)
    }

    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.pressed_mouse_buttons.contains(&button)
    }

    pub fn was_mouse_button_just_pressed(&self, button: MouseButton) -> bool {
        self.just_pressed_mouse_buttons.contains(&button)
    }

    pub fn was_mouse_button_just_released(&self, button: MouseButton) -> bool {
        self.just_released_mouse_buttons.contains(&button)
    }

    pub fn get_mouse_position(&self) -> (f64, f64) {
        self.mouse_position
    }

    pub fn get_mouse_delta(&self) -> (f64, f64) {
        self.mouse_delta
    }

    pub fn get_mouse_scroll_delta(&self) -> (f32, f32) {
        self.mouse_scroll_delta
    }

    /// Call this once per frame before processing new events.
    pub fn start_frame(&mut self) {
        self.previous_pressed_keys = self.pressed_keys.clone();
        self.previous_pressed_mouse_buttons = self.pressed_mouse_buttons.clone();

        self.just_pressed_keys.clear();
        self.just_released_keys.clear();
        self.just_pressed_mouse_buttons.clear();
        self.just_released_mouse_buttons.clear();

        self.mouse_delta = (0.0, 0.0);
        self.mouse_scroll_delta = (0.0, 0.0);
    }

    /// Call this once per frame after processing all events.
    pub fn end_frame(&mut self) {
        // Calculate just_pressed and just_released after all events are processed for the frame
        for key in &self.pressed_keys {
            if !self.previous_pressed_keys.contains(key) {
                self.just_pressed_keys.insert(*key);
            }
        }
        for key in &self.previous_pressed_keys {
            if !self.pressed_keys.contains(key) {
                self.just_released_keys.insert(*key);
            }
        }

        for button in &self.pressed_mouse_buttons {
            if !self.previous_pressed_mouse_buttons.contains(button) {
                self.just_pressed_mouse_buttons.insert(*button);
            }
        }
        for button in &self.previous_pressed_mouse_buttons {
            if !self.pressed_mouse_buttons.contains(button) {
                self.just_released_mouse_buttons.insert(*button);
            }
        }
    }
}
