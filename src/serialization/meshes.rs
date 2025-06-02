use bevy::prelude::*;
use std::path::PathBuf;

use super::zones::SerializableZoneMeshBuilder;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub enum SerializableMesh {
    Sprite {
        path: PathBuf,
    },
    // Directly using meshes will cause a panic on deserialization
    Mesh {
        mesh: Mesh,
    },
    // This is preferable to mesh
    Primitive {
        shape: SerializableMeshPrimitives,
    },
    Zone {
        zone: super::zones::SerializableZone,
    },
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

    pub fn zone<T: Into<super::zones::SerializableZone>>(zone: T) -> Self {
        SerializableMesh::Zone { zone: zone.into() }
    }
}

// NOTE: Directly using meshes causes deserialization to fail because of a divide by zero
// Somehow, somewhere, somebody sets the one of the mesh vertex buffer layouts'
// size to zero, which causes a failure in a division when allocating memory

pub fn initialize_meshes(
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
                let mesh: SerializableZoneMeshBuilder = zone.clone().into();

                entity_commands.insert(Mesh2d(meshes.add(mesh)))
            }
        };
    });
}
