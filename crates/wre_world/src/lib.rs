mod entity;

pub use entity::WorldEntity;

pub struct World {
    entities: Vec<dyn &WorldEntity>,
}

impl World {
    fn new() -> Self {
        Self {}
    }
}
