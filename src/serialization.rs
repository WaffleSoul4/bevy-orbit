use avian2d::prelude::ColliderConstructor;
use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css::{RED, WHITE},
    gizmos,
    prelude::*,
    reflect::TypeRegistry,
    render::mesh::PrimitiveTopology,
};
use std::{fs::File, path::PathBuf};

use crate::{cursor::CursorPosition, game::KillOnCollision};

pub struct SerializationPlugin;

impl Plugin for SerializationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SerializeableTypeRegistrationPlugin)
            .add_event::<SaveEvent>()
            .insert_resource(LevelSerializationData::new("test_levels/level2.scn.ron"))
            .add_systems(
                Update,
                (
                    initialize_colliders,
                    initialize_meshes,
                    initialize_mesh_materials,
                    serialize_objects,
                    free_temp_scene_children,
                    // Show all collisions
                    // | collisions: avian2d::prelude::Collisions| collisions.iter().for_each(|collision| info!("{:?}", collision)),
                ),
            );
    }
}

type InternalSerializableTypes = (
    crate::gravity::Gravity,
    crate::gravity::GravityLayers,
    crate::game::GameTrigger,
    crate::game::KillOnCollision,
    crate::LevelObject,
    crate::DynamicObject,
    SerializableCollider,
    SerializableMesh,
    SerializableMeshPrimitives,
    SerilializableMeshMaterial,
    GameSerializable,
    StartPoint,
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
    }
}

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

pub fn load_active_level(
    level_serialization_data: Res<LevelSerializationData>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    info!(
        "Loaded level as scene from {:?}",
        level_serialization_data.path
    );

    commands.spawn((
        DynamicSceneRoot(asset_server.load(level_serialization_data.path.clone())),
        ActiveLevel,
    ));
}

pub fn remove_active_level(
    mut commands: Commands,
    active_level: Single<Entity, With<ActiveLevel>>,
) {
    commands.entity(active_level.into_inner()).despawn();
}

fn serializable_components_type_registry() -> TypeRegistry {
    let mut registry = TypeRegistry::new();

    registry.register::<(InternalSerializableTypes, ExternalSerializableTypes)>();

    registry
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct GameSerializable;

// Only for use in editor mode
fn serialize_objects(
    mut events: EventReader<SaveEvent>,
    world: &World,
    entities: Query<Entity, With<GameSerializable>>,
) {
    for event in events.read() {
        // Dynamic programming when
        let scene_builder = DynamicSceneBuilder::from_world(world)
            // Internal types
            .allow_component::<crate::gravity::Gravity>()
            .allow_component::<crate::gravity::GravityLayers>()
            .allow_component::<crate::game::GameTrigger>()
            .allow_component::<crate::game::KillOnCollision>()
            .allow_component::<crate::LevelObject>()
            .allow_component::<crate::DynamicObject>()
            .allow_component::<SerializableCollider>()
            .allow_component::<SerializableMesh>()
            .allow_component::<SerilializableMeshMaterial>()
            .allow_component::<GameSerializable>()
            // External types
            .allow_component::<Transform>()
            .allow_component::<avian2d::prelude::CollisionLayers>()
            .allow_component::<avian2d::prelude::Mass>()
            .allow_component::<avian2d::prelude::RigidBody>()
            // Resources
            .allow_resource::<StartPoint>();

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
    level_serialization_data: Res<LevelSerializationData>,
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
                .try_remove::<crate::game::Triggered>() // Hacky fix! Despawning these entities is annoying
                .despawn();

            // info!("Despawned {}", entity_commands.id())
        }
    });
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

impl From<Circle> for SerializableCollider {
    fn from(value: Circle) -> Self {
        SerializableCollider(avian2d::prelude::ColliderConstructor::Circle {
            radius: value.radius,
        })
    }
}

impl Into<ColliderConstructor> for SerializableZoneBuilder {
    fn into(self) -> ColliderConstructor {
        let mut indices: Vec<[u32; 3]> = vec![];

        for i in 0..=self.0.indices.len() / 3 - 1 {
            indices.push(self.0.indices[i * 3..i * 3 + 3].try_into().unwrap())
        }

        ColliderConstructor::Trimesh {
            vertices: self.0.vertices,
            indices,
        }
    }
}

#[derive(Component, Default, Clone)]
pub struct SerializableZoneBuilder(pub SerializableZone);

impl SerializableZoneBuilder {
    fn insert_point(&mut self, point: Vec2) {
        self.0.vertices.push(point);
    }

