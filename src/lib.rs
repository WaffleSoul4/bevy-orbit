pub mod camera;
pub mod cursor;
pub mod debug;
pub mod editor;
pub mod game;
pub mod gravity;
pub mod serialization;

use std::{fmt::Debug, marker::PhantomData};

use avian2d::prelude::*;
use bevy::{
    ecs::{component::HookContext, system::IntoObserverSystem, world::DeferredWorld},
    prelude::*,
};

use cursor::CursorPosition;
use game::{DeathEvent, DeathEventsEnabled, DynamicObjectBundle, LaunchingObjectConfig, Triggered};
use serialization::{GameSerializable, StartPoint};
pub fn setup(mut commands: Commands) {
    commands.init_resource::<StartPoint>();

    commands.insert_resource(AppState::Play);
    commands.insert_resource(GameState::Launching);

    // Disable Avian Gravity
    commands.insert_resource(avian2d::prelude::Gravity::ZERO);
}

#[derive(Component)]
pub struct Launching;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct DynamicObject;

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
pub enum AppState {
    Editor,
    Play,
}

#[derive(Resource, PartialEq, Copy, Clone)]
pub enum GameState {
    Paused,
    Sandbox,
    Launching,
    Launched,
}

pub fn toggle_gamestate(mut state: ResMut<AppState>) {
    *state = match *state {
        AppState::Editor => AppState::Play,
        AppState::Play => AppState::Editor,
    }
}

pub fn launch_launching(
    launching_query: Query<(Entity, &Transform, &LaunchingObjectConfig), With<Launching>>,
    cursor_position: Res<CursorPosition>,
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
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

            *game_state = match *game_state {
                GameState::Launching => GameState::Launched,
                GameState::Paused => {
                    warn!("Object launched while game paused");
                    GameState::Paused
                }
                _ => {
                    warn!("Object launched while in the launched state of the game, switching to sandbox");
                    GameState::Sandbox
                } // If something is launched while already in the launched state, enter sandbox
            }
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
        (
            Or<(With<DynamicObject>, With<game::PathTracer>)>,
            Without<DeathEventsEnabled>,
        ),
    >,
    killable_query: Query<Entity, With<DeathEventsEnabled>>,
    mut trigger_query: Query<Entity, With<game::Triggered>>,
    starting_position: Res<StartPoint>,
    mut game_state: ResMut<GameState>,
) {
    remove_query
        .iter()
        .for_each(|x| commands.get_entity(x).unwrap().despawn());

    commands.trigger_targets(
        DeathEvent::new(game::DeathSource::Reset),
        killable_query.iter().collect::<Vec<Entity>>(),
    );

    trigger_query.iter_mut().for_each(|entity| {
        commands.entity(entity).remove::<Triggered>();
    });

    *game_state = match **starting_position {
        Some(_) => GameState::Launching,
        None => GameState::Sandbox,
    }
}

pub fn initialize_object(mut commands: Commands, starting_position: Res<StartPoint>) {
    info!("Initializing object...");

    if let Some(start_point) = **starting_position {
        commands.spawn(game::LaunchObjectBundle::default().with_position(start_point));
    }
}

pub fn app_state_is(state: AppState) -> impl Fn(Res<AppState>) -> bool {
    move |state_res: Res<AppState>| *state_res == state
}

// I know they're the same but it's more verbose
pub fn game_state_is(state: GameState) -> impl Fn(Res<GameState>) -> bool {
    move |state_res: Res<GameState>| *state_res == state
}

pub fn dump_events<T: Event + std::fmt::Debug>(mut reader: EventReader<T>) {
    reader.read().for_each(|event| info!("{event:?}"));
}

#[derive(Component)]
#[component(on_add = initialize_event_dumper::<T>)]
pub struct DumpEvents<T: Event + std::fmt::Debug>(PhantomData<T>);

fn initialize_event_dumper<T: Event + std::fmt::Debug>(
    mut world: DeferredWorld,
    context: HookContext,
) {
    world
        .commands()
        .entity(context.entity)
        .observe(|trigger: Trigger<T>| info!("{trigger:?}"));
}

fn add_observer_on_hook<T: Event, F: IntoObserverSystem<T, B, M> + Clone, B: Bundle, M>(
    observer: F,
) -> impl Fn(DeferredWorld, HookContext) {
    move |mut world: DeferredWorld, context: HookContext| {
        world
            .commands()
            .get_entity(context.entity)
            .expect("Failed to find entity from component hook")
            .observe(observer.clone());
    }
}

pub fn debug_resource<T: Resource + Debug>(res: Res<T>) {
    eprintln!("{res:?}")
}

pub fn set_resource<T: Resource + Copy>(val: T) -> impl Fn(ResMut<T>) {
    move |mut res: ResMut<T>| *res = val
}
