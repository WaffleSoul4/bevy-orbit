use avian2d::{math::*, prelude::*};
use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
};

const GRAVITATIONAL_CONSTANT: f32 = 1.0;
const CAMERA_MOVE_SPEED: f32 = 5.0;

#[derive(Component, Debug)]
struct Mass(f32);

#[derive(Component)]
struct Gravitable;

#[derive(Component)]
struct Gravitator;

#[derive(Resource)]
struct DebugInfo(bool);

#[derive(Default, Reflect, GizmoConfigGroup)]
struct ArrowGizmos;

#[derive(Resource)]
struct MousePos(Vec2);

#[derive(Component)]
struct Selected;

#[derive(Component)]
struct Trigger {
    pub state: bool,
}

#[derive(Component)]
struct GameCamera;

impl Trigger {
    fn new(state: bool) -> Self {
        Trigger { state }
    }

    fn trigger(&mut self) {
        self.state = true;
    }

    fn reset(&mut self) {
        self.state = false;
    }
}

#[derive(PhysicsLayer, Default)]
enum Layer {
    #[default]
    Main,
    Triggers,
}

#[derive(Component)]
struct CameraTrackable; // Oh god the naming is getting worse

#[derive(Component)]
struct TriggerIndicator;

#[derive(Resource, PartialEq)]
enum GameState {
    Editor,
    Play,
}

fn main() {
    App::new()
        .init_gizmo_group::<ArrowGizmos>()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                apply_gravity,
                mouse_tracker_sys,
                draw_velocity_arrows,
                mouse_input,
                toggle_arrows,
                toggle_gamestate,
                release_object,
                draw_selected_velocity_arrows,
                create_trigger,
                update_triggers,
                move_camera,
                zoom_camera,
                mouse_panning,
                move_camera_around_main_object,
                draw_grid,
            ),
        )
        .add_systems(PostUpdate, (clear_level, reset_level))
        .run();
}

fn setup(
    mut commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((Camera2d, GameCamera));

    commands.insert_resource(DebugInfo(true));
    commands.insert_resource(MousePos(Vec2::ZERO));
    commands.insert_resource(GameState::Editor);

    // Disable Avian Gravity
    commands.insert_resource(Gravity::ZERO);
}

fn toggle_arrows(keys: Res<ButtonInput<KeyCode>>, mut show_arrows: ResMut<DebugInfo>) {
    if keys.just_pressed(KeyCode::KeyQ) {
        show_arrows.0 = !show_arrows.0
    }
}

// I still don't know if seperating inputs like this is OK
// Should run in parralel(?)

fn toggle_gamestate(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<GameState>) {
    if keys.just_pressed(KeyCode::KeyE) {
        *state = match *state {
            GameState::Editor => GameState::Play,
            GameState::Play => GameState::Editor,
        }
    }
}

fn apply_gravity(
    gravitator: Query<(&Mass, &Transform), With<Gravitator>>,
    mut gravitated: Query<(&Mass, &Transform, &mut LinearVelocity), With<Gravitable>>,
    time: Res<Time>,
) {
    for (mass, transform) in &gravitator {
        for (gravitee_mass, gravitee_transform, mut velocity) in &mut gravitated {
            let diff_vector = transform.translation.xy() - gravitee_transform.translation.xy();

            let dist = diff_vector.length();

            if dist > 0.01 {
                velocity.0 += diff_vector.normalize()
                    * (gravitee_mass.0 * mass.0 / dist.powi(2))
                    * 10000.0
                    * GRAVITATIONAL_CONSTANT
                    * time.delta_secs()
            }
        }
    }
}

fn move_camera_around_main_object(
    state: Res<GameState>,
    main_object_query: Query<&Transform, (Without<GameCamera>, With<CameraTrackable>)>,
    mut camera_query: Query<&mut Transform, With<GameCamera>>,
    window_query: Query<&Window, Without<GameCamera>>,
) {
    if main_object_query.iter().count() != 0 && *state == GameState::Play {
        let main_object_translation = main_object_query
            .get_single()
            .expect("Multiple main objects detected")
            .translation;

        let mut camera_transform = camera_query
            .get_single_mut()
            .expect("Multiple game cameras detected");

        let window = window_query
            .get_single()
            .expect("Multiple windows detected");

        let window_bounding_box = bevy::math::bounding::Aabb2d::new(
            camera_transform.translation.xy(),
            Vec2::new(window.width() * 0.4, window.height() * 0.4),
        );

        let closest_point = window_bounding_box.closest_point(main_object_translation.xy());

        if closest_point == main_object_translation.xy() {
            // In the bounding box

            let dist_between = camera_transform.translation.xy() - closest_point;

            // Move 1/60th of the distance towards the obj

            update_xy(
                &mut camera_transform.translation,
                dbg!(-dist_between / 60.0),
            );
        } else {
            // Outside of bounding box

            let dist_between = -closest_point + main_object_translation.xy();

            // Instantly move to the closest point

            update_xy(&mut camera_transform.translation, dist_between);
        }

        println!("Camera translation {camera_transform:?}");
    }
}

