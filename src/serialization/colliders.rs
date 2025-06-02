use bevy::prelude::*;

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