    fn remove_last(&mut self) {
        let len = self.0.vertices.len();

        self.0.vertices.remove(len - 1);
    }

    fn get_center(&self) -> Vec2 {
        let mut iterator = self.0.vertices.iter();

        iterator.next(); // Ignore the first thing 

        iterator.fold(Vec2::ZERO, |acc, pos| acc + pos) / (self.0.vertices.len() as f32 - 1.0)
    }

    // Always draws lines to well.. the center (hence the naive)
    // Am I figuring out how to correctly generate indices??? No!
    // If you're sad that this doesn't work perfectly, too bad!
    fn build_indicies_naive(mut self) -> (Self, Vec2) {
        let center = self.get_center();

        self.0.vertices[0] = center;

        self.0
            .vertices
            .iter_mut()
            .for_each(|vertice| *vertice -= center);

        for i in 1..(self.0.vertices.len() - 1) as u32 {
            self.0.indices.push(0);
            self.0.indices.push(i);
            self.0.indices.push(i + 1);
        }

        self.0.indices.push(0);
        self.0.indices.push(self.0.vertices.len() as u32 - 1);
        self.0.indices.push(1);

        (self, center)
    }
}

// Bevy why do I need another type for this help meeeee
struct SerializableZoneMeshBuilder(SerializableZone);

impl MeshBuilder for SerializableZoneMeshBuilder {
    fn build(&self) -> Mesh {
        if self.0.vertices.len() <= 2 {
            panic!("Not enough vertices to form a zone")
        }

        if self.0.indices.len() <= 2 {
            panic!("Not enough indices to form a zone")
        }

        let normals = vec![[0.0, 0.0, 1.0]; self.0.vertices.len()];

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            self.0
                .vertices
                .iter()
                .map(|vertice| vertice.extend(0.0))
                .collect::<Vec<Vec3>>()
                .clone(),
        )
        .with_inserted_indices(bevy::render::mesh::Indices::U32(self.0.indices.clone()))
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    }
}

impl Into<SerializableZone> for SerializableZoneBuilder {
    fn into(self) -> SerializableZone {
        self.0
    }
}

pub fn convert_zone_builders(
    mut commands: Commands,
    builders: Query<(Entity, &SerializableZoneBuilder)>,
) {
    builders.iter().for_each(|(entity, builder)| {
        let mut entity_commands = commands.entity(entity);

        let (zone, center) = builder.clone().build_indicies_naive();
        entity_commands
            .insert((
                SerializableMesh::zone(zone.clone()),
                SerializableCollider::new(zone.clone().into()),
                Transform::from_translation(center.extend(-1.0)),
                SerilializableMeshMaterial::color(Color::srgba(0.8, 0.1, 0.3, 0.3)),
                GameSerializable,
                KillOnCollision,
            ))
            .remove::<SerializableZoneBuilder>();
    })
}

pub fn initialize_zone_builder(mut commands: Commands) {
    commands.spawn(SerializableZoneBuilder::default());
}

pub fn zone_creation_input_handler(
    mut zone_builder: Single<&mut SerializableZoneBuilder>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_pos: Res<CursorPosition>,
    mut _commands: Commands,
) {
    if let Some(cursor_pos) = **cursor_pos {
        if mouse.just_pressed(MouseButton::Left) {
            zone_builder.insert_point(cursor_pos);
        }

        if keys.just_pressed(KeyCode::Space) {
            zone_builder.0.vertices.truncate(1);
        }

        if mouse.just_pressed(MouseButton::Back) {
            zone_builder.remove_last();
        }
    }
}

pub fn zone_creation_outline_gizmos(
    mut gizmos: Gizmos,
    zones: Query<&SerializableZoneBuilder>,
    cursor_pos: Res<CursorPosition>,
) {
    zones
        .iter()
        .filter(|SerializableZoneBuilder(SerializableZone { vertices, .. })| vertices.len() > 1)
        .map(|SerializableZoneBuilder(SerializableZone { vertices, .. })| vertices[1..].to_owned())
        .for_each(|mut vertices| {
            if let Some(cursor_pos) = **cursor_pos {
                vertices.push(cursor_pos)
            }

            vertices.iter().for_each(|vertice| {
                gizmos.rect_2d(
                    Isometry2d::from_translation(*vertice),
                    Vec2::new(5.0, 5.0),
                    Color::srgb(1.0, 0.0, 0.2),
                )
            });

            if vertices.len() > 1 {
                for i in 0..=vertices.len() - 2 {
                    gizmos.line_2d(vertices[i], vertices[i + 1], Color::srgb(1.0, 0.0, 0.2))
                }
            }
        });
}

