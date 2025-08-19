use crate::entity::WorldEntity;

mod entity;

pub struct WreWorld {
    pub entities: Vec<std::boxed::Box<dyn WorldEntity>>,
}
