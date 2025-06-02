use super::launch::GameLayer;
use crate::serialization::{
    GameSerializable, colliders::SerializableCollider, materials::SerilializableMeshMaterial,
    meshes::SerializableMesh,
};
use avian2d::prelude::*;
use bevy::{
    ecs::{component::HookContext, world::DeferredWorld},
    prelude::*,
};

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
    #[allow(dead_code)]
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
