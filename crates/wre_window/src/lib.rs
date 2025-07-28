use std::{sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::{self, ActiveEventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

pub use winit::{event::MouseButton, keyboard::KeyCode};

pub struct WreWindow {
    window: Arc<Window>,
}

impl WreWindow {
    pub fn new(event_loop: &ActiveEventLoop) -> Self {
        let window_attributes = Window::default_attributes();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        Self { window }
    }
}
