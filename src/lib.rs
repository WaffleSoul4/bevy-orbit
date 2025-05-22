pub mod camera;
pub mod cursor;
pub mod debug;
pub mod editor;
pub mod game;
pub mod gravity;
pub mod serialization;

use avian2d::prelude::*;
use bevy::prelude::*;

use cursor::CursorPosition;
use game::{DynamicObjectBundle, LaunchingObjectConfig};
use serialization::{GameSerializable};

pub fn setup(mut commands: Commands) {
    commands.insert_resource(GameState::Play);

    // Disable Avian Gravity
    commands.insert_resource(avian2d::prelude::Gravity::ZERO);
}

#[derive(Component)]
pub struct Launching;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct DynamicObject;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct LevelObject;

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

#[derive(Resource, PartialEq, Clone)]
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

pub fn launch_launching(
    launching_query: Query<(Entity, &Transform, &LaunchingObjectConfig), With<Launching>>,
    cursor_position: Res<CursorPosition>,
    mut commands: Commands,
) {
    launching_query
        .iter()
        .for_each(|(entity, transform, config)| {
            let dif =
                transform.translation.xy() - cursor_position.unwrap_or(transform.translation.xy());

            commands
                .entity(entity)
                .insert(DynamicObjectBundle::new(config, dif))
                .remove::<Launching>();
        });
}

pub fn clear_level(
    mut commands: Commands,
    query: Query<Entity, (With<Mesh2d>, With<Collider>, With<GlobalTransform>)>,
) {
    query
        .iter()
        .for_each(|x| commands.get_entity(x).unwrap().despawn())
}

pub fn reset_level(
    mut commands: Commands,
    remove_query: Query<Entity, (With<LinearVelocity>, Without<GameSerializable>)>,
    mut trigger_query: Query<(Entity, &mut Trigger)>,
) {
    remove_query
        .iter()
        .for_each(|x| commands.get_entity(x).unwrap().despawn());

    trigger_query.iter_mut().for_each(|(entity, mut trigger)| {
        trigger.reset();
        commands.entity(entity).despawn_related::<Children>();
    });
}

pub fn state_is(state: GameState) -> impl Fn(Res<GameState>) -> bool {
    move |state_res: Res<GameState>| *state_res == state
}
