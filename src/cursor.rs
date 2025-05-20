use bevy::{input::mouse::MouseMotion, prelude::*};

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CursorPosition(None))
            .init_resource::<CursorMotions>()
            .add_systems(
                Update,
                (
                    update_cursor_motions,
                    get_cursor_position.pipe(update_resource::<CursorPosition, _>),
                ),
            );
    }
}

#[derive(Resource, Deref)]
pub struct CursorPosition(pub Option<Vec2>);

impl From<Option<Vec2>> for CursorPosition {
    fn from(value: Option<Vec2>) -> Self {
        CursorPosition(value)
    }
}

fn get_cursor_position(
    window: Single<&Window>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    let (camera, camera_transform) = camera_query.into_inner();

    window.cursor_position().and_then(|cursor| {
        let viewport_rect = camera.logical_viewport_rect()?;

        if viewport_rect.contains(cursor) {
            return camera.viewport_to_world_2d(camera_transform, cursor).ok();
        }

        return None;
    })
}

// This whole thing for adding camera velocity

#[derive(Resource, Debug)]
pub struct CursorMotions(std::collections::VecDeque<Vec2>);

impl Default for CursorMotions {
    fn default() -> Self {
        use std::collections::VecDeque;

        let mut past_mouse_motions: VecDeque<Vec2> = VecDeque::with_capacity(5);

        for _ in 0..5 {
            past_mouse_motions.push_back(Vec2::ZERO)
        }

        CursorMotions(past_mouse_motions)
    }
}

impl CursorMotions {
    pub fn update(&mut self, val: Vec2) {
        self.0.pop_front();
        self.0.push_back(val);
    }

    pub fn sum(&self) -> Vec2 {
        self.0.iter().sum()
    }
}

fn update_cursor_motions(
    mut motions: EventReader<MouseMotion>,
    mut cursor_motions: ResMut<CursorMotions>,
) {
    match motions.is_empty() {
        false => motions
            .read()
            .map(|motion| {
                let mut delta = motion.delta.clone();
                delta.x = -motion.delta.x;

                delta
            })
            .for_each(|motion| cursor_motions.update(motion)),
        true => cursor_motions.update(Vec2::ZERO),
    }
}

// This makes me feel like a real programmer
fn update_resource<Resource: bevy::prelude::Resource + From<Input>, Input>(
    In(value): In<Input>,
    mut resource: ResMut<Resource>,
) {
    *resource = Resource::from(value)
}
