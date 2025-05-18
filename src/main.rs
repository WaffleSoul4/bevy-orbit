use avian2d::prelude::*;
use bevy::{
    input::common_conditions::{input_just_pressed, input_pressed},
    prelude::*,
};
use bevy_egui::EguiPlugin;
use bevy_orbit::{
    camera::{CameraPlugin, restore_viewport},
    cursor::CursorPlugin,
    debug::{DebugPlugin, toggle_debug_ui},
    editor::{EditorPlugin, side_menu},
    gravity::GravityPlugin,
    serialization::SerializationPlugin,
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
            EguiPlugin {
                enable_multipass_for_primary_context: false,
            },
        ))
        // Startup Systems
        .add_systems(Startup, setup)
        // Passive Update Systems
        .add_systems(Update, (create_objects, release_selected))
        // Keybind systems
        .add_systems(
            Update,
            (
                game_binds.run_if(|state: Res<GameState>| *state == GameState::Play),
                global_binds,
            ),
        )
        // Miscellaeneous systems
        .add_systems(
            Update,
            (
                (toggle_gamestate, restore_viewport)
                    .chain()
                    .run_if(input_just_pressed(KeyCode::KeyE)),
                toggle_debug_ui.run_if(input_just_pressed(KeyCode::KeyQ)),
                side_menu.run_if(|state: Res<GameState>| *state == GameState::Editor),
                update_triggers,
            ),
        )
        // Post Update Systems
        .add_systems(
            PostUpdate,
            (
                clear_level
                    .run_if(input_pressed(KeyCode::ShiftLeft))
                    .run_if(input_just_pressed(KeyCode::Space)),
                reset_level.run_if(input_just_pressed(KeyCode::Space)),
            ),
        )
        .run();
}
