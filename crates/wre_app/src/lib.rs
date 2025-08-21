use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};
use wre_input::WreInput;
use wre_renderer::WreRenderer;
use wre_world::{WreEntity, WreWorld};

pub struct WreApp<T> {
    application: T,
    renderer: Option<WreRenderer>,
    window: Option<Arc<Window>>,
    world: WreWorld,
    input: WreInput,
}

impl<T> WreApp<T> {
    pub fn new(application: T) -> Self {
        Self {
            application,
            renderer: None,
            window: None,
            world: WreWorld {
                entities: Vec::new(),
            },
            input: WreInput::new(),
        }
    }
}

impl<T> ApplicationHandler for WreApp<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = WindowAttributes::default().with_title("App");
        let window = Arc::new(event_loop.create_window(attributes).unwrap());
        self.renderer = Some(WreRenderer::new(window.clone()).unwrap());
        self.window = Some(window.clone());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            // Call the world's update for all entities.
            self.world.update_entities();

            window.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        println!("Application is exiting gracefully.");
    }
}

pub struct ApplicationBuilder<T: 'static> {
    state: T,
    world: WreWorld,
}

impl<T: 'static> ApplicationBuilder<T> {
    /// Creates a new builder with the initial application state.
    pub fn new(state: T) -> Self {
        Self {
            state,
            world: WreWorld::new(),
        }
    }

    /// Adds an entity to the world. The entity type must implement both `Entity` and `Default`.
    pub fn add_entity<E: WreEntity + Default + 'static>(mut self) -> Self {
        self.world.add_entity(Box::new(E::default()));
        self
    }

    /// Consumes the builder and runs the game engine.
    pub fn run(self) -> Result<(), impl std::error::Error + 'static> {
        let event_loop = EventLoop::new().unwrap();

        // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
        // dispatched any events. This is ideal for games and similar applications.
        event_loop.set_control_flow(ControlFlow::Poll);

        // ControlFlow::Wait pauses the event loop if no events are available to process.
        // This is ideal for non-game applications that only update in response to user
        // input, and uses significantly less power/CPU time than ControlFlow::Poll.
        event_loop.set_control_flow(ControlFlow::Wait);

        let mut app = WreApp::new(self.state);
        event_loop.run_app(&mut app)
    }
}
