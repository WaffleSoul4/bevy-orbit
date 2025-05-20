use crate::cursor;
use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, initialize_camera).add_systems(
            Update,
            (
                pan_camera_keys,
                pan_camera_mouse,
                zoom_camera,
                apply_camera_velocity,
                ease_camera_velocity,
            ),
        );
    }
}

fn initialize_camera(mut commands: Commands) {
    commands.spawn((GameCamera, CameraVelocity(Vec2::ZERO), Camera2d));
}

const CAMERA_PAN_SPEED: f32 = 10.0;

/// Marker trait for the game camera
#[derive(Component)]
pub struct GameCamera;

// LinearVelocity on the camera can act a little wierdly
#[derive(Component, Deref, DerefMut)]
pub struct CameraVelocity(Vec2);

fn pan_camera_keys(
    camera_query: Single<(&mut Transform, &Projection), With<GameCamera>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    use KeyCode::*;

    let (mut transform, projection) = camera_query.into_inner();

    match projection {
        Projection::Orthographic(projection) => {
            let camera_movement_speed = CAMERA_PAN_SPEED * projection.scale;

            if keys.any_pressed([ArrowRight, KeyD]) {
                transform.translation.x += camera_movement_speed;
            }

            if keys.any_pressed([ArrowLeft, KeyA]) {
                transform.translation.x -= camera_movement_speed;
            }

            if keys.any_pressed([ArrowDown, KeyS]) {
                transform.translation.y -= camera_movement_speed;
            }

            if keys.any_pressed([ArrowUp, KeyW]) {
                transform.translation.y += camera_movement_speed;
            }
        }
        _ => unimplemented!(),
    };
}

fn zoom_camera(
    camera_query: Single<(&mut Projection, &mut Transform), With<GameCamera>>,
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    cursor_pos: Res<cursor::CursorPosition>,
) {
    use bevy::input::mouse::MouseScrollUnit;

    match *cursor_pos {
        cursor::CursorPosition(Some(cursor_pos)) => {
            let (mut projection, mut transform) = camera_query.into_inner();

            match projection.as_mut() {
                Projection::Orthographic(projection) => {
                    for event in scroll_events.read() {
                        let mut zoom_modifier = 1.0;

                        match event.unit {
                            MouseScrollUnit::Line => {
                                if event.y <= -1.0 {
                                    projection.scale *= 1.1;
                                    zoom_modifier = -1.1;
                                } else if event.y >= 1.0 {
                                    projection.scale /= 1.1;
                                }

                                let dif_vec = -transform.translation + cursor_pos.extend(0.0);

                                transform.translation += (dif_vec / 10.0) * zoom_modifier;
                            }
                            MouseScrollUnit::Pixel => todo!(),
                        }
                    }
                }
                _ => unimplemented!(),
            };
        }
        _ => {}
    }
}

// Using the window data to defie the viewport doesn't support changes to the window
// This caused an error where the viewport wouldn't update when the window was resized
pub fn restore_viewport(mut camera: Single<&mut Camera, With<GameCamera>>) {
    camera.viewport = None;
}

fn pan_camera_mouse(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor_position: Res<crate::cursor::CursorPosition>,
    cursor_motions: Res<crate::cursor::CursorMotions>,
    camera_query: Single<(&mut Transform, &Projection, &mut CameraVelocity), With<GameCamera>>,
    mut cursor_lock_position: Local<Option<Vec2>>,
) {
    use KeyCode::*;
    use MouseButton::*;

    match **cursor_position {
        Some(cursor_position) => {
            let (mut transform, projection, mut velocity) = camera_query.into_inner();

            if mouse.pressed(Left) && keys.pressed(ShiftLeft) {
                match *cursor_lock_position {
                    Some(lock_position) => {
                        transform.translation += (lock_position - cursor_position).extend(0.0)
                    }
                    None => *cursor_lock_position = Some(cursor_position),
                }
            }
        }
        None => {}
    }

    // @TODO: Add exit velocity again
}

fn apply_camera_velocity(
    camera_query: Single<(&mut Transform, &CameraVelocity), With<GameCamera>>,
) {
    let (mut transform, velocity) = camera_query.into_inner();

    transform.translation += Vec3::new(velocity.x, velocity.y, 0.0);
}

fn ease_camera_velocity(mut camera: Single<&mut CameraVelocity, With<GameCamera>>) {
    camera.0 *= 0.95
}
