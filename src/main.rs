use avian2d::prelude::*;
use bevy::{
    input::common_conditions::{input_just_pressed, input_just_released, input_pressed},
    prelude::*,
};
use bevy_orbit::{
    AppState::*,
    camera::{CameraPlugin, restore_viewport},
    cursor::CursorPlugin,
    debug::{DebugPlugin, toggle_debug_ui},
    editor::{EditorPlugin, side_menu},
    game::{
        DeathEvent, clear_triggered_indicators, initialize_triggered_indicators,
        spawn_launching_object_system, trace_object_paths,
    },
    gravity::GravityPlugin,
    serialization::{
        SerializationPlugin, StartPoint, convert_zone_builders, draw_start_point,
        initialize_zone_builder, load_active_level, remove_active_level, remove_level_entities,
        spawn_temp_scene, zone_creation_input_handler, zone_creation_outline_gizmos,
    },
    *,
};

fn main() {
    App::new()
        .add_plugins((
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
        // Keybind systems
        .add_systems(
            Update,
            (
                // game_input_handler
                //     .run_if(state_is(Play))
                //     .run_if(not(input_pressed(KeyCode::ShiftLeft))),
                launch_launching.run_if(input_just_released(MouseButton::Left)),
            ),
        )
        // Silly zone thing
        .add_systems(
            Update,
            (
                (
                    zone_creation_input_handler.run_if(input_pressed(KeyCode::ControlLeft)),
                    initialize_zone_builder.run_if(input_just_pressed(KeyCode::ControlLeft)),
                    convert_zone_builders.run_if(input_just_released(KeyCode::ControlLeft)),
                    zone_creation_outline_gizmos,
                )
                    .run_if(app_state_is(Editor)),
                convert_zone_builders.run_if(app_state_is(Play)),
            ),
        )
        // Miscellaeneous systems
        .add_systems(
            Update,
            (
                (toggle_gamestate, restore_viewport)
                    .chain()
                    .run_if(input_just_pressed(KeyCode::Backquote)),
                toggle_debug_ui.run_if(input_just_pressed(KeyCode::KeyQ)),
                side_menu.run_if(app_state_is(Editor)),
                (initialize_triggered_indicators, clear_triggered_indicators)
                    .run_if(app_state_is(Play)),
                trace_object_paths,
                draw_start_point.run_if(app_state_is(Editor)),
            ),
        )
        // Serialization bindings
        .add_systems(
            Update,
            (
                spawn_launching_object_system
                    .run_if(game_state_is(GameState::Launching))
                    .run_if(app_state_is(AppState::Play)),
                (
                    reset_level,
                    clear_level,
                    remove_active_level,
                    spawn_temp_scene,
                )
                    .chain()
                    .run_if(resource_changed::<AppState>)
                    .run_if(app_state_is(Editor)),
                (remove_level_entities, load_active_level)
                    .chain()
                    .run_if(resource_changed::<AppState>)
                    .run_if(app_state_is(Play)),
            ),
        )
        // Post Update Systems
        .add_systems(
            PostUpdate,
            (
                clear_level
                    .run_if(input_pressed(KeyCode::ShiftLeft))
                    .run_if(input_just_pressed(KeyCode::Space))
                    .run_if(app_state_is(Editor)),
                (reset_level, initialize_object)
                    .run_if(input_just_pressed(KeyCode::Space))
                    .run_if(not(input_pressed(KeyCode::ShiftLeft))),
            ),
        )
        .run();
}
