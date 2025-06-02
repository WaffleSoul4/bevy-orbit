use avian2d::prelude::*;
use bevy::{
    input::common_conditions::{input_just_pressed, input_pressed},
    prelude::*,
};
use bevy_orbit::{
    AppState::*,
    camera::{CameraPlugin, restore_viewport},
    cursor::CursorPlugin,
    debug::{DebugPlugin, toggle_debug_ui},
    editor::EditorPlugin,
    game::{GamePlugin, clear_level, death::DeathEvent, gravity::GravityPlugin},
    helper::app_state_is,
    serialization::SerializationPlugin,
    *,
};

fn main() {
    App::new()
        .add_plugins((
            GamePlugin,
            DefaultPlugins,
            EditorPlugin,
            GravityPlugin,
            CursorPlugin,
            CameraPlugin,
            DebugPlugin,
            SerializationPlugin,
            PhysicsPlugins::default(),
        ))
        .add_event::<DeathEvent>()
        // Startup Systems
        .add_systems(Startup, setup)
        // Miscellaeneous systems
        .add_systems(
            Update,
            (
                (toggle_app_state, restore_viewport)
                    .chain()
                    .run_if(input_just_pressed(KeyCode::Backquote)),
                toggle_debug_ui.run_if(input_just_pressed(KeyCode::KeyQ)),
            ),
        )
        // Serialization bindings
        .add_systems(
            Update,
            (
                (
                    clear_level,
                    serialization::game::remove_active_level,
                    serialization::editor::spawn_temp_scene,
                )
                    .chain()
                    .run_if(resource_changed::<AppState>)
                    .run_if(app_state_is(Editor)),
                (
                    serialization::editor::remove_level_entities,
                    serialization::game::load_active_level,
                )
                    .chain()
                    .run_if(resource_changed::<AppState>)
                    .run_if(app_state_is(Play)),
            ),
        )
        // Post Update Systems
        .add_systems(
            PostUpdate,
            (
                bevy_orbit::clear_level
                    .run_if(input_pressed(KeyCode::ShiftLeft))
                    .run_if(input_just_pressed(KeyCode::Space))
                    .run_if(app_state_is(Editor)),
                clear_level
                    .run_if(input_just_pressed(KeyCode::Space))
                    .run_if(not(input_pressed(KeyCode::ShiftLeft))),
            ),
        )
        .run();
}