fn update_xy(vec: &mut Vec3, xy: Vec2) {
    vec.x += xy.x;
    vec.y += xy.y;
}

fn move_camera(
    mut camera_query: Query<(&mut Transform, &OrthographicProjection), With<GameCamera>>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<GameState>,
) {
    if *state == GameState::Editor {
        let (mut transform, projection) = camera_query.single_mut();

        let camera_movement_speed = CAMERA_MOVE_SPEED * projection.scale;

        if keys.pressed(KeyCode::ArrowRight) {
            transform.translation.x += camera_movement_speed;
        }

        if keys.pressed(KeyCode::ArrowLeft) {
            transform.translation.x -= camera_movement_speed;
        }

        if keys.pressed(KeyCode::ArrowDown) {
            transform.translation.y -= camera_movement_speed;
        }

        if keys.pressed(KeyCode::ArrowUp) {
            transform.translation.y += camera_movement_speed;
        }
    }
}

fn zoom_camera(
    mut camera_query: Query<(&mut OrthographicProjection, &mut Transform), With<GameCamera>>,
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    state: Res<GameState>,
    mouse_pos: Res<MousePos>,
) {
    use bevy::input::mouse::MouseScrollUnit;

    if *state == GameState::Editor {
        let (mut projection, mut transform) = camera_query.single_mut();

        for event in scroll_events.read() {
            let mut zoom_modifier = 1.0;

            match event.unit {
                MouseScrollUnit::Line => {
                    if event.y <= -1.0 {
                        projection.scale *= 1.1;
                        zoom_modifier = -1.0;
                    } else if event.y >= 1.0 {
                        projection.scale /= 1.1;
                    }

                    let dif_vec = -transform.translation
                        + Vec3::new(mouse_pos.0.x, mouse_pos.0.y, 0.0) * zoom_modifier;

                    transform.translation += dif_vec / 10.0;
                }
                MouseScrollUnit::Pixel => {
                    println!(
                        "Scroll (pixel units): vertical: {}, horizontal: {}",
                        event.y, event.x
                    );
                    todo!()
                }
            }
        }
    }
}

fn update_triggers(
    mut trigger_query: Query<(Entity, &mut Trigger, &Transform), With<Collider>>,
    collisions: Res<Collisions>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let _ = trigger_query
        .iter_mut()
        .filter(|x| {
            collisions
                .iter()
                .any(|y| y.entity1 == x.0 || y.entity2 == x.0) // Check if the trigger is among the collisions
        })
        .for_each(|(_, ref mut t, transform)| {
            if !t.state {
                t.trigger();

                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(12.0))),
                    Transform {
                        translation: Vec3 {
                            x: transform.translation.x,
                            y: transform.translation.y,
                            z: transform.translation.z - 1.0,
                        },
                        ..transform.clone()
                    },
                    MeshMaterial2d(materials.add(Color::srgb(0.1, 0.7, 0.3))),
                    TriggerIndicator,
                ));
            }
        });
}

fn mouse_tracker_sys(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut mouse_pos: ResMut<MousePos>,
) {
    let window = windows.single();
    let (camera, camera_transform) = camera_query.single();

    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    {
        *mouse_pos = MousePos(Vec2::new(world_position.x, world_position.y))
    }
}

fn create_trigger(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_pos: Res<MousePos>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if keys.just_pressed(KeyCode::KeyZ) {
        commands.spawn((
            Mesh2d(meshes.add(Circle::new(10.0))),
            Transform {
                translation: Vec3 {
                    x: mouse_pos.0.x,
                    y: mouse_pos.0.y,
                    z: -1.0,
                },
                ..default()
            },
            MeshMaterial2d(materials.add(Color::srgb(0.1, 0.3, 0.7))),
            Trigger::new(false),
            Collider::circle(10.0),
            CollisionLayers::new(Layer::Triggers, [Layer::Main]),
        ));
    }
}

fn mouse_input(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<MousePos>,
    state: Res<GameState>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if mouse_input.just_pressed(MouseButton::Right) {
        create_static(&mut commands, &mut meshes, &mut materials, mouse_pos.0);
    }

    if mouse_input.just_pressed(MouseButton::Left) && *state == GameState::Play {
        commands.spawn((
            Mesh2d(meshes.add(Circle::new(10.0))),
            Transform {
                translation: Vec3 {
                    x: mouse_pos.0.x,
                    y: mouse_pos.0.y,
                    ..default()
                },
                ..default()
            },
            MeshMaterial2d(materials.add(Color::oklab(1.0, 0.7, 0.3))),
            Selected,
        ));
    }
}

fn mouse_panning(
    state: Res<GameState>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_reader: EventReader<MouseMotion>,
    mut camera_query: Query<(&mut Transform, &OrthographicProjection), With<GameCamera>>,
) {
    if *state == GameState::Editor && mouse_input.pressed(MouseButton::Left) {
        let (mut camera_transform, projection) = camera_query.single_mut();

        for mouse_motion in mouse_motion_reader.read() {
            update_xy(
                &mut camera_transform.translation,
                Vec2::new(-mouse_motion.delta.x, mouse_motion.delta.y) * projection.scale,
            );
        }
    }
}

