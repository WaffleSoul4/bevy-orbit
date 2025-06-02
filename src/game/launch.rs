use super::{
    GameState,
    death::{DeathEventsEnabled, DieOnCollision},
    gravity::{Gravity, GravityLayers},
    trace::Traceable,
};
use crate::serialization::{materials::SerilializableMeshMaterial, meshes::SerializableMesh};
use avian2d::prelude::*;
use bevy::prelude::*;

// Yep, I'm calling it the launch object
#[derive(Bundle)]
pub struct LaunchObjectBundle {
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
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
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
    #[allow(dead_code)]
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

    pub fn with_position(self, position: Vec2) -> Self {
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
    #[allow(dead_code)]
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
    traceable: Traceable,
    death_events_enabled: DeathEventsEnabled,
    die_on_collision: DieOnCollision,
}

impl From<&LaunchingObjectConfig> for DynamicObjectBundle {
    fn from(config: &LaunchingObjectConfig) -> Self {
        DynamicObjectBundle {
            gravity: Gravity,
            collider: config.collider.clone(),
            mass: Mass(config.mass),
            velocity: LinearVelocity(Vec2::ZERO),
            rigid_body: RigidBody::Dynamic,
            traceable: Traceable,
            death_events_enabled: DeathEventsEnabled,
            die_on_collision: DieOnCollision,
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

pub fn spawn_launching_objects(
    mut commands: Commands,
    launching: Query<(), With<Launching>>,
    starting_point: Res<crate::serialization::StartPoint>,
) {
    if launching.is_empty() {
        if let Some(start_point) = **starting_point {
            commands.spawn(LaunchObjectBundle::default().with_position(start_point));
        } else {
            warn!("Tried to spawn a launching object while starting point was unset");
        }
    }
}

#[derive(Component)]
pub struct Launching;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct DynamicObject;

#[derive(PhysicsLayer, Default)]
pub enum GameLayer {
    #[default]
    Main,
    Triggers,
}

pub fn launch_launching(
    launching_query: Query<(Entity, &Transform, &LaunchingObjectConfig), With<Launching>>,
    cursor_position: Res<crate::cursor::CursorPosition>,
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
) {
    launching_query
        .iter()
        .for_each(|(entity, transform, config)| {
            let dif =
                transform.translation.xy() - cursor_position.unwrap_or(transform.translation.xy());

            let launched = commands
                .entity(entity)
                .insert(DynamicObjectBundle::new(config, dif))
                .remove::<Launching>()
                .id();

            commands.spawn((
                super::trace::PathTracer::new(launched),
                Transform::from_translation(Vec3::ZERO),
            ));

            *game_state = match *game_state {
                GameState::Launching => GameState::Launched,
                GameState::Paused => {
                    warn!("Object launched while game paused");
                    GameState::Paused
                }
                GameState::Sandbox => GameState::Sandbox,
                GameState::Launched => {
                    warn!("Object launched while in the launched state of the game, switching to sandbox");
                    GameState::Sandbox
                } // If something is launched while already in the launched state, enter sandbox
            }
        });
}
