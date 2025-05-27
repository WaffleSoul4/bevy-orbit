use std::f32::consts::PI;

use crate::{
    DynamicObject, GameLayer, Launching, add_observer_on_hook,
    cursor::CursorPosition,
    gravity::{Gravity, GravityLayers},
    serialization::{
        GameSerializable, SerializableCollider, SerializableMesh, SerilializableMeshMaterial,
    },
};
use avian2d::prelude::*;
use bevy::{
    ecs::{component::HookContext, world::DeferredWorld},
    prelude::*,
};

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

// State is represented by the presence of [Triggered]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(CollisionEventsEnabled)]
#[component(on_add = initialize_trigger_observers)]
pub struct GameTrigger;

fn initialize_trigger_observers(mut world: DeferredWorld, context: HookContext) {
    // info!("Initializing trigger observer");

    world.commands().entity(context.entity).observe(
        |trigger: Trigger<OnCollisionStart>, mut commands: Commands| {
            // info!("Collision event w/ trigger");
            commands.entity(trigger.target()).insert_if_new(Triggered);
        },
    );
}

#[derive(Bundle)]
pub struct GameTriggerBundle {
    transform: Transform,
    mesh: SerializableMesh,
    material: SerilializableMeshMaterial,
    game_trigger: GameTrigger, // Collision event enabled implied here
    collider: SerializableCollider,
    collision_layers: CollisionLayers,
    serializable: GameSerializable,
    collision_events: CollisionEventsEnabled,
}

impl Default for GameTriggerBundle {
    fn default() -> Self {
        GameTriggerBundle {
            transform: Transform::default(),
            mesh: SerializableMesh::primitive(Circle::new(10.0)),
            material: SerilializableMeshMaterial::color(Color::srgb(0.1, 0.3, 0.7)),
            game_trigger: GameTrigger,
            collider: SerializableCollider::new(ColliderConstructor::Circle { radius: 10.0 }),
            collision_layers: CollisionLayers::new(GameLayer::Triggers, GameLayer::Main),
            serializable: GameSerializable,
            collision_events: CollisionEventsEnabled,
        }
    }
}

