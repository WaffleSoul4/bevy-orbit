use avian2d::{math::*, prelude::*};
use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_egui::EguiPlugin;
use debug::DebugPlugin;
use editor::{CreateObject, LoadEvent, SaveEvent, SelectedDynamicConfig};
use gravity::{Gravitable, Gravitator, GravityLayer, GravityLayers};
use std::{collections::VecDeque, path::PathBuf, str::FromStr};

mod debug;
mod editor;
mod gravity;
mod level;

const CAMERA_MOVE_SPEED: f32 = 10.0;

#[derive(Resource, Deref)]
struct MousePos(Vec2);

#[derive(Component)]
struct Selected;

#[derive(Component)]
struct DynamicObject;

#[derive(Component)]
struct StaticObject;

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
enum GameLayer {
    #[default]
    Main,
    Triggers,
}

#[derive(Resource, Debug)]
struct PastMouseMotions(VecDeque<Vec2>);

impl Default for PastMouseMotions {
    fn default() -> Self {
        let mut past_mouse_motions: VecDeque<Vec2> = VecDeque::new();
        for _ in 0..5 {
            past_mouse_motions.push_back(Vec2::ZERO)
        }
        PastMouseMotions(past_mouse_motions)
    }
}

// I had some errors with LinearVelocity, so this will do until I'm not lazy
#[derive(Component, Deref, DerefMut)]
struct CameraVelocity(Vec2);

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
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            EguiPlugin,
            DebugPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                /*move_camera_around_main_object,*/ release_selected,
                game_binds,
            )
                .run_if(|state: Res<GameState>| *state == GameState::Play),
        )
        .add_systems(
            Update,
            (
                gravity::apply_gravity,
                create_objects,
                mouse_tracker_sys,
                global_binds,
                toggle_gamestate,
                update_triggers,
                pan_camera_keys,
                zoom_camera,
                pan_camera_mouse,
                apply_camera_velocity,
                editor::save_level,
                editor::load_level,
            ),
        )
        .add_systems(PostUpdate, (clear_level, reset_level))
        .add_event::<CreateObject>()
        .add_event::<SaveEvent>()
        .add_event::<LoadEvent>()
        .run();
}

fn setup(
    mut commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<ColorMaterial>>,
) {
    // Spawn your average camera
    commands.spawn((Camera2d, GameCamera, CameraVelocity(Vec2::ZERO)));

    commands.insert_resource(MousePos(Vec2::ZERO));
    commands.insert_resource(GameState::Editor);

    commands.insert_resource(PastMouseMotions::default());

    // Disable Avian Gravity
    commands.insert_resource(Gravity::ZERO);
}

fn toggle_gamestate(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<GameState>) {
    if keys.just_pressed(KeyCode::KeyE) {
        *state = match *state {
            GameState::Editor => GameState::Play,
            GameState::Play => GameState::Editor,
        }
    }
}

// Needs fixing
fn move_camera_around_main_object(
    state: Res<GameState>,
    main_object_query: Query<&Transform, (Without<GameCamera>, With<CameraTrackable>)>,
    mut camera_query: Query<(&mut Transform, &OrthographicProjection), With<GameCamera>>,
    window_query: Query<&Window, Without<GameCamera>>,
) {
    if main_object_query.iter().count() != 0 && *state == GameState::Play {
        let main_object_translation = main_object_query
            .get_single()
            .expect("Multiple main objects detected")
            .translation;

        let (mut camera_transform, camera_projection) = camera_query
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
                (-dist_between / 60.0) / camera_projection.scale,
            );
        } else {
            // Outside of bounding box

            let dist_between = -closest_point + main_object_translation.xy();

            // Instantly move to the closest point

            update_xy(&mut camera_transform.translation, dist_between);
        }
    }
}

fn update_xy(vec: &mut Vec3, xy: Vec2) {
    vec.x += xy.x;
    vec.y += xy.y;
}

