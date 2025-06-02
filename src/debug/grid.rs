use bevy::prelude::*;
use bevy_egui::egui;

pub struct GridSettings {
    // Whether to show the grid or not
    pub show_grid: bool,
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

pub fn grid_settings_ui(ui: &mut egui::Ui, grid_settings: &mut GridSettings) {
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

fn egui_hsva_to_bevy_hsva(hsva: egui::epaint::Hsva) -> Hsva {
    Hsva::new(hsva.h * 360.0, hsva.s, hsva.v, hsva.a)
}

fn bevy_hsva_to_egui_hsva(hsva: Hsva) -> egui::epaint::Hsva {
    egui::epaint::Hsva::new(hsva.hue / 360.0, hsva.saturation, hsva.value, hsva.alpha)
}

pub fn draw_grid(
    mut gizmos: Gizmos,
    debug_settings: Res<super::DebugSettings>,
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
