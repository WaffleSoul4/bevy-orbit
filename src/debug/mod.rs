mod grid;
mod velocity;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use grid::GridSettings;

pub struct DebugPlugin;

impl Default for DebugPlugin {
    fn default() -> Self {
        DebugPlugin
    }
}

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin {
                enable_multipass_for_primary_context: false,
            });
        }

        app.add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::new()
                .run_if(|settings: Res<DebugSettings>| settings.show_inspector),
        )
        .insert_resource(DebugSettings::default())
        .add_systems(
            Update,
            (
                grid::draw_grid
                    .run_if(|settings: Res<DebugSettings>| settings.grid_settings.show_grid),
                velocity::draw_velocity_arrows
                    .run_if(|settings: Res<DebugSettings>| settings.show_velocity_arrows),
                debug_ui.run_if(|settings: Res<DebugSettings>| settings.show_ui),
            ),
        );
    }
}

#[derive(Resource)]
pub struct DebugSettings {
    pub show_inspector: bool,
    pub show_ui: bool,
    show_velocity_arrows: bool,
    grid_settings: GridSettings,
}

impl Default for DebugSettings {
    fn default() -> Self {
        DebugSettings {
            show_inspector: false,
            show_ui: false,
            show_velocity_arrows: false,
            grid_settings: GridSettings::default(),
        }
    }
}

impl DebugSettings {
    pub fn toggle_ui(&mut self) {
        self.show_ui = !self.show_ui;
    }

    pub fn toggle_inspector(&mut self) {
        self.show_inspector = !self.show_inspector;
    }
}

pub fn toggle_debug_ui(mut settings: ResMut<DebugSettings>) {
    settings.toggle_ui();
}

pub fn debug_ui(
    mut contexts: EguiContexts,
    debug_settings: ResMut<DebugSettings>,
    camera_query: Single<
        (&mut Transform, &mut crate::camera::CameraVelocity),
        With<crate::camera::GameCamera>,
    >,
) {
    let mut debug_settings = debug_settings;

    let mut camera = camera_query.into_inner();

    egui::Window::new("Debug").show(contexts.ctx_mut(), |ui| {
        ui.button("Toggle inspector")
            .clicked()
            .then(|| debug_settings.toggle_inspector());
        ui.checkbox(&mut debug_settings.grid_settings.show_grid, "Show grid");
        ui.checkbox(
            &mut debug_settings.show_velocity_arrows,
            "Show velocity arrows",
        );
        ui.collapsing("Grid Settings", |ui| {
            grid::grid_settings_ui(ui, &mut debug_settings.grid_settings);
        });
        ui.collapsing("Camera Settings", |ui| {
            camera_settings_ui(ui, &mut *camera.0, &mut *camera.1);
        });
    });
}

fn camera_settings_ui(
    ui: &mut egui::Ui,
    camera_transform: &mut Transform,
    camera_velocity: &mut crate::camera::CameraVelocity,
) {
    ui.label("Position");
    ui.horizontal(|ui| {
        ui.label("x");
        ui.add(egui::DragValue::new(&mut camera_transform.translation.x));
        ui.label("y");
        ui.add(egui::DragValue::new(&mut camera_transform.translation.y));
    });
    ui.label("Velocity");
    ui.horizontal(|ui| {
        ui.label("x");
        ui.add(egui::DragValue::new(&mut camera_velocity.x));
        ui.label("y");
        ui.add(egui::DragValue::new(&mut camera_velocity.y));
    });
}
