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
use game::{DynamicObjectBundle, LaunchingObjectConfig, Triggered};
use serialization::GameSerializable;

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

#[derive(PhysicsLayer, Default)]
enum GameLayer {
    #[default]
    Main,
    Triggers,
}

#[derive(Component)]
struct CameraTrackable; // Oh god the naming is getting worse

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

            let launched = commands
                .entity(entity)
                .insert(DynamicObjectBundle::new(config, dif))
                .remove::<Launching>()
                .id();

            commands.spawn((
                game::PathTracer::new(launched),
                Transform::from_translation(Vec3::ZERO),
            ));
        });
}

pub fn clear_level(mut commands: Commands, query: Query<Entity, With<GameSerializable>>) {
    query
        .iter()
        .for_each(|x| commands.get_entity(x).unwrap().despawn());
}

// This is  the one that runs during the game
pub fn reset_level(
    mut commands: Commands,
    remove_query: Query<
        Entity,
        Or<(
            (With<LinearVelocity>, Without<GameSerializable>),
            With<game::PathTracer>,
        )>,
    >,
    mut trigger_query: Query<Entity, With<game::Triggered>>,
) {
    remove_query
        .iter()
        .for_each(|x| commands.get_entity(x).unwrap().despawn());

    trigger_query.iter_mut().for_each(|entity| {
        commands.entity(entity).remove::<Triggered>();
    });
}

pub fn state_is(state: GameState) -> impl Fn(Res<GameState>) -> bool {
    move |state_res: Res<GameState>| *state_res == state
}
