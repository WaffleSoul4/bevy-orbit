use std::{fs::File, path::PathBuf};

use bevy::{prelude::*, reflect::TypeRegistry};

use super::{ExternalSerializableTypes, GameSerializable, InternalSerializableTypes};

// Only for use in editor mode
pub fn serialize_objects(
    mut events: EventReader<super::SaveEvent>,
    world: &World,
    entities: Query<Entity, With<super::GameSerializable>>,
) {
    for event in events.read() {
        // Dynamic programming when
        let scene_builder = DynamicSceneBuilder::from_world(world)
            // Internal types
            .allow_component::<crate::game::gravity::Gravity>()
            .allow_component::<crate::game::gravity::GravityLayers>()
            .allow_component::<crate::game::trigger::GameTrigger>()
            .allow_component::<crate::game::death::KillOnCollision>()
            .allow_component::<crate::serialization::LevelObject>()
            .allow_component::<crate::game::launch::DynamicObject>()
            .allow_component::<crate::serialization::colliders::SerializableCollider>()
            .allow_component::<crate::serialization::meshes::SerializableMesh>()
            .allow_component::<crate::serialization::materials::SerilializableMeshMaterial>()
            .allow_component::<crate::serialization::GameSerializable>()
            // External types
            .allow_component::<Transform>()
            .allow_component::<avian2d::prelude::CollisionLayers>()
            .allow_component::<avian2d::prelude::Mass>()
            .allow_component::<avian2d::prelude::RigidBody>()
            // Resources
            .allow_resource::<crate::serialization::StartPoint>();

        let scene = scene_builder
            .extract_entities(entities.iter())
            .extract_resources()
            .build();

        // info!(
        //     "Scene: {:?}",
        //     scene
        //         .entities
        //         .iter()
        //         .for_each(|entity| info!("{:?}", entity.components))
        // );

        let type_registry = serializable_components_type_registry();

        let serialized = scene.serialize(&type_registry).unwrap_or_else(|err| {
            error!("Failed to serialize scene: {err}");
            String::new()
        });

        use std::io::Write;

        let mut target_path = PathBuf::from("assets/");

        target_path.push(event.path.clone());

        bevy::tasks::IoTaskPool::get()
            .spawn(async move {
                File::create(target_path.clone())
                    .unwrap_or_else(|err| {
                        panic!("Failed to open file '{}': {err}", target_path.display())
                    })
                    .write_all(serialized.as_bytes())
                    .unwrap_or_else(|err| {
                        panic!(
                            "Failed to write data to file '{}': {err}",
                            target_path.display()
                        )
                    });

                info!(
                    "Succesfully serialized and saved scene to '{}'",
                    target_path.display()
                )
            })
            .detach();
    }
}

#[derive(Component)]
pub struct TempSceneRoot;

// Only for use in editor
// Just like the scene thing but extract all of the entities directly into the world
pub fn spawn_temp_scene(
    level_serialization_data: Res<super::LevelSerializationData>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let scene: Handle<DynamicScene> = asset_server.load(level_serialization_data.path.clone());

    commands.spawn((TempSceneRoot, DynamicSceneRoot(scene)));
}

pub fn free_temp_scene_children(
    temp_scene: Single<Entity, With<TempSceneRoot>>,
    children: Query<Entity, (With<ChildOf>, With<GameSerializable>)>,
    mut commands: Commands,
) {
    for child in children {
        commands.entity(child).remove::<ChildOf>();
    }

    commands.entity(temp_scene.into_inner()).despawn();
}

pub fn remove_level_entities(
    mut commands: Commands,
    level_entities: Query<Entity, With<GameSerializable>>,
) {
    level_entities.iter().for_each(|entity| {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands
                .try_remove::<crate::game::trigger::Triggered>() // Hacky fix! Despawning these entities is annoying
                .despawn();

            // info!("Despawned {}", entity_commands.id())
        }
    });
}

fn serializable_components_type_registry() -> TypeRegistry {
    let mut registry = TypeRegistry::new();

    registry.register::<(InternalSerializableTypes, ExternalSerializableTypes)>();

    registry
}