#[derive(Reflect, Clone)]
pub struct SerializableZone {
    vertices: Vec<Vec2>,
    // What order the points should be drawn in
    indices: Vec<u32>,
}

impl Default for SerializableZone {
    fn default() -> Self {
        SerializableZone {
            vertices: vec![Vec2::ZERO],
            indices: vec![],
        }
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
            // info!("Initializing collider");

            commands
                .entity(entity)
                .insert(serializable_collider.0.clone());
        });
}

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

#[derive(Component, Reflect)]
#[reflect(Component)]
pub enum SerializableMesh {
    Sprite { path: PathBuf },
    // Directly using meshes will cause a panic on deserialization
    Mesh { mesh: Mesh },
    // This is preferable to mesh
    Primitive { shape: SerializableMeshPrimitives },
    Zone { zone: SerializableZone },
}

#[derive(Clone, Reflect)]
pub enum SerializableMeshPrimitives {
    Circle(Circle),
}

impl From<Circle> for SerializableMeshPrimitives {
    fn from(value: Circle) -> Self {
        SerializableMeshPrimitives::Circle(value)
    }
}

impl Into<Mesh> for SerializableMeshPrimitives {
    fn into(self) -> Mesh {
        match self {
            SerializableMeshPrimitives::Circle(circle) => circle.mesh().build(),
        }
    }
}

impl SerializableMesh {
    pub fn sprite<T: Into<PathBuf>>(path: T) -> Self {
        SerializableMesh::Sprite { path: path.into() }
    }

    pub fn mesh<T: Into<Mesh>>(mesh: T) -> Self {
        SerializableMesh::Mesh { mesh: mesh.into() }
    }

    pub fn primitive<T: Into<SerializableMeshPrimitives>>(shape: T) -> Self {
        SerializableMesh::Primitive {
            shape: shape.into(),
        }
    }

    pub fn zone<T: Into<SerializableZone>>(zone: T) -> Self {
        SerializableMesh::Zone { zone: zone.into() }
    }
}

// NOTE: Directly using meshes causes deserialization to fail because of a divide by zero
// Somehow, somewhere, somebody sets the one of the mesh vertex buffer layouts'
// size to zero, which causes a failure in a division when allocating memory

fn initialize_meshes(
    serializable_meshes: Query<(&SerializableMesh, Entity), Without<Mesh2d>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    serializable_meshes.iter().for_each(|(mesh, entity)| {
        // info!("Initializing asset");

        let mut entity_commands = commands.entity(entity);

        match mesh {
            SerializableMesh::Sprite { path } => {
                entity_commands.insert(Sprite::from_image(asset_server.load(path.clone())))
            }
            SerializableMesh::Mesh { mesh } => {
                entity_commands.insert(Mesh2d(meshes.add(mesh.clone())))
            }
            SerializableMesh::Primitive { shape } => {
                entity_commands.insert(Mesh2d(meshes.add(shape.clone())))
            }
            SerializableMesh::Zone { zone } => {
                let mesh: Mesh = SerializableZoneMeshBuilder(zone.clone()).build();

                entity_commands.insert(Mesh2d(meshes.add(mesh)))
            }
        };
    });
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub enum SerilializableMeshMaterial {
    Color(ColorMaterial),
}

impl From<ColorMaterial> for SerilializableMeshMaterial {
    fn from(value: ColorMaterial) -> Self {
        SerilializableMeshMaterial::Color(value)
    }
}

impl Into<ColorMaterial> for SerilializableMeshMaterial {
    fn into(self) -> ColorMaterial {
        match self {
            SerilializableMeshMaterial::Color(color_material) => color_material,
        }
    }
}

impl SerilializableMeshMaterial {
    pub fn color<T: Into<ColorMaterial>>(color: T) -> Self {
        SerilializableMeshMaterial::Color(color.into())
    }
}

fn initialize_mesh_materials(
    serializable_materials: Query<
        (&SerilializableMeshMaterial, Entity),
        Without<MeshMaterial2d<ColorMaterial>>,
    >,
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    serializable_materials
        .iter()
        .for_each(|(material, entity)| {
            // info!("Initializing material");

            let mut entity_commands = commands.entity(entity);

            match material {
                SerilializableMeshMaterial::Color(color_material) => {
                    entity_commands.insert(MeshMaterial2d(materials.add(color_material.clone())))
                }
            };
        });
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
        let start_point_color = WHITE;

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
