pub trait WreEntity {
    /// Called once when the entity is initialized.
    fn init(&mut self);

    /// Called on every frame to update the entity's state.
    fn update(&mut self);

    /// Called on every frame to render the entity.
    fn render(&mut self);
}

pub struct WreWorld {
    pub entities: Vec<std::boxed::Box<dyn WreEntity>>,
}

impl WreWorld {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// Public method to add an entity to the world. This is the only way
    /// the end user will interact with the entity collection.
    pub fn add_entity(&mut self, mut entity: Box<dyn WreEntity>) {
        entity.init();
        self.entities.push(entity);
    }

    /// Internal method to update all entities in the world.
    pub fn update_entities(&mut self) {
        for entity in self.entities.iter_mut() {
            entity.update();
        }
    }

    /// Internal method to render all entities in the world.
    pub fn render_entities(&mut self) {
        for entity in self.entities.iter_mut() {
            entity.render();
        }
    }
}
