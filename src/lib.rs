pub mod camera;
pub mod cursor;
pub mod debug;
pub mod editor;
pub mod game;
pub mod helper;
pub mod serialization;

use bevy::prelude::*;
use serialization::{GameSerializable, StartPoint};
pub fn setup(mut commands: Commands) {
    commands.init_resource::<StartPoint>();

    commands.insert_resource(AppState::Play);

    // Disable Avian Gravity
    commands.insert_resource(avian2d::prelude::Gravity::ZERO);
}

#[derive(Resource, PartialEq, Clone)]
pub enum AppState {
    Editor,
    Play,
}

pub fn toggle_app_state(mut state: ResMut<AppState>) {
    *state = match *state {
        AppState::Editor => AppState::Play,
        AppState::Play => AppState::Editor,
    }
}

pub fn clear_level(mut commands: Commands, query: Query<Entity, With<GameSerializable>>) {
    query
        .iter()
        .for_each(|x| commands.get_entity(x).unwrap().despawn());
}
