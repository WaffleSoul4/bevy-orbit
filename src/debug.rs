use avian2d::prelude::LinearVelocity;
use bevy::prelude::*;
use bevy_egui::{
    EguiContexts,
    egui::{self},
};

pub struct DebugPlugin;

impl Default for DebugPlugin {
    fn default() -> Self {
        DebugPlugin
    }
}

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugSettings::default()).add_systems(
            Update,
            (
                draw_grid.run_if(|settings: Res<DebugSettings>| settings.grid_settings.show_grid),
                draw_velocity_arrows
                    .run_if(|settings: Res<DebugSettings>| settings.show_velocity_arrows),
                debug_ui.run_if(|settings: Res<DebugSettings>| settings.show_ui),
            ),
        );
    }
}

struct GridSettings {
    // Whether to show the grid or not
    show_grid: bool,
    // The size of each cell
    cell_size: Vec2,
    // How many cells to draw on x and y axis
    grid_draw_dimensions: Vec2,
    // Size of the upper lines
    recursive_depth: i32,
    // Color of lower lines
    lower_color: Hsva,
    // Color of upper lines (Hsva because of egui)
    upper_color: Hsva,
}

impl Default for GridSettings {
    fn default() -> Self {
        GridSettings {
            show_grid: false,
            cell_size: Vec2::new(64.0, 64.0),
            grid_draw_dimensions: Vec2::new(20.0, 16.0),
            recursive_depth: 2,
            lower_color: Hsva::new(0.0, 0.0, 0.2, 1.0),
            upper_color: Hsva::new(0.0, 0.0, 0.45, 1.0),
        }
    }
}

#[derive(Resource)]
pub struct DebugSettings {
    pub show_ui: bool,
    show_velocity_arrows: bool,
    grid_settings: GridSettings,
}

impl Default for DebugSettings {
    fn default() -> Self {
        DebugSettings {
            show_ui: true,
            show_velocity_arrows: false,
            grid_settings: GridSettings::default(),
        }
    }
}

impl DebugSettings {
    pub fn toggle_ui(&mut self) {
        self.show_ui = !self.show_ui;
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
        ui.checkbox(&mut debug_settings.grid_settings.show_grid, "Show grid");
        ui.checkbox(
            &mut debug_settings.show_velocity_arrows,
            "Show velocity arrows",
        );
        ui.collapsing("Grid Settings", |ui| {
            grid_settings_ui(ui, &mut debug_settings.grid_settings);
        });
        ui.collapsing("Camera Settings", |ui| {
            camera_settings_ui(ui, &mut *camera.0, &mut *camera.1);
        });
    });
}

