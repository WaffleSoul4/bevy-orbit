// This isn't really my code...
// It's just refactored from https://github.com/Jondolf/avian/blob/main/src/collision/collider/layers.rs
// I used a few hacky workarounds which were probably
// unnecessary, but got the job done. I don't think there
// is a better way to create custom layers besides
// copy pasting and refactoring code from avian.
//
// Could make a pr addressing this


use avian2d::prelude::LayerMask;

#[derive(bevy::prelude::Component, Clone, Copy)]
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
