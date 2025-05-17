use avian2d::{math::*, prelude::*};
use bevy::{
    input::{common_conditions::input_just_pressed, mouse::MouseMotion},
    prelude::*,
};
use bevy_egui::EguiPlugin;
use debug::{DebugPlugin, DebugSettings};
use editor::{CreateObject, GameSerializable, LoadEvent, SaveEvent, SelectedDynamicConfig};
use gravity::{Gravitable, Gravitator, GravityLayer, GravityLayers};
use std::{collections::VecDeque, path::PathBuf, str::FromStr};

mod debug;
mod editor;
mod gravity;
mod level;

const CAMERA_MOVE_SPEED: f32 = 10.0;

#[derive(Resource, Deref)]
struct MousePos(Option<Vec2>);

impl From<Option<Vec2>> for MousePos {
    fn from(value: Option<Vec2>) -> Self {
        MousePos(value)
    }
}

// This is just a resilient version of SerializableCollider
#[derive(Component, Reflect)]
#[reflect(Component)]
struct SerializableCollider(ColliderConstructor);

fn initialize_colliders(
    colliders: Query<(&SerializableCollider, Entity), Without<Collider>>,
    mut commands: Commands,
) {
    colliders.iter().for_each(|(serializable_collider, entity)|{
        commands
            .entity(entity)
            .with_child(serializable_collider.0.clone());
    });
}

#[derive(Component)]
struct Selected;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct DynamicObject;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct StaticObject;

#[derive(Component, Reflect)]
#[reflect(Component)]
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
        let mut past_mouse_motions: VecDeque<Vec2> = VecDeque::with_capacity(5);

        for _ in 0..5 {
            past_mouse_motions.push_back(Vec2::ZERO)
        }

        PastMouseMotions(past_mouse_motions)
    }
}

