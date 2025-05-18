pub mod camera;
pub mod cursor;
pub mod debug;
pub mod editor;
pub mod gravity;
pub mod serialization;

use avian2d::{math::Scalar, prelude::*};
use bevy::prelude::*;

use cursor::CursorPosition;
use editor::CreateObject;
use gravity::{Gravitable, Gravitator, GravityLayer, GravityLayers};
use serialization::{GameSerializable, SerializableAsset, SerializableCollider};

pub fn setup(mut commands: Commands) {
    commands.insert_resource(GameState::Play);

    // Disable Avian Gravity
    commands.insert_resource(Gravity::ZERO);
}

#[derive(Component)]
pub struct Selected;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct DynamicObject;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct StaticObject;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Trigger {
    pub state: bool,
}

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

#[derive(Component)]
struct CameraTrackable; // Oh god the naming is getting worse

#[derive(Component)]
pub struct TriggerIndicator;

#[derive(Resource, PartialEq)]
pub enum GameState {
    Editor,
    Play,
}

pub fn toggle_gamestate(mut state: ResMut<GameState>) {
    *state = match *state {
        GameState::Editor => GameState::Play,
        GameState::Play => GameState::Editor,
    }
}

pub fn update_triggers(
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

pub fn global_binds(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_position: Res<CursorPosition>,
    keys: Res<ButtonInput<KeyCode>>,
    mut object_events: EventWriter<CreateObject>,
) {
    if mouse.just_pressed(MouseButton::Right) {
        object_events.write(CreateObject::new_static(
            5.0,
            cursor_position.unwrap_or_default(),
            10.0,
        ));
    }

    if keys.just_pressed(KeyCode::KeyZ) {
        object_events.write(CreateObject::new_trigger(
            cursor_position.unwrap_or_default(),
        ));
    }
}

pub fn game_binds(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_position: Res<CursorPosition>,
    keys: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<CreateObject>,
) {
    if mouse.just_pressed(MouseButton::Left) && !keys.pressed(KeyCode::ShiftLeft) {
        events.write(
            *CreateObject::new_dynamic(5.0, cursor_position.unwrap_or_default(), 10.0)
                .set_selected(),
        );
    }
}

// Basically I want to be able to store data ([Gravitable], [Gravitator])
// but not actually have them enabled
// This is a mediocre solution
#[derive(Component)]
pub struct SelectedDynamicConfig {
    pub gravitable: bool,
    pub gravitator: bool,
    pub radius: f32,
    // Mass being here just feels more consistent
    pub mass: f32,
}

impl SelectedDynamicConfig {
    pub fn new(gravitable: bool, gravitator: bool, radius: f32, mass: f32) -> Self {
        SelectedDynamicConfig {
            gravitable,
            gravitator,
            radius,
            mass,
        }
    }
}

pub fn create_objects(
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
                    SerializableAsset::mesh(Circle::new(*radius)),
                    Transform {
                        translation: position.extend(0.0),
                        ..default()
                    },
                    MeshMaterial2d(materials.add(Color::oklab(1.0, 0.7, 0.3))),
                    Mass(*mass),
                    Gravitator,
                    SerializableCollider::new(ColliderConstructor::Circle {
                        radius: *radius as Scalar,
                    }),
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
                commands
                    .spawn((
                        SerializableAsset::mesh(Circle::new(10.0)),
                        Transform::from_translation(position.extend(0.0)),
                        MeshMaterial2d(materials.add(Color::oklab(1.0, 0.7, 0.3))),
                        SelectedDynamicConfig::new(*gravitable, *gravitator, *radius, *mass),
                        DynamicObject,
                    ))
                    .insert_if(Selected, || *selected);
            }
            CreateObject::Trigger { position } => {
                commands.spawn((
                    SerializableAsset::mesh(Circle::new(10.0)),
                    Transform::from_translation(position.extend(-1.0)),
                    MeshMaterial2d(materials.add(Color::srgb(0.1, 0.3, 0.7))),
                    Trigger::new(false),
                    SerializableCollider::new(ColliderConstructor::Circle { radius: 10.0 }),
                    CollisionLayers::new(GameLayer::Triggers, [GameLayer::Main]),
                    GameSerializable,
                ));
            }
        }
    }
}

pub fn release_selected(
    selected_query: Query<(Entity, &Transform, &SelectedDynamicConfig), With<Selected>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_position: Res<CursorPosition>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if mouse.just_released(MouseButton::Left) {
        for (entity, transform, selected_dynamic_config) in selected_query.iter() {
            // If the cursor pos is none, do nothing
            let dif =
                transform.translation.xy() - cursor_position.unwrap_or(transform.translation.xy());

            let mut entity_commands = commands.entity(entity);

            entity_commands
                .insert((
                    Mass(selected_dynamic_config.mass),
                    LinearVelocity(dif),
                    SerializableCollider::new(ColliderConstructor::Circle {
                        radius: selected_dynamic_config.radius as Scalar,
                    }),
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

pub fn clear_level(mut commands: Commands, query: Query<Entity, With<Mesh2d>>) {
    query
        .iter()
        .for_each(|x| commands.get_entity(x).unwrap().despawn())
}

pub fn reset_level(
    mut commands: Commands,
    remove_query: Query<Entity, Or<(With<Gravitable>, With<TriggerIndicator>)>>,
    mut trigger_query: Query<&mut Trigger>,
) {
    remove_query
        .iter()
        .for_each(|x| commands.get_entity(x).unwrap().despawn());

    trigger_query.iter_mut().for_each(|ref mut x| x.reset());
}
