pub mod colliders;
pub mod editor;
pub mod game;
pub mod materials;
pub mod meshes;
pub mod zones;

use bevy::prelude::*;
use std::path::PathBuf;

pub struct SerializationPlugin;

impl Plugin for SerializationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((SerializeableTypeRegistrationPlugin, zones::ZonePlugin))
            .add_event::<SaveEvent>()
            .insert_resource(LevelSerializationData::new("test_levels/level2.scn.ron"))
            .init_resource::<StartPoint>()
            .add_systems(
                Update,
                (
                    colliders::initialize_colliders,
                    meshes::initialize_meshes,
                    materials::initialize_mesh_materials,
                    editor::serialize_objects,
                    editor::free_temp_scene_children,
                    draw_start_point.run_if(crate::helper::app_state_is(crate::AppState::Editor)),
                ),
            );
    }
}

type InternalSerializableTypes = (
    crate::game::gravity::Gravity,
    crate::game::gravity::GravityLayers,
    crate::game::trigger::GameTrigger,
    crate::game::death::KillOnCollision,
    crate::game::launch::DynamicObject,
    colliders::SerializableCollider,
    meshes::SerializableMesh,
    meshes::SerializableMeshPrimitives,
    materials::SerilializableMeshMaterial,
    GameSerializable,
    StartPoint,
    LevelObject,
);

type ExternalSerializableTypes = (
    avian2d::prelude::CollisionLayers,
    avian2d::prelude::Mass,
    avian2d::prelude::RigidBody,
    Transform,
);

// Put types that need to be serialized in here to add them to the registry
pub struct SerializeableTypeRegistrationPlugin;

impl Plugin for SerializeableTypeRegistrationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<(ExternalSerializableTypes, InternalSerializableTypes)>();
        info!("Serializable types registered!")
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct LevelObject;

// Ok a few things here
// When I refer to level, I'm referring to the current scene that has the level info
// When I refer to serialization, that is the act of writing the scene to a file
// Deserialization is getting the scene from a file
// Serializable entities should automatically be added to the level on being spwaned
//
// Things I have to do:
// - Initialize the level
// - Improve events for serializing and deserializing
// - (Maybe) be able to handle different scenes without having an aneurysm
//
// Problems
// - When serializing, some componenets must be removed. This means the data has to
// go through a sort of middle layer where all the components get filtered. But filters
// can only be used in builders, which can only extract entities from a world...
//
// Dynamic Scene -> Normal Scene -> World -> Dynamic Scene Builder ---filtering--> Serializable Dynamic Scene!
//
// Or just figure out how type registries work and use those as a sort of filter instead...
//
// A dynamic scene is serializable and generally pretty cool, also can be built from
// a dynamic scene builder
// A normal scene is just a world in a box that can't do like anything
//
// After a lot of thought, I've realised a few things
// - Assets aren't meant to modified, and won't suffice for holding all the data
// - (I could theoretically make spawning entities just write to that file then reload the assets, but I like spawn)
// - The alternatives are
//     1. Serializing from the world like I was doing before (Less work, mediocre)
//     2. Adding another scene as a resource that serves as a buffer (Whoever thought of this is an idiot)
//     3. Seperate editing from main game functionality (Most work, seems great in concept) <-- This one!

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct GameSerializable;

#[derive(Resource)]
pub struct LevelSerializationData {
    pub path: PathBuf, // Should this be an option?
}

impl LevelSerializationData {
    fn new<T: Into<PathBuf>>(path: T) -> Self {
        LevelSerializationData { path: path.into() }
    }
}
// Marker for the active level
#[derive(Component)]
pub struct ActiveLevel;

// Events for saving and loading
// Pros
// - Events for saving allow me to add more flexibility
// - Only way for remote systems to trigger a save
// - Better abstraction (maybe)
// Cons
// - Adds an extra layer of complexity to already complex system
// I'll try to keep it for now
//
// But these events are only useful in the context of the editor!
#[derive(Event)]
pub struct SaveEvent {
    path: PathBuf,
}

impl SaveEvent {
    pub fn new<U: Into<PathBuf>>(path: U) -> Self {
        SaveEvent { path: path.into() }
    }
}

// Instead of storing an option I could just make it either exist or not...
#[derive(Deref, DerefMut, Reflect, Resource, Debug)]
#[reflect(Resource)]
pub struct StartPoint(Option<Vec2>);

impl Default for StartPoint {
    fn default() -> Self {
        StartPoint(None)
    }
}

pub fn draw_start_point(start_point: Res<StartPoint>, mut gizmos: Gizmos) {
    if let Some(start_point) = **start_point {
        let start_point_color = bevy::color::palettes::basic::WHITE;

        gizmos.circle_2d(
            Isometry2d::from_translation(start_point),
            10.0,
            start_point_color,
        );
        gizmos.line_2d(
            start_point + Vec2::new(5.0, 0.0),
            start_point + Vec2::new(-5.0, 0.0),
            start_point_color,
        );
        gizmos.line_2d(
            start_point + Vec2::new(0.0, 5.0),
            start_point + Vec2::new(0.0, -5.0),
            start_point_color,
        );
    }
}
