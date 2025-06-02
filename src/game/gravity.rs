use avian2d::prelude::*;
use bevy::prelude::*;

pub struct GravityPlugin;

impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_gravity);
    }
}

const GRAVITATIONAL_CONSTANT: f32 = 1.0;

// I guess I'll re-use the [PhysicsLayer] trait instead of
// copy pasting more code
#[derive(PhysicsLayer, Default, Copy, Clone)]
pub enum GravityLayer {
    #[default]
    Main,
    Level,
}

// So for clarity, if object a is a member of layer x with filters y and z,
// objects with membership y and z will affect it

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Gravity;

pub fn apply_gravity(
    mut gravity_objects: Query<
        (
            &Mass,
            &Transform,
            &mut LinearVelocity,
            Option<&GravityLayers>,
        ),
        With<Gravity>,
    >,
    time: Res<Time>,
) {
    let mut combinations = gravity_objects.iter_combinations_mut::<2>();

    while let Some(
        [
            (mass1, transform1, mut velocity1, gravity_layer_1),
            (mass2, transform2, mut velocity2, gravity_layer_2),
        ],
    ) = combinations.fetch_next()
    {
        let layers_1 = gravity_layer_1.unwrap_or(&GravityLayers::DEFAULT);
        let layers_2 = gravity_layer_2.unwrap_or(&GravityLayers::DEFAULT);

        // If layer one applies gravity to layer two
        if layers_1.interacts_with(*layers_2) {
            let diff_vector = transform1.translation.xy() - transform2.translation.xy();

            let dist = diff_vector.length();

            if dist > 0.01 {
                velocity2.0 += diff_vector.normalize()
                    * (mass1.0 * mass2.0 / dist.powi(2))
                    * 10000.0
                    * GRAVITATIONAL_CONSTANT
                    * time.delta_secs()
            }
        }

        // If layer two applies gravity to layer one
        if layers_2.interacts_with(*layers_1) {
            let diff_vector = transform2.translation.xy() - transform1.translation.xy();

            let dist = diff_vector.length();

            if dist > 0.01 {
                velocity1.0 += diff_vector.normalize()
                    * (mass1.0 * mass2.0 / dist.powi(2))
                    * 10000.0
                    * GRAVITATIONAL_CONSTANT
                    * time.delta_secs()
            }
        }
    }
}

// Everything below here isn't fully my code
// It's just refactored from https://github.com/Jondolf/avian/blob/main/src/collision/collider/layers.rs
// I used a few hacky workarounds which were probably
// unnecessary, but got the job done. I don't think there
// is a better way to create custom layers besides
// copy pasting and refactoring code from avian.

use avian2d::prelude::LayerMask;

#[derive(bevy::prelude::Component, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct GravityLayers {
    pub memberships: LayerMask,

    pub filters: LayerMask,
}

#[allow(dead_code)]
impl GravityLayers {
    pub const DEFAULT: Self = Self {
        memberships: LayerMask::DEFAULT,

        filters: LayerMask::ALL,
    };

    pub const ALL: Self = Self {
        memberships: LayerMask::ALL,

        filters: LayerMask::ALL,
    };

    pub const NONE: Self = Self {
        memberships: LayerMask::NONE,

        filters: LayerMask::NONE,
    };

    pub const ALL_MEMBERSHIPS: Self = Self {
        memberships: LayerMask::ALL,

        filters: LayerMask::NONE,
    };

    pub const ALL_FILTERS: Self = Self {
        memberships: LayerMask::NONE,

        filters: LayerMask::ALL,
    };

    pub fn new(memberships: impl Into<LayerMask>, filters: impl Into<LayerMask>) -> Self {
        Self {
            memberships: memberships.into(),

            filters: filters.into(),
        }
    }

    pub const fn from_bits(memberships: u32, filters: u32) -> Self {
        Self {
            memberships: LayerMask(memberships),

            filters: LayerMask(filters),
        }
    }

    pub fn interacts_with(self, other: Self) -> bool {
        (self.memberships & other.filters) != LayerMask::NONE
            && (other.memberships & self.filters) != LayerMask::NONE
    }
}

impl Default for GravityLayers {
    fn default() -> Self {
        GravityLayers::DEFAULT
    }
}