fn pan_camera_keys(
    mut camera_query: Query<(&mut Transform, &OrthographicProjection), With<GameCamera>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    use KeyCode::*;

    let (mut transform, projection) = camera_query.single_mut();

    let camera_movement_speed = CAMERA_MOVE_SPEED * projection.scale;

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

fn zoom_camera(
    mut camera_query: Query<(&mut OrthographicProjection, &mut Transform), With<GameCamera>>,
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    mouse_pos: Res<MousePos>,
) {
    use bevy::input::mouse::MouseScrollUnit;

    let (mut projection, mut transform) = camera_query.single_mut();

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

                let dif_vec = -transform.translation + Vec3::new(mouse_pos.0.x, mouse_pos.0.y, 0.0); // * zoom_modifier;

                transform.translation += (dif_vec / 10.0) * zoom_modifier;
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

fn global_binds(
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<MousePos>,
    keys: Res<ButtonInput<KeyCode>>,
    mut object_events: EventWriter<CreateObject>,
    mut save_events: EventWriter<SaveEvent>,
    mut load_events: EventWriter<LoadEvent>,
) {
    if mouse.just_pressed(MouseButton::Right) {
        object_events.send(CreateObject::new_static(5.0, mouse_pos.0, 10.0));
    }

    if keys.just_pressed(KeyCode::KeyZ) {
        object_events.send(CreateObject::new_trigger(mouse_pos.0));
    }

    if keys.just_pressed(KeyCode::KeyJ) {
        save_events.send(SaveEvent::new(
            PathBuf::from_str("test_levels/level").unwrap(),
            "Interesting",
        ));
    }

    if keys.just_pressed(KeyCode::KeyK) {
        load_events.send(LoadEvent::new(
            PathBuf::from_str("test_levels/level").unwrap(),
        ));
    }
}

fn game_binds(
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<MousePos>,
    _keys: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<CreateObject>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        events.send(*CreateObject::new_dynamic(5.0, mouse_pos.0, 10.0).set_selected());
    }
}

fn create_objects(
    mut events: EventReader<CreateObject>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for event in events.read() {
        match event {
            CreateObject::Static {
                mass,
                position,
                radius,
            } => {
                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(*radius))),
                    Transform {
                        translation: position.extend(0.0),
                        ..default()
                    },
                    MeshMaterial2d(materials.add(Color::oklab(1.0, 0.7, 0.3))),
                    Mass(*mass),
                    Gravitator,
                    Collider::circle(*radius as Scalar),
                    RigidBody::Static,
                    GravityLayers::new(GravityLayer::Static, [GameLayer::Main]),
                    StaticObject,
                ));
            }
            CreateObject::Dynamic {
                mass,
                position,
                radius,
                gravitable,
                gravitator,
                selected,
            } => {
                commands
                    .spawn((
                        Mesh2d(meshes.add(Circle::new(10.0))),
                        Transform::from_translation(position.extend(0.0)),
                        MeshMaterial2d(materials.add(Color::oklab(1.0, 0.7, 0.3))),
                        SelectedDynamicConfig::new(*gravitable, *gravitator, *radius, *mass),
                        DynamicObject,
                    ))
                    .insert_if(Selected, || *selected);
            }
            CreateObject::Trigger { position } => {
                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(10.0))),
                    Transform::from_translation(position.extend(-1.0)),
                    MeshMaterial2d(materials.add(Color::srgb(0.1, 0.3, 0.7))),
                    Trigger::new(false),
                    Collider::circle(10.0),
                    CollisionLayers::new(GameLayer::Triggers, [GameLayer::Main]),
                ));
            }
        }
    }
}

fn pan_camera_mouse(
    mouse_input: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut past_mouse_motions: ResMut<PastMouseMotions>,
    mut mouse_motion_reader: EventReader<MouseMotion>,
    mut camera_query: Query<
        (&mut Transform, &OrthographicProjection, &mut CameraVelocity),
        With<GameCamera>,
    >,
) {
    if !keys.pressed(KeyCode::ShiftLeft) {
        *past_mouse_motions = PastMouseMotions::default();
    }

    let (mut camera_transform, projection, mut velocity) = camera_query.single_mut();

    let mut mouse_move_counter = Vec2::ZERO;

    if mouse_input.pressed(MouseButton::Left) && keys.pressed(KeyCode::ShiftLeft) {
        for mouse_motion in mouse_motion_reader.read() {
            let corrected_mouse_motion = Vec2::new(-mouse_motion.delta.x, mouse_motion.delta.y);

            past_mouse_motions.0.push_back(corrected_mouse_motion);
            past_mouse_motions.0.pop_front();

            update_xy(
                &mut camera_transform.translation,
                corrected_mouse_motion * projection.scale,
            );
            mouse_move_counter += mouse_motion.delta;
        }
    }

    if mouse_input.just_released(MouseButton::Left) {
        velocity.0 += past_mouse_motions.0.iter().sum::<Vec2>() * projection.scale;
    }

    if mouse_input.just_pressed(MouseButton::Left) {
        velocity.0 = Vec2::ZERO
    }

    velocity.0 /= 1.05
}

fn apply_camera_velocity(
    mut camera_query: Query<(&mut Transform, &CameraVelocity), With<GameCamera>>,
) {
    let (mut transform, velocity) = camera_query.single_mut();

    transform.translation += Vec3::new(velocity.x, velocity.y, 0.0);
}

fn release_selected(
    selected_query: Query<(Entity, &Transform, &SelectedDynamicConfig), With<Selected>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<MousePos>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if mouse_input.just_released(MouseButton::Left) {
        for (entity, transform, selected_dynamic_config) in selected_query.iter() {
            let dif = -(mouse_pos.0 - transform.translation.xy());

            let mut entity_commands = commands.entity(entity);

            entity_commands
                .insert((
                    Mass(selected_dynamic_config.mass),
                    LinearVelocity(dif),
                    Collider::circle(selected_dynamic_config.radius as Scalar),
                    RigidBody::Dynamic,
                ))
                .insert_if(Gravitable, || selected_dynamic_config.gravitable)
                .insert_if(Gravitator, || selected_dynamic_config.gravitator)
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
