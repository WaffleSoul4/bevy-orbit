use bevy::prelude::*;

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

pub fn initialize_mesh_materials(
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
