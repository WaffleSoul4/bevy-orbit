use avian2d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    input::common_conditions::{input_just_pressed, input_just_released, input_pressed},
    prelude::*,
    render::mesh::PrimitiveTopology,
};

use crate::{AppState, helper::app_state_is};

// So much stuff I want to create a plugin for it!

pub struct ZonePlugin;

impl Plugin for ZonePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (
                    zone_creation_input_handler.run_if(input_pressed(KeyCode::ControlLeft)),
                    initialize_zone_builder.run_if(input_just_pressed(KeyCode::ControlLeft)),
                    convert_zone_builders.run_if(input_just_released(KeyCode::ControlLeft)),
                    zone_creation_outline_gizmos,
                )
                    .run_if(app_state_is(AppState::Editor)),
                convert_zone_builders.run_if(app_state_is(AppState::Play)),
            ),
        );
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
pub struct SerializableZoneMeshBuilder(SerializableZone);

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
                super::meshes::SerializableMesh::zone(zone.clone()),
                super::colliders::SerializableCollider::new(zone.clone().into()),
                Transform::from_translation(center.extend(-1.0)),
                super::materials::SerilializableMeshMaterial::color(Color::srgba(
                    0.8, 0.1, 0.3, 0.3,
                )),
                super::GameSerializable,
                crate::game::death::KillOnCollision,
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
    cursor_pos: Res<crate::cursor::CursorPosition>,
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
    cursor_pos: Res<crate::cursor::CursorPosition>,
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

impl Into<SerializableZoneMeshBuilder> for SerializableZone {
    fn into(self) -> SerializableZoneMeshBuilder {
        SerializableZoneMeshBuilder(self)
    }
}
