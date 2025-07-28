use std::{any::TypeId, collections::HashMap};

use anymap3::Map;
use slotmap::{DefaultKey, SecondaryMap, SlotMap};
use wre_transform::Transform;
/// The `Entity` ID, a key into the primary `SlotMap`.
pub type Entity = DefaultKey;

/// The trait for user-defined game logic systems.
pub trait System: 'static {
    fn update(&mut self, world: &mut World);
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

pub trait Component {}

/// The main World struct, containing all ECS data and systems.
pub struct World {
    /// The primary map storing all living entities and their required Transform.
    entities: SlotMap<Entity, Transform>,

    /// Stores all component data in SecondaryMaps.
    /// anymap3 maps the component type `C` to its `SecondaryMap<Entity, C>`.
    components: Map,

    /// Stores all systems to be run.
    registered_systems: HashMap<TypeId, Box<dyn System>>,
    system_execution_order: Vec<TypeId>,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: SlotMap::new(),
            components: Map::new(),
            registered_systems: HashMap::new(),
            system_execution_order: Vec::new(),
        }
    }

    // --- Entity Methods ---

    pub fn create_entity(&mut self, transform: Transform) -> Entity {
        self.entities.insert(transform)
    }

    /// Removes an entity from the world.
    /// Note: This leaves component data "dangling" in the secondary maps,
    /// which is perfectly fine and efficient. The storage will be reused,
    /// and any attempt to access it with the old key will correctly fail.
    pub fn remove_entity(&mut self, entity: Entity) -> Option<Transform> {
        self.entities.remove(entity)
    }

    pub fn get_entities(&self) -> Vec<Entity> {
        self.entities.keys().collect()
    }

    pub fn get_transform(&self, entity: Entity) -> &Transform {
        if self.entities.contains_key(entity) {
            let transform = unsafe { self.entities.get_unchecked(entity) };
            return transform;
        }

        panic!("Entity not found");
    }

    pub fn get_transform_mut(&mut self, entity: Entity) -> &mut Transform {
        if self.entities.contains_key(entity) {
            let transform = unsafe { self.entities.get_unchecked_mut(entity) };
            return transform;
        }

        panic!("Entity not found");
    }

    pub fn get_transforms(&self) -> &SlotMap<Entity, Transform> {
        &self.entities
    }

    // New method to get mutable access to the entire Transform SlotMap
    pub fn get_transforms_mut(&mut self) -> &mut SlotMap<Entity, Transform> {
        &mut self.entities
    }

    // --- Component Methods ---

    pub fn register_component<C: Component + 'static>(&mut self) {
        // We map the component type `C` directly to its SecondaryMap storage.
        self.components
            .insert::<SecondaryMap<Entity, C>>(SecondaryMap::new());
    }

    pub fn add_component<C: Component + 'static>(&mut self, entity: Entity, component: C) {
        if let Some(map) = self.components.get_mut::<SecondaryMap<Entity, C>>() {
            map.insert(entity, component);
        } else {
            panic!("Component type not registered. Call world.register_component::<C>() first.");
        }
    }

    pub fn get_component<C: Component + 'static>(&self, entity: Entity) -> Option<&C> {
        self.components
            .get::<SecondaryMap<Entity, C>>()
            .and_then(|map| map.get(entity))
    }

    pub fn get_component_mut<C: Component + 'static>(&mut self, entity: Entity) -> Option<&mut C> {
        self.components
            .get_mut::<SecondaryMap<Entity, C>>()
            .and_then(|map| map.get_mut(entity))
    }

    pub fn get_components<C: Component + 'static>(&self) -> &SecondaryMap<DefaultKey, C> {
        self.components
            .get::<SecondaryMap<Entity, C>>()
            .expect("Component not registered")
    }

    pub fn get_components_mut<C: Component + 'static>(
        &mut self,
    ) -> &mut SecondaryMap<DefaultKey, C> {
        self.components
            .get_mut::<SecondaryMap<Entity, C>>()
            .expect("Component not registered")
    }

    // --- System Methods: ZST check for instantiation ---
    /// Registers a system by its type.
    /// Requires the system type `S` to be a Zero-Sized Type (ZST),
    /// effectively forcing it to be a unit struct.
    pub fn register_system<S: System + Default + 'static>(&mut self) {
        // Default bound here!
        let system_instance = S::default(); // Now S must implement Default
        let type_id = system_instance.type_id();

        let existing_system = self
            .registered_systems
            .insert(type_id, Box::new(system_instance));

        if existing_system.is_none() {
            self.system_execution_order.push(type_id);
        }
    }

    /// Runs all registered systems in their defined order.
    pub fn run_systems(&mut self) {
        let mut registered_systems = std::mem::take(&mut self.registered_systems);
        let order = self.system_execution_order.clone();

        for type_id in order {
            if let Some(system) = registered_systems.get_mut(&type_id) {
                system.update(self);
            }
        }
        self.registered_systems = registered_systems;
    }
}
