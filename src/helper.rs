use crate::{AppState, game::GameState};
use bevy::{
    ecs::{component::HookContext, world::DeferredWorld},
    prelude::*,
};

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
pub struct DumpEvents<T: Event + std::fmt::Debug>(std::marker::PhantomData<T>);

fn initialize_event_dumper<T: Event + std::fmt::Debug>(
    mut world: DeferredWorld,
    context: HookContext,
) {
    world
        .commands()
        .entity(context.entity)
        .observe(|trigger: Trigger<T>| info!("{trigger:?}"));
}

pub fn add_observer_on_hook<
    T: Event,
    F: bevy::ecs::system::IntoObserverSystem<T, B, M> + Clone,
    B: Bundle,
    M,
>(
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

pub fn debug_resource<T: Resource + std::fmt::Debug>(res: Res<T>) {
    eprintln!("{res:?}")
}

pub fn set_resource<T: Resource + Copy>(val: T) -> impl Fn(ResMut<T>) {
    move |mut res: ResMut<T>| *res = val
}