impl PastMouseMotions {
    fn update(&mut self, val: Vec2) {
        self.0.pop_front();
        self.0.push_back(val);
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
            EguiPlugin {
                enable_multipass_for_primary_context: false,
            },
            DebugPlugin,
            editor::SerializeableTypeRegistrationPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (release_selected, game_binds)
                .run_if(|state: Res<GameState>| *state == GameState::Play),
        )
        .add_systems(
            Update,
            (
                initialize_colliders,
                gravity::apply_gravity,
                create_objects,
                get_cursor_position.pipe(update_resource::<Option<Vec2>, MousePos>),
                (global_binds, zoom_camera, pan_camera_mouse)
                    .run_if(resource_is_some::<MousePos, _>),
                toggle_gamestate,
                update_triggers,
                pan_camera_keys,
                apply_camera_velocity,
                editor::side_menu.run_if(|state: Res<GameState>| *state == GameState::Editor),
                restore_viewport.run_if(resource_changed::<GameState>),
                editor::serialize_objects,
                editor::deserialize_objects,
            ),
        )
        .add_systems(
            Update,
            (|mut settings: ResMut<DebugSettings>| settings.toggle_ui())
                .run_if(input_just_pressed(KeyCode::KeyL)),
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

    commands.insert_resource(MousePos(None));
    commands.insert_resource(GameState::Play);

    commands.insert_resource(PastMouseMotions::default());

    // Disable Avian Gravity
    commands.insert_resource(Gravity::ZERO);
}

// Using the window data to defie the viewport doesn't support changes to the window
// This caused an error where the viewport wouldn't update when the window was resized
fn restore_viewport(mut camera: Single<&mut Camera, With<GameCamera>>) {
    camera.viewport = None;
}

fn toggle_gamestate(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<GameState>) {
    if keys.just_pressed(KeyCode::KeyE) {
        *state = match *state {
            GameState::Editor => GameState::Play,
            GameState::Play => GameState::Editor,
        }
    }
}

fn pan_camera_keys(
    camera_query: Single<(&mut Transform, &Projection), With<GameCamera>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    use KeyCode::*;

    let (mut transform, projection) = camera_query.into_inner();

    let projection = match projection {
        Projection::Orthographic(orthographic_projection) => orthographic_projection,
        _ => panic!("Invalid projection type found"),
    };

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
    camera_query: Single<(&mut Projection, &mut Transform), With<GameCamera>>,
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    mouse_pos: Res<MousePos>,
) {
    use bevy::input::mouse::MouseScrollUnit;

    let (mut projection, mut transform) = camera_query.into_inner();

    let projection = match projection.as_mut() {
        Projection::Orthographic(orthographic_projection) => orthographic_projection,
        _ => panic!("Invalid camera projection type found"),
    };

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

                let dif_vec = -transform.translation + mouse_pos.unwrap_or(Vec2::ZERO).extend(0.0);

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
    mut trigger_query: Query<(Entity, &mut Trigger), With<Collider>>,
    collisions: Collisions,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let _ = trigger_query
        .iter_mut()
        .filter(|(trigger_entity, _)| collisions.collisions_with(*trigger_entity).next().is_some())
        .for_each(|(trigger_entity, trigger)| {
            if !trigger.state {
                trigger.into_inner().trigger();

                let child = commands
                    .spawn((
                        Mesh2d(meshes.add(Circle::new(12.0))),
                        Transform::from_xyz(0.0, 0.0, -1.0),
                        MeshMaterial2d(materials.add(Color::srgb(0.1, 0.7, 0.3))),
                        TriggerIndicator,
                    ))
                    .id();

                commands.entity(trigger_entity).add_child(child);
            }
        });
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

// This makes me feel like a real programmer
fn update_resource<Input, Resource: bevy::prelude::Resource + From<Input>>(
    In(value): In<Input>,
    mut resource: ResMut<Resource>,
) {
    *resource = Resource::from(value)
}

fn resource_is_some<Resource: bevy::prelude::Resource + std::ops::Deref<Target = Option<T>>, T>(
    resource: Res<Resource>,
) -> bool {
    resource.into_inner().is_some()
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
        object_events.write(CreateObject::new_static(
            5.0,
            mouse_pos.unwrap_or_default(),
            10.0,
        ));
    }

    if keys.just_pressed(KeyCode::KeyZ) {
        object_events.write(CreateObject::new_trigger(mouse_pos.unwrap_or_default()));
    }

    if keys.just_pressed(KeyCode::KeyJ) {
        save_events.write(SaveEvent::new(
            PathBuf::from_str("test_levels/level").unwrap(),
            "Interesting",
        ));
    }

    if keys.just_pressed(KeyCode::KeyK) {
        load_events.write(LoadEvent::new(
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
        events.write(
            *CreateObject::new_dynamic(5.0, mouse_pos.unwrap_or_default(), 10.0).set_selected(),
        );
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
                    Gravitator, // Thing
                    SerializableCollider(ColliderConstructor::Circle { radius: *radius as Scalar }),
                    RigidBody::Static,
                    GravityLayers::new(GravityLayer::Static, [GameLayer::Main]),
                    StaticObject,
                    GameSerializable,
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
                commands.spawn((
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
                    SerializableCollider(ColliderConstructor::Circle { radius: 10.0 }),
                    CollisionLayers::new(GameLayer::Triggers, [GameLayer::Main]),
                    GameSerializable,
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
    camera_query: Single<(&mut Transform, &Projection, &mut CameraVelocity), With<GameCamera>>,
) {
    if !keys.pressed(KeyCode::ShiftLeft) {
        *past_mouse_motions = PastMouseMotions::default();
    }

    let (mut camera_transform, projection, mut velocity) = camera_query.into_inner();

    let projection = match projection {
        Projection::Orthographic(orthographic_projection) => orthographic_projection,
        _ => panic!("Invalid camera projection type found"),
    };

    if mouse_input.pressed(MouseButton::Left) && keys.pressed(KeyCode::ShiftLeft) {
        let mut motions = mouse_motion_reader.read();

        match motions.next() {
            Some(motion) => {
                let corrected_motion = Vec2::new(-motion.delta.x, motion.delta.y);

                past_mouse_motions.update(corrected_motion);

                let camera_z = camera_transform.translation.z;

                camera_transform.translation += corrected_motion.extend(camera_z);

                motions.for_each(|motion| {
                    let corrected_motion = Vec2::new(-motion.delta.x, motion.delta.y);

                    past_mouse_motions.update(corrected_motion);

                    let camera_z = camera_transform.translation.z;

                    camera_transform.translation += corrected_motion.extend(camera_z);
                });
            }
            None => past_mouse_motions.update(Vec2::ZERO),
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
    camera_query: Single<(&mut Transform, &CameraVelocity), With<GameCamera>>,
) {
    let (mut transform, velocity) = camera_query.into_inner();

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
            // If the mouse pos is none, do nothing
            let dif = transform.translation.xy() - mouse_pos.unwrap_or(transform.translation.xy());

            let mut entity_commands = commands.entity(entity);

            entity_commands
                .insert((
                    Mass(selected_dynamic_config.mass),
                    LinearVelocity(dif),
                    SerializableCollider(ColliderConstructor::Circle { radius: selected_dynamic_config.radius as Scalar }),
                    RigidBody::Dynamic,
                ))
                .insert_if(Gravitable, || selected_dynamic_config.gravitable)
                .insert_if(Gravitator, || selected_dynamic_config.gravitator) // Other thing
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
