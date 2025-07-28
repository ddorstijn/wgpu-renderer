use winit::event::WindowEvent;
use wre_input::WreInput;
use wre_renderer::WreRenderer;
use wre_window::WreWindow;

pub struct WreApp {
    renderer: Option<WreRenderer>,
    window: Option<WreWindow>,
    world: WreWorld,
    input: WreInput,
    player_controller: WrePlayer,
}

impl ApplicationHandler for WreApp {
    // This is called once the event loop is running.
    // It's the new place for initialization.
    fn resumed(&mut self, event_loop: &impl ActiveEventLoop) {
        if self.window.is_none() {
            let window = Window::new(event_loop);
            let renderer = pollster::block_on(Renderer::new(&window));

            self.window = Some(window);
            self.renderer = Some(renderer);

            // You can now load your world assets, as the renderer is ready
            // self.world.load_assets(self.renderer.as_ref().unwrap());
        }
    }

    // This replaces the main match block for window events.
    fn window_event(
        &mut self,
        event_loop: &impl ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Ensure the event is for our main window, and the window exists.
        if self.window.is_none() || window_id != self.window.as_ref().unwrap().id() {
            return;
        }

        let window = self.window.as_ref().unwrap();
        let renderer = self.renderer.as_mut().unwrap();

        // Let the input manager process first
        if !self.input.process_event(&event) {
            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::Resized(physical_size) => {
                    renderer.resize(physical_size);
                }
                WindowEvent::RedrawRequested => {
                    // --- UPDATE ---
                    self.player_controller
                        .update_camera(&mut self.world.camera, &self.input);
                    self.world.update();
                    self.input.end_frame();

                    // --- RENDER ---
                    match renderer.render(&self.world) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size()),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => eprintln!("Render error: {:?}", e),
                    }
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(code),
                            state: keystate,
                            ..
                        },
                    ..
                } => {
                    todo!()
                }
                _ => (),
            }
        }
    }

    // This is called when the event loop is about to block and wait for new events.
    // It's the perfect place to request a redraw for the next frame.
    fn about_to_wait(&mut self, event_loop: &impl ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.winit_window.request_redraw();
        }
    }
}
