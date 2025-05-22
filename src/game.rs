use crate::{
    cursor::CursorPosition, gravity::{GravityLayers, Gravity}, serialization::{SerializableMesh, SerilializableMeshMaterial}, DynamicObject, Launching
};
use avian2d::prelude::*;
use bevy::prelude::*;

// Yep, I'm calling it the launch object
#[derive(Bundle)]
struct LaunchObjectBundle {
    transform: Transform,
    mesh: SerializableMesh, // I might need this to be serializable
    material: SerilializableMeshMaterial, // Same here
    gravity_layers: GravityLayers,
    config: LaunchingObjectConfig,
    dynamic_object: DynamicObject,
    launching: Launching,
}

impl Default for LaunchObjectBundle {
    fn default() -> Self {
        LaunchObjectBundle {
            transform: Transform::from_translation(Vec3::ZERO),
            mesh: SerializableMesh::primitive(Circle::new(10.0)),
            material: SerilializableMeshMaterial::color(Color::oklch(0.6067, 0.1, 298.59)),
            gravity_layers: GravityLayers::default(),
            config: LaunchingObjectConfig::default(),
            dynamic_object: DynamicObject,
            launching: Launching,
        }
    }
}

impl LaunchObjectBundle {
    fn from_circle(circle: Circle) -> Self {
        LaunchObjectBundle {
            mesh: SerializableMesh::primitive(circle),
            config: LaunchingObjectConfig::from_circle(circle),
            ..default()
        }
    }

    fn with_translation(mut self, translation: Vec3) -> Self {
        self.transform.translation = translation;

        self
    }

    fn with_position(self, position: Vec2) -> Self {
        self.with_translation(position.extend(0.0))
    }
}

/// Configuration for an object that is in the launching state
#[derive(Component)]
pub struct LaunchingObjectConfig {
    pub gravity_layers: GravityLayers,
    pub collider: Collider,
    pub mass: f32,
}

impl Default for LaunchingObjectConfig {
    fn default() -> Self {
        LaunchingObjectConfig {
            gravity_layers: GravityLayers::default(),
            collider: Collider::circle(10.0),
            mass: 5.0,
        }
    }
}

impl LaunchingObjectConfig {
    fn from_circle(circle: Circle) -> Self {
        LaunchingObjectConfig {
            collider: Collider::circle(circle.radius),
            ..default()
        }
    }
}
// Post launch
#[derive(Bundle)]
pub struct DynamicObjectBundle {
    gravity: Gravity,
    collider: Collider,
    mass: Mass,
    velocity: LinearVelocity,
    rigid_body: RigidBody,
}

impl From<&LaunchingObjectConfig> for DynamicObjectBundle {
    fn from(config: &LaunchingObjectConfig) -> Self {
        DynamicObjectBundle {
            gravity: Gravity,
            collider: config.collider.clone(),
            mass: Mass(config.mass),
            velocity: LinearVelocity(Vec2::ZERO),
            rigid_body: RigidBody::Dynamic,
        }
    }
}

impl DynamicObjectBundle {
    pub fn new(config: &LaunchingObjectConfig, velocity: Vec2) -> Self {
        DynamicObjectBundle {
            velocity: LinearVelocity(velocity),
            ..DynamicObjectBundle::from(config)
        }
    }
}

pub fn game_input_handler(
    _keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_position: Res<CursorPosition>,
    mut commands: Commands,
) {

    if let Some(cursor_position) = **cursor_position {
        if mouse.just_pressed(MouseButton::Left) {
            commands.spawn(LaunchObjectBundle::default().with_position(cursor_position));
        }
    }
}
