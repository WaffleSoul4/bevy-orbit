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
    Static,
}

// So for clarity, if object a is a member of layer x with filters y and z,
// objects with membership y and z will affect it

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Gravitable;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Gravitator;

pub fn apply_gravity(
    gravitator: Query<(&Mass, &Transform, Option<&GravityLayers>), With<Gravitator>>,
    mut gravitated: Query<
        (
            &Mass,
            &Transform,
            &mut LinearVelocity,
            Option<&GravityLayers>,
        ),
        With<Gravitable>,
    >,
    time: Res<Time>,
) {
    for (mass, transform, gravity_layers) in &gravitator {
        for (gravitated_mass, gravitated_transform, mut velocity, gravitated_gravity_layers) in
            &mut gravitated
        {
            let default_gravity_layer = GravityLayers::default();
            let gravitator_layers = gravity_layers.unwrap_or(&default_gravity_layer);
            let gravitated_layers = gravitated_gravity_layers.unwrap_or(&default_gravity_layer);

            if gravitator_layers.interacts_with(*gravitated_layers) {
                let diff_vector =
                    transform.translation.xy() - gravitated_transform.translation.xy();

                let dist = diff_vector.length();

                // This is still necessary for some reason
                // It throws quite the ambigous error when not included
                // Something along the lines of:
                // "The given sine and cosine produce an invalid rotation"
                // Probably has to do with normalization, might look into it

                if dist > 0.01 {
                    velocity.0 += diff_vector.normalize()
                        * (gravitated_mass.0 * mass.0 / dist.powi(2))
                        * 10000.0
                        * GRAVITATIONAL_CONSTANT
                        * time.delta_secs()
                }
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
