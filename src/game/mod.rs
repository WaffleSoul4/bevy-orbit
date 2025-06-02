pub mod death;
pub mod gravity;
pub mod launch;
pub mod trace;
pub mod trigger;

use bevy::{input::common_conditions::input_just_pressed, prelude::*};

use crate::{AppState, helper::app_state_is};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameState::Launching).add_systems(
            Update,
            (
                launch::launch_launching.run_if(input_just_pressed(MouseButton::Left)),
                launch::spawn_launching_objects.run_if(game_state_is(GameState::Launching)),
                (
                    trigger::initialize_triggered_indicators,
                    trigger::clear_triggered_indicators,
                )
                    .run_if(app_state_is(AppState::Play)),
                trace::trace_object_paths,
            ),
        );
    }
}

#[derive(Resource, PartialEq, Copy, Clone)]
pub enum GameState {
    Paused,
    Sandbox,
    Launching,
    Launched,
}

pub fn sandbox_input_handler(
    _keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_position: Res<crate::cursor::CursorPosition>,
    mut commands: Commands,
) {
    if let Some(cursor_position) = **cursor_position {
        if mouse.just_pressed(MouseButton::Left) {
            commands.spawn(launch::LaunchObjectBundle::default().with_position(cursor_position));
        }
    }
}

pub fn game_state_is(state: GameState) -> impl Fn(Res<GameState>) -> bool {
    move |state_res: Res<GameState>| *state_res == state
}

// Does not reset everything, do that yourself
pub fn clear_level(
    mut commands: Commands,
    remove_query: Query<
        Entity,
        (
            Or<(With<launch::DynamicObject>, With<trace::PathTracer>)>,
            Without<death::DeathEventsEnabled>,
        ),
    >,
    killable_query: Query<Entity, With<death::DeathEventsEnabled>>,
    mut trigger_query: Query<Entity, With<trigger::Triggered>>,
    starting_position: Res<crate::serialization::StartPoint>,
    mut game_state: ResMut<GameState>,
) {
    remove_query
        .iter()
        .for_each(|x| commands.get_entity(x).unwrap().despawn());

    commands.trigger_targets(
        death::DeathEvent::new(death::DeathSource::Reset),
        killable_query.iter().collect::<Vec<Entity>>(),
    );

    trigger_query.iter_mut().for_each(|entity| {
        commands.entity(entity).remove::<trigger::Triggered>();
    });

    *game_state = match **starting_position {
        Some(_) => GameState::Launching,
        None => GameState::Sandbox,
    }
}