fn grid_settings_ui(ui: &mut egui::Ui, grid_settings: &mut GridSettings) {
    let mut lower_egui_hsva = bevy_hsva_to_egui_hsva(grid_settings.lower_color);
    let mut upper_egui_hsva = bevy_hsva_to_egui_hsva(grid_settings.upper_color);

    ui.label("Cell Dimensions");
    ui.horizontal(|ui| {
        ui.label("x");
        ui.add(egui::DragValue::new(&mut grid_settings.cell_size.x));
        ui.label("y");
        ui.add(egui::DragValue::new(&mut grid_settings.cell_size.y))
    });
    ui.label("Cell Draw Dimensions");
    ui.horizontal(|ui| {
        ui.label("x");
        ui.add(egui::DragValue::new(
            &mut grid_settings.grid_draw_dimensions.x,
        ));
        ui.label("y");
        ui.add(egui::DragValue::new(
            &mut grid_settings.grid_draw_dimensions.y,
        ))
    });
    ui.label("Recursion count");
    ui.add(egui::Slider::new(&mut grid_settings.recursive_depth, 2..=6));
    ui.label("Colors");
    ui.horizontal(|ui| {
        ui.label("Lower");
        ui.color_edit_button_hsva(&mut lower_egui_hsva);
        ui.label("Upper");
        ui.color_edit_button_hsva(&mut upper_egui_hsva);
    });

    grid_settings.lower_color = egui_hsva_to_bevy_hsva(lower_egui_hsva);
    grid_settings.upper_color = egui_hsva_to_bevy_hsva(upper_egui_hsva);
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

fn egui_hsva_to_bevy_hsva(hsva: egui::epaint::Hsva) -> Hsva {
    Hsva::new(hsva.h * 360.0, hsva.s, hsva.v, hsva.a)
}

fn bevy_hsva_to_egui_hsva(hsva: Hsva) -> egui::epaint::Hsva {
    egui::epaint::Hsva::new(hsva.hue / 360.0, hsva.saturation, hsva.value, hsva.alpha)
}

pub fn draw_velocity_arrows(
    mut gizmos: Gizmos,
    mouse_pos: Res<crate::cursor::CursorPosition>,
    dynamic_object_query: Query<(&LinearVelocity, &Transform), With<Mesh2d>>,
    selected_object_query: Query<&Transform, With<crate::Selected>>,
) {
    dynamic_object_query
        .iter()
        .for_each(|(velocity, transform)| {
            gizmos.arrow_2d(
                transform.translation.xy(),
                transform.translation.xy() + velocity.0.xy() / 6.0,
                Color::srgb(0.1, 0.4, 0.6),
            );
        });

    // Draw arrows for objects that are currently [Selected]
    selected_object_query.iter().for_each(|transform| {
        let dif = transform.translation.xy() - mouse_pos.unwrap_or(transform.translation.xy());

        gizmos.arrow_2d(
            transform.translation.xy(),
            transform.translation.xy() + dif / 6.0,
            Color::srgb(0.1, 0.4, 0.6),
        );
    });
}

pub fn draw_grid(
    mut gizmos: Gizmos,
    debug_settings: Res<DebugSettings>,
    camera_query: Single<(&Transform, &Projection), With<crate::camera::GameCamera>>,
) {
    let (camera_transform, projection) = camera_query.into_inner();

    let projection = match projection {
        Projection::Orthographic(orthographic_projection) => orthographic_projection,
        _ => panic!("Invalid projection type found"),
    };

    let scale = projection.scale;

    let grid_settings = &debug_settings.grid_settings;
    let recursive_depth: f32 = grid_settings.recursive_depth as f32;

    // This allows for the flooring of the linear scale at exponential rates
    let floored_scaling = recursive_depth.powf(scale.log(recursive_depth).floor());

    // The space between every line of the grid
    let grid_spacing = floored_scaling * grid_settings.cell_size;

    let camera_xy = camera_transform.translation.xy();

    // The point closest to the center while still being aligned to the grid
    let closest_aligned_center = camera_xy - (camera_xy % (grid_spacing * recursive_depth));

    // How many cells away from center to draw on x and y axis
    let x_width = grid_settings.grid_draw_dimensions.x;
    let y_width = grid_settings.grid_draw_dimensions.y;

    for x in -x_width as i32..x_width as i32 {
        let line_x: f32 = x as f32 * grid_spacing.x;
        let color = if line_x % (grid_spacing.x * recursive_depth) == 0.0 {
            grid_settings.upper_color
        } else {
            grid_settings.lower_color
        };
        gizmos.line_2d(
            Vec2::new(line_x as f32, y_width * grid_spacing.y) + closest_aligned_center,
            Vec2::new(line_x as f32, -y_width * grid_spacing.y) + closest_aligned_center,
            color,
        );
    }

    for y in -y_width as i32..y_width as i32 {
        let line_y: f32 = y as f32 * grid_spacing.y;

        let color = if line_y % (grid_spacing.y * recursive_depth) == 0.0 {
            grid_settings.upper_color
        } else {
            grid_settings.lower_color
        };

        gizmos.line_2d(
            Vec2::new(x_width * grid_spacing.x, line_y as f32) + closest_aligned_center,
            Vec2::new(-x_width * grid_spacing.x, line_y as f32) + closest_aligned_center,
            color,
        );
    }
}
