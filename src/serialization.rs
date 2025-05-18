use bevy::prelude::*;
use std::{fmt::Display, fs::File, path::PathBuf};

pub struct SerializationPlugin;

impl Plugin for SerializationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SerializeableTypeRegistrationPlugin)
            .add_event::<SaveEvent>()
            .add_event::<LoadEvent>()
            .add_systems(
                Update,
                (
                    initialize_colliders,
                    initialize_textures,
                    serialize_objects,
                    deserialize_objects,
                ),
            );
    }
}

// Put types that need to be serialized in here to add them to the registry
pub struct SerializeableTypeRegistrationPlugin;

impl Plugin for SerializeableTypeRegistrationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<crate::gravity::Gravitable>()
            .register_type::<crate::gravity::Gravitator>()
            .register_type::<crate::gravity::GravityLayers>()
            .register_type::<crate::Trigger>()
            .register_type::<crate::StaticObject>()
            .register_type::<crate::DynamicObject>()
            .register_type::<SerializableCollider>()
            .register_type::<SerializableAsset>();
    }
}

#[derive(Component)]
pub struct GameSerializable;

fn serialize_objects(
    mut events: EventReader<SaveEvent>,
    world: &World,
    entities: Query<Entity, With<GameSerializable>>,
    type_registry_mutex: Res<AppTypeRegistry>,
) {
    for event in events.read() {
        let scene_builder = DynamicSceneBuilder::from_world(world)
            .deny_all()
            // Internal types
            .allow_component::<crate::gravity::Gravitable>()
            .allow_component::<crate::gravity::Gravitator>()
            .allow_component::<crate::gravity::GravityLayers>()
            .allow_component::<crate::Trigger>()
            .allow_component::<crate::StaticObject>()
            .allow_component::<crate::DynamicObject>()
            .allow_component::<SerializableCollider>()
            .allow_component::<SerializableAsset>()
            // External types
            .allow_component::<Transform>()
            .allow_component::<avian2d::prelude::Mass>();

        let scene = scene_builder.extract_entities(entities.iter()).build();

        let type_registry = type_registry_mutex.read();

        let serialized = scene.serialize(&type_registry).unwrap_or_else(|err| {
            error!("Failed to serialize scene: {err}");
            String::new()
        });

        use std::io::Write;

        let target_path = event.path.clone();

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

fn deserialize_objects(
    mut save_events: EventReader<LoadEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for event in save_events.read() {
        commands.spawn(DynamicSceneRoot(asset_server.load(event.path.clone())));
    }
}

// This is just a resilient version of ColliderConstructor
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SerializableCollider(avian2d::prelude::ColliderConstructor);

impl SerializableCollider {
    pub fn new(collider: avian2d::prelude::ColliderConstructor) -> Self {
        SerializableCollider(collider)
    }
}

// I don't think it's possible to use data from inside the component when registering required components
pub fn initialize_colliders(
    colliders: Query<
        (&SerializableCollider, Entity),
        (
            Without<avian2d::prelude::Collider>,
            Without<avian2d::prelude::ColliderConstructor>,
        ),
    >,
    mut commands: Commands,
) {
    colliders
        .iter()
        .for_each(|(serializable_collider, entity)| {
            info!("Initializing collider");

            commands
                .entity(entity)
                .insert(serializable_collider.0.clone());
        });
}

// Events for saving and loading
#[derive(Event)]
pub struct SaveEvent {
    path: PathBuf,
    level_name: String,
}

impl SaveEvent {
    pub fn new<T: Display, U: Into<PathBuf>>(path: U, level_name: T) -> Self {
        SaveEvent {
            path: path.into(),
            level_name: level_name.to_string(),
        }
    }
}

#[derive(Event)]
pub struct LoadEvent {
    path: PathBuf,
}

impl LoadEvent {
    pub fn new<U: Into<PathBuf>>(path: U) -> Self {
        LoadEvent { path: path.into() }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub enum SerializableAsset{
    Sprite { path: PathBuf },
    // Using meshes will cause a panic on deserialization
    Mesh { mesh: Mesh },
    // This is preferable to mesh
    Primitive { shape: SerializableMeshPrimitives },
}

#[derive(Clone, Reflect)]
pub enum SerializableMeshPrimitives {
    Circle(Circle), // This is the only one we need for now
}

impl From<Circle> for SerializableMeshPrimitives {
    fn from(value: Circle) -> Self {
        SerializableMeshPrimitives::Circle(value)
    }
}

impl Into<Mesh> for SerializableMeshPrimitives {
    fn into(self) -> Mesh {
        match self {
            SerializableMeshPrimitives::Circle(circle) => circle.mesh().build()
        }
    }
}

impl SerializableAsset {
    pub fn sprite<T: Into<PathBuf>>(path: T) -> Self {
        SerializableAsset::Sprite { path: path.into() }
    }

    pub fn mesh<T: Into<Mesh>>(mesh: T) -> Self {
        SerializableAsset::Mesh { mesh: mesh.into() }
    }

    pub fn primitive<T: Into<SerializableMeshPrimitives>>(shape: T) -> Self {
        SerializableAsset::Primitive { shape: shape.into() }
    }
}

// NOTE: Directly using meshes causes deserialization to fail because of a divide by zero
// Somehow, somewhere, somebody sets the one of the mesh vetex buffer layouts'
// size to zero, which causes a failure in a division when allocating memory

fn initialize_textures(
    assets: Query<(&SerializableAsset, Entity), (Without<Mesh2d>,)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    assets.iter().for_each(|(asset, entity)| {
        info!("Initializing asset");

        let mut entity_commands = commands.entity(entity);

        match asset {
            SerializableAsset::Sprite { path } => {
                entity_commands.insert(Sprite::from_image(asset_server.load(path.clone())))
            }
            SerializableAsset::Mesh { mesh } => {
                entity_commands.insert(Mesh2d(meshes.add(mesh.clone())))
            }
            SerializableAsset::Primitive { shape } => {
                entity_commands.insert(Mesh2d(meshes.add(shape.clone())))
            }
        };
    });
}
