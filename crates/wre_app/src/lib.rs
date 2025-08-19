use winit::{application::ApplicationHandler, event::WindowEvent};
use wre_input::WreInput;
use wre_renderer::WreRenderer;
use wre_window::WreWindow;
use wre_world::WreWorld;

pub struct WreApp {
    renderer: Option<WreRenderer>,
    window: Option<WreWindow>,
    world: WreWorld,
    input: WreInput,
}

impl WreApp {
    pub fn new() -> Self {
        Self {
            renderer: None,
            window: None,
            world: WreWorld {
                entities: Vec::new(),
            },
            input: WreInput::new(),
        }
    }
}

impl ApplicationHandler for WreApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window = Some(WreWindow::new(event_loop));
        self.renderer = Some(WreRenderer::new(self.window.as_ref().unwrap()));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        todo!()
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        todo!()
    }
}
