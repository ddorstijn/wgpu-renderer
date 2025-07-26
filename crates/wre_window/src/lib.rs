use std::{sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

pub use winit::{event::MouseButton, keyboard::KeyCode};

pub struct WreWindow {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
}

impl WreWindow {
    pub fn new() -> Self {
        Self {}
    }
}

impl ApplicationHandler for WreWindow {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(s) => s,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta_time = now - self.last_frame_instant;

                state.update(delta_time);
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => log::error!("Unable to render {}", e),
                }

                self.last_frame_instant = Instant::now();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: keystate,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, keystate.is_pressed()),
            _ => {}
        }
    }
}