impl GameTriggerBundle {
    fn from_circle(circle: Circle) -> Self {
        GameTriggerBundle {
            mesh: SerializableMesh::primitive(circle),
            collider: SerializableCollider::new(ColliderConstructor::Circle {
                radius: circle.radius,
            }),
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

#[derive(Component)]
// #[component(on_remove = clean_trigger_indicator)]
pub struct Triggered;

// Using system here is just so much easier and less error prone
#[allow(dead_code)]
fn clean_trigger_indicator(mut world: DeferredWorld, context: HookContext) {
    world
        .commands()
        .get_entity(context.entity)
        .expect("Failed to get entity of removed trigger")
        .despawn_related::<Children>();

    // info!("Deleted the children of {}", context.entity)
}

pub fn clear_triggered_indicators(
    trigger_query: Query<Entity, (Without<Triggered>, With<Children>, With<GameTrigger>)>,
    mut commands: Commands,
) {
    trigger_query.iter().for_each(|entity| {
        commands
            .get_entity(entity)
            .and_then(|mut entity_commands| {
                entity_commands.despawn_related::<Children>();
                Ok(())
            })
            .expect("Failed to get entity commands for clearing trigger indicator")
    });
}

pub fn initialize_triggered_indicators(
    trigger_query: Query<Entity, (With<Triggered>, Without<Children>)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // SystemState + DeferredWorld = Nope
    // Same with resource scope...
    // How do I get resources in my component hook?

    trigger_query.iter().for_each(|entity| {
        let child = commands
            .spawn((
                Mesh2d(meshes.add(Circle::new(12.0))), // Always a circle (for now)
                MeshMaterial2d(materials.add(Color::srgb(0.1, 0.7, 0.3))),
                Transform::from_translation(Vec2::ZERO.extend(-1.0)),
            ))
            .id();

        commands.entity(entity).add_child(child);

        // info!("Spawned child of trigger {}", entity)
    });
}

#[derive(Component)]
pub struct Traceable;

// Traces the path of an entity and spawns the path as its children
#[derive(Component)]
#[component(on_add = get_starting_position)]
pub struct PathTracer {
    previous: Vec2,
    precision: u32, // Zero is every frame
    min_length: f32,
    width: f32,
    color: Color,
    precision_counter: u32,
    target: Option<Entity>, // No entity means disabled
}

// Mmm tasty scopes
fn get_starting_position(mut world: DeferredWorld, context: HookContext) {
    let tracer_target_entity = {
        let mut tracer_commands = world.entity_mut(context.entity);

        tracer_commands
            .get_mut::<PathTracer>()
            .expect("What...")
            .target
            .clone()
    };

    let target_transform = {
        world
            .get_entity(
                tracer_target_entity.expect("Please provide a target when initializing tracers"),
            )
            .expect("Invalid target entity found for tracer")
            .get::<Transform>()
            .expect("Tracer target doesn't have a transfor to trace")
            .clone()
    };

    // Duplication out of necessity

    let mut tracer_commands = world.entity_mut(context.entity);

    let mut tracer = tracer_commands
        .get_mut::<PathTracer>()
        .expect("This is a hook for if this component was added ofc it's here");

    tracer.previous = target_transform.translation.xy();
}

impl PathTracer {
    pub fn new(target: Entity) -> Self {
        PathTracer {
            previous: Vec2::ZERO,
            precision: 1, // Every other frame
            min_length: 3.0,
            width: 2.0,
            color: Color::srgb(0.1, 0.3, 0.7),
            precision_counter: 0,
            target: Some(target),
        }
    }

    pub fn increment(&mut self) {
        self.precision_counter += 1
    }

    pub fn reset(&mut self) {
        self.precision_counter = 0
    }
}

#[derive(Component)]
pub struct PathSegment;

pub fn trace_object_paths(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    mut tracers: Query<(Entity, &mut PathTracer)>,
    traceable: Query<&GlobalTransform, With<Traceable>>,
) {
    tracers
        .iter_mut()
        .filter(|(_, tracer)| tracer.target.is_some())
        .for_each(|(entity, mut tracer)| {
            if tracer.precision_counter >= tracer.precision {
                match traceable.get(tracer.target.unwrap()) {
                    Ok(transform) => {
                        let difference = transform.translation().xy() - tracer.previous;

                        if difference.length() > tracer.min_length {
                            let length = difference.length() * 1.5;

                            let rectangle = Rectangle::from_size(Vec2::new(tracer.width, length));

                            let angle = difference.to_angle() + PI / 2.0; // Add 90 degrees

                            let segment = commands
                                .spawn((
                                    MeshMaterial2d(materials.add(tracer.color)),
                                    Mesh2d(meshes.add(rectangle)),
                                    Transform::from_rotation(Quat::from_rotation_z(angle))
                                        .with_translation(
                                            transform.translation().xy().extend(-1.0),
                                        ),
                                    PathSegment,
                                ))
                                .id();

                            commands.entity(entity).add_child(segment);

                            tracer.previous = transform.translation().xy();
                        }
                    }
                    Err(e) => {
                        info!("Tracer failed to find target: {}, Disabling", e);

                        tracer.target = None;
                    }
                }

                tracer.reset();
            } else {
                tracer.increment();
            }
        });
}

#[derive(Debug)]
pub enum DeathSource {
    Reset,
    Collision,
}

/// Defines whether an entity can give a death event (it can die)
#[derive(Component)]
#[component(on_add = add_observer_on_hook(death_event_handler))]
pub struct DeathEventsEnabled;

/// Defines whether an entity can kill entities with death events enabled on collision
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct KillOnCollision;

/// Defines whether an entity will die upon collision (I might replace this with a DeathLayers thing later)
#[derive(Component)]
#[component(on_add = add_observer_on_hook(collision_observer))]
#[require(CollisionEventsEnabled)]
pub struct DieOnCollision;

fn collision_observer(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    query: Query<(), With<KillOnCollision>>,
) {
    if query.contains(trigger.collider) {
        commands.trigger_targets(DeathEvent::new(DeathSource::Collision), trigger.target());
    }
}

fn death_event_handler(trigger: Trigger<DeathEvent>, mut commands: Commands) {
    info!("Object died from {:?}", trigger.source);

    commands.entity(trigger.target()).despawn();
}

#[derive(Event, Debug)]
pub struct DeathEvent {
    source: DeathSource,
}

impl DeathEvent {
    pub fn new(source: DeathSource) -> Self {
        DeathEvent { source }
    }
}