fn release_object(
    selected_query: Query<(Entity, &Transform), With<Selected>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<MousePos>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if mouse_input.just_released(MouseButton::Left) {
        for selected in selected_query.iter() {
            let dif = -(mouse_pos.0 - selected.1.translation.xy());

            let mut entity_commands = commands.entity(selected.0);

            entity_commands
                .insert((
                    Mass(10.0),
                    Gravitable,
                    Gravitator,
                    LinearVelocity(dif),
                    Collider::circle(10.0 as Scalar),
                    RigidBody::Dynamic,
                ))
                .remove::<Selected>();

            if keys.pressed(KeyCode::ShiftLeft) {
                entity_commands.insert(CameraTrackable);
            }
        }
    }
}

fn clear_level(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    query: Query<Entity, With<Mesh2d>>,
) {
    if keys.just_pressed(KeyCode::Space) && keys.pressed(KeyCode::ShiftLeft) {
        query
            .iter()
            .for_each(|x| commands.get_entity(x).unwrap().despawn())
    }
}

fn reset_level(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    remove_query: Query<Entity, Or<(With<Gravitable>, With<TriggerIndicator>)>>,
    mut trigger_query: Query<&mut Trigger>,
) {
    if keys.just_pressed(KeyCode::Space) && !keys.pressed(KeyCode::ShiftLeft) {
        remove_query
            .iter()
            .for_each(|x| commands.get_entity(x).unwrap().despawn());

        trigger_query.iter_mut().for_each(|ref mut x| x.reset());
    }
}

fn create_static(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    pos: Vec2,
) {
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(10.0))),
        Transform {
            translation: Vec3 {
                x: pos.x,
                y: pos.y,
                ..default()
            },
            ..default()
        },
        MeshMaterial2d(materials.add(Color::oklab(1.0, 0.7, 0.3))),
        Mass(10.0),
        Gravitator,
        Collider::circle(10.0 as Scalar),
        RigidBody::Static,
    ));
}

fn draw_velocity_arrows(
    show_arrows: Res<DebugInfo>,
    mut gizmos: Gizmos,
    query: Query<(&LinearVelocity, &Transform)>,
) {
    if show_arrows.0 {
        query.iter().for_each(|(velocity, transform)| {
            gizmos.arrow_2d(
                transform.translation.xy(),
                transform.translation.xy() + velocity.0.xy() / 6.0,
                Color::srgb(0.1, 0.4, 0.6),
            );
        })
    }
}

fn draw_selected_velocity_arrows(
    debug_info: Res<DebugInfo>,
    mouse_pos: Res<MousePos>,
    mut gizmos: Gizmos,
    transform_query: Query<&Transform, With<Selected>>,
) {
    if debug_info.0 {
        transform_query.iter().for_each(|transform| {
            let dif = -(mouse_pos.0 - transform.translation.xy());

            gizmos.arrow_2d(
                transform.translation.xy(),
                transform.translation.xy() + dif / 6.0,
                Color::srgb(0.1, 0.4, 0.6),
            );
        })
    }
}

fn draw_grid(
    debug_info: Res<DebugInfo>,
    mut gizmos: Gizmos,
    camera_query: Query<(&Transform, &OrthographicProjection), With<GameCamera>>,
) {
    if debug_info.0 {
        let (camera_transform, projection) = camera_query.single();

        let mut scale = projection.scale;

        let scale_jumps_iter = std::iter::successors(Some(0.00390625_f32), |x| Some(x * 2.0));

        for scale_jump in scale_jumps_iter {
            if scale_jump > scale {
                scale = scale_jump;
                break;
            }
        }

        let grid_spacing = scale * 64.0;

        let camera_xy = camera_transform.translation.xy();

        let closest_center = dbg!(camera_xy - (camera_xy % (grid_spacing * 2.0)));

        let x_width = 16_f32;
        let y_width = 16_f32;

        for x in -x_width as i32..x_width as i32 {
            let line_x: f32 = x as f32 * grid_spacing;
            let lightness = if dbg!(line_x % (grid_spacing * 2.0)) == 0.0 {
                0.2
            } else {
                0.05
            };
            gizmos.line_2d(
                Vec2::new(line_x as f32, y_width * grid_spacing) + closest_center,
                Vec2::new(line_x as f32, -y_width * grid_spacing) + closest_center,
                LinearRgba::gray(lightness),
            );
        }

        for y in -y_width as i32..y_width as i32 {
            let line_y: f32 = y as f32 * grid_spacing;

            let lightness = if line_y % (grid_spacing * 2.0) == 0.0 {
                0.2
            } else {
                0.05
            };

            gizmos.line_2d(
                Vec2::new(x_width * grid_spacing, line_y as f32) + closest_center,
                Vec2::new(-x_width * grid_spacing, line_y as f32) + closest_center,
                LinearRgba::gray(lightness),
            );
        }
    }
}
