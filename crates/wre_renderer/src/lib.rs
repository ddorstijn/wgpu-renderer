mod renderable;

pub use renderable::Renderable;
use wre_window::WreWindow;

pub struct WreRenderer {
    is_surface_configured: bool,
}

impl WreRenderer {
    pub fn new(window: &WreWindow) -> Self {
        Self {
            is_surface_configured: false,
        }
    }

    pub fn render(&mut self, renderables: &[&dyn Renderable]) -> Result<(), ()> {
        Ok(())
    }
}
