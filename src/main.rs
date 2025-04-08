use avian2d::{math::*, prelude::*};
use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_egui::{
    EguiContexts, EguiPlugin,
    egui::{self},
};

const GRAVITATIONAL_CONSTANT: f32 = 1.0;
const CAMERA_MOVE_SPEED: f32 = 10.0;

#[derive(Component, Debug)]
struct Mass(f32);

#[derive(Component)]
struct Gravitable;

#[derive(Component)]
struct Gravitator;

#[derive(Resource)]
struct DebugSettings {
    show_grid: bool,
    show_velocity_arrows: bool,
    show_grid_settings: bool,
    grid_settings: GridSettings,
}

struct GridSettings {
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
            cell_size: Vec2::new(64.0, 64.0),
            grid_draw_dimensions: Vec2::new(16.0, 16.0),
            recursive_depth: 2,
            lower_color: Hsva::new(0.0, 0.0, 0.2, 1.0),
            upper_color: Hsva::new(0.0, 0.0, 0.45, 1.0),
        }
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct ArrowGizmos;

#[derive(Resource)]
struct MousePos(Vec2);

#[derive(Component)]
struct Selected;

#[derive(Component)]
struct Trigger {
    pub state: bool,
}

#[derive(Component)]
struct GameCamera;

impl Trigger {
    fn new(state: bool) -> Self {
        Trigger { state }
    }

    fn trigger(&mut self) {
        self.state = true;
    }

    fn reset(&mut self) {
        self.state = false;
    }
}

#[derive(PhysicsLayer, Default)]
enum Layer {
    #[default]
    Main,
    Triggers,
}

#[derive(Component)]
struct CameraTrackable; // Oh god the naming is getting worse

#[derive(Component)]
struct TriggerIndicator;

#[derive(Resource, PartialEq)]
enum GameState {
    Editor,
    Play,
}

fn main() {
    App::new()
        .init_gizmo_group::<ArrowGizmos>()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default(), EguiPlugin))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                apply_gravity,
                mouse_tracker_sys,
                draw_velocity_arrows,
                mouse_input,
                toggle_gamestate,
                release_object,
                draw_selected_velocity_arrows,
                create_trigger,
                update_triggers,
                move_camera,
                zoom_camera,
                mouse_panning,
                move_camera_around_main_object,
                draw_grid,
                simulation_setting,
                grid_settings,
            ),
        )
        .add_systems(PostUpdate, (clear_level, reset_level))
        .run();
}

fn setup(
    mut commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((Camera2d, GameCamera));

    commands.insert_resource(DebugSettings {
        show_grid: false,
        show_velocity_arrows: false,
        show_grid_settings: false,
        grid_settings: GridSettings::default(),
    });
    commands.insert_resource(MousePos(Vec2::ZERO));
    commands.insert_resource(GameState::Editor);

    // Disable Avian Gravity
    commands.insert_resource(Gravity::ZERO);
}

fn toggle_gamestate(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<GameState>) {
    if keys.just_pressed(KeyCode::KeyE) {
        *state = match *state {
            GameState::Editor => GameState::Play,
            GameState::Play => GameState::Editor,
        }
    }
}

fn apply_gravity(
    gravitator: Query<(&Mass, &Transform), With<Gravitator>>,
    mut gravitated: Query<(&Mass, &Transform, &mut LinearVelocity), With<Gravitable>>,
    time: Res<Time>,
) {
    for (mass, transform) in &gravitator {
        for (gravitee_mass, gravitee_transform, mut velocity) in &mut gravitated {
            let diff_vector = transform.translation.xy() - gravitee_transform.translation.xy();

            let dist = diff_vector.length();

            if dist > 0.01 {
                velocity.0 += diff_vector.normalize()
                    * (gravitee_mass.0 * mass.0 / dist.powi(2))
                    * 10000.0
                    * GRAVITATIONAL_CONSTANT
                    * time.delta_secs()
            }
        }
    }
}

fn move_camera_around_main_object(
    state: Res<GameState>,
    main_object_query: Query<&Transform, (Without<GameCamera>, With<CameraTrackable>)>,
    mut camera_query: Query<&mut Transform, With<GameCamera>>,
    window_query: Query<&Window, Without<GameCamera>>,
) {
    if main_object_query.iter().count() != 0 && *state == GameState::Play {
        let main_object_translation = main_object_query
            .get_single()
            .expect("Multiple main objects detected")
            .translation;

        let mut camera_transform = camera_query
            .get_single_mut()
            .expect("Multiple game cameras detected");

        let window = window_query
            .get_single()
            .expect("Multiple windows detected");

        let window_bounding_box = bevy::math::bounding::Aabb2d::new(
            camera_transform.translation.xy(),
            Vec2::new(window.width() * 0.4, window.height() * 0.4),
        );

        let closest_point = window_bounding_box.closest_point(main_object_translation.xy());

        if closest_point == main_object_translation.xy() {
            // In the bounding box

            let dist_between = camera_transform.translation.xy() - closest_point;

            // Move 1/60th of the distance towards the obj

            update_xy(&mut camera_transform.translation, -dist_between / 60.0);
        } else {
            // Outside of bounding box

            let dist_between = -closest_point + main_object_translation.xy();

            // Instantly move to the closest point

            update_xy(&mut camera_transform.translation, dist_between);
        }
    }
}

fn update_xy(vec: &mut Vec3, xy: Vec2) {
    vec.x += xy.x;
    vec.y += xy.y;
}

fn move_camera(
    mut camera_query: Query<(&mut Transform, &OrthographicProjection), With<GameCamera>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    use KeyCode::*;

    let (mut transform, projection) = camera_query.single_mut();

    let camera_movement_speed = CAMERA_MOVE_SPEED * projection.scale;

    if keys.any_pressed([ArrowRight, KeyD]) {
        transform.translation.x += camera_movement_speed;
    }

    if keys.any_pressed([ArrowLeft, KeyA]) {
        transform.translation.x -= camera_movement_speed;
    }

    if keys.any_pressed([ArrowDown, KeyS]) {
        transform.translation.y -= camera_movement_speed;
    }

    if keys.any_pressed([ArrowUp, KeyW]) {
        transform.translation.y += camera_movement_speed;
    }
}

fn zoom_camera(
    mut camera_query: Query<(&mut OrthographicProjection, &mut Transform), With<GameCamera>>,
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    mouse_pos: Res<MousePos>,
) {
    use bevy::input::mouse::MouseScrollUnit;

    let (mut projection, mut transform) = camera_query.single_mut();

    for event in scroll_events.read() {
        let mut zoom_modifier = 1.0;

        match event.unit {
            MouseScrollUnit::Line => {
                if event.y <= -1.0 {
                    projection.scale *= 1.1;
                    zoom_modifier = -1.1;
                } else if event.y >= 1.0 {
                    projection.scale /= 1.1;
                }

                let dif_vec = -transform.translation + Vec3::new(mouse_pos.0.x, mouse_pos.0.y, 0.0); // * zoom_modifier;

                transform.translation += (dif_vec / 10.0) * zoom_modifier;
            }
            MouseScrollUnit::Pixel => {
                println!(
                    "Scroll (pixel units): vertical: {}, horizontal: {}",
                    event.y, event.x
                );
                todo!()
            }
        }
    }
}

fn update_triggers(
    mut trigger_query: Query<(Entity, &mut Trigger, &Transform), With<Collider>>,
    collisions: Res<Collisions>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let _ = trigger_query
        .iter_mut()
        .filter(|x| {
            collisions
                .iter()
                .any(|y| y.entity1 == x.0 || y.entity2 == x.0) // Check if the trigger is among the collisions
        })
        .for_each(|(_, ref mut t, transform)| {
            if !t.state {
                t.trigger();

                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(12.0))),
                    Transform {
                        translation: Vec3 {
                            x: transform.translation.x,
                            y: transform.translation.y,
                            z: transform.translation.z - 1.0,
                        },
                        ..transform.clone()
                    },
                    MeshMaterial2d(materials.add(Color::srgb(0.1, 0.7, 0.3))),
                    TriggerIndicator,
                ));
            }
        });
}

fn mouse_tracker_sys(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut mouse_pos: ResMut<MousePos>,
) {
    let window = windows.single();
    let (camera, camera_transform) = camera_query.single();

    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    {
        *mouse_pos = MousePos(Vec2::new(world_position.x, world_position.y))
    }
}

fn create_trigger(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_pos: Res<MousePos>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if keys.just_pressed(KeyCode::KeyZ) {
        commands.spawn((
            Mesh2d(meshes.add(Circle::new(10.0))),
            Transform {
                translation: Vec3 {
                    x: mouse_pos.0.x,
                    y: mouse_pos.0.y,
                    z: -1.0,
                },
                ..default()
            },
            MeshMaterial2d(materials.add(Color::srgb(0.1, 0.3, 0.7))),
            Trigger::new(false),
            Collider::circle(10.0),
            CollisionLayers::new(Layer::Triggers, [Layer::Main]),
        ));
    }
}

fn mouse_input(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<MousePos>,
    state: Res<GameState>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if mouse_input.just_pressed(MouseButton::Right) {
        create_static(&mut commands, &mut meshes, &mut materials, mouse_pos.0);
    }

    if mouse_input.just_pressed(MouseButton::Left) && *state == GameState::Play {
        commands.spawn((
            Mesh2d(meshes.add(Circle::new(10.0))),
            Transform {
                translation: Vec3 {
                    x: mouse_pos.0.x,
                    y: mouse_pos.0.y,
                    ..default()
                },
                ..default()
            },
            MeshMaterial2d(materials.add(Color::oklab(1.0, 0.7, 0.3))),
            Selected,
        ));
    }
}

fn mouse_panning(
    state: Res<GameState>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_reader: EventReader<MouseMotion>,
    mut camera_query: Query<(&mut Transform, &OrthographicProjection), With<GameCamera>>,
) {
    if *state == GameState::Editor && mouse_input.pressed(MouseButton::Left) {
        let (mut camera_transform, projection) = camera_query.single_mut();

        for mouse_motion in mouse_motion_reader.read() {
            update_xy(
                &mut camera_transform.translation,
                Vec2::new(-mouse_motion.delta.x, mouse_motion.delta.y) * projection.scale,
            );
        }
    }
}

fn release_object(
    selected_query: Query<(Entity, &Transform), With<Selected>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<MousePos>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if mouse_input.just_released(MouseButton::Left) {
        for selected in selected_query.iter() {
            let dif = -(mouse_pos.0 - selected.1.translation.xy());

            let mut entity_commands = commands.entity(selected.0);

            entity_commands
                .insert((
                    Mass(10.0),
                    Gravitable,
                    Gravitator,
                    LinearVelocity(dif),
                    Collider::circle(10.0 as Scalar),
                    RigidBody::Dynamic,
                ))
                .remove::<Selected>();

            if keys.pressed(KeyCode::ShiftLeft) {
                entity_commands.insert(CameraTrackable);
            }
        }
    }
}

fn clear_level(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    query: Query<Entity, With<Mesh2d>>,
) {
    if keys.just_pressed(KeyCode::Space) && keys.pressed(KeyCode::ShiftLeft) {
        query
            .iter()
            .for_each(|x| commands.get_entity(x).unwrap().despawn())
    }
}

fn reset_level(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    remove_query: Query<Entity, Or<(With<Gravitable>, With<TriggerIndicator>)>>,
    mut trigger_query: Query<&mut Trigger>,
) {
    if keys.just_pressed(KeyCode::Space) && !keys.pressed(KeyCode::ShiftLeft) {
        remove_query
            .iter()
            .for_each(|x| commands.get_entity(x).unwrap().despawn());

        trigger_query.iter_mut().for_each(|ref mut x| x.reset());
    }
}

fn create_static(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    pos: Vec2,
) {
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(10.0))),
        Transform {
            translation: Vec3 {
                x: pos.x,
                y: pos.y,
                ..default()
            },
            ..default()
        },
        MeshMaterial2d(materials.add(Color::oklab(1.0, 0.7, 0.3))),
        Mass(10.0),
        Gravitator,
        Collider::circle(10.0 as Scalar),
        RigidBody::Static,
    ));
}

fn draw_velocity_arrows(
    debug_settings: Res<DebugSettings>,
    mut gizmos: Gizmos,
    query: Query<(&LinearVelocity, &Transform)>,
) {
    if debug_settings.show_velocity_arrows {
        query.iter().for_each(|(velocity, transform)| {
            gizmos.arrow_2d(
                transform.translation.xy(),
                transform.translation.xy() + velocity.0.xy() / 6.0,
                Color::srgb(0.1, 0.4, 0.6),
            );
        })
    }
}

fn draw_selected_velocity_arrows(
    debug_info: Res<DebugSettings>,
    mouse_pos: Res<MousePos>,
    mut gizmos: Gizmos,
    transform_query: Query<&Transform, With<Selected>>,
) {
    if debug_info.show_velocity_arrows {
        transform_query.iter().for_each(|transform| {
            let dif = -(mouse_pos.0 - transform.translation.xy());

            gizmos.arrow_2d(
                transform.translation.xy(),
                transform.translation.xy() + dif / 6.0,
                Color::srgb(0.1, 0.4, 0.6),
            );
        })
    }
}

fn draw_grid(
    debug_settings: Res<DebugSettings>,
    mut gizmos: Gizmos,
    camera_query: Query<(&Transform, &OrthographicProjection), With<GameCamera>>,
) {
    if debug_settings.show_grid {
        let (camera_transform, projection) = camera_query.single();

        let mut scale = projection.scale;

        let grid_settings = &debug_settings.grid_settings;
        let recursive_depth: f32 = grid_settings.recursive_depth as f32;

        let scale_jumps_iter =
            std::iter::successors(Some(0.00390625_f32), |x| Some(x * recursive_depth));

        for scale_jump in scale_jumps_iter {
            if scale_jump > scale {
                scale = scale_jump;
                break;
            }
        }

        let grid_spacing = scale * grid_settings.cell_size;

        let camera_xy = camera_transform.translation.xy();

        let closest_center = camera_xy - (camera_xy % (grid_spacing * recursive_depth));

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
                Vec2::new(line_x as f32, y_width * grid_spacing.y) + closest_center,
                Vec2::new(line_x as f32, -y_width * grid_spacing.y) + closest_center,
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
                Vec2::new(x_width * grid_spacing.x, line_y as f32) + closest_center,
                Vec2::new(-x_width * grid_spacing.x, line_y as f32) + closest_center,
                color,
            );
        }
    }
}

fn simulation_setting(
    mut contexts: EguiContexts,
    state: Res<GameState>,
    debug_settings: ResMut<DebugSettings>,
) {
    if *state == GameState::Editor {
        let mut debug_settings = debug_settings;

        egui::Window::new("Editor Mode").show(contexts.ctx_mut(), |ui| {
            ui.checkbox(&mut debug_settings.show_grid, "Show grid");
            ui.checkbox(
                &mut debug_settings.show_velocity_arrows,
                "Show velocity arrows",
            );
            ui.checkbox(&mut debug_settings.show_grid_settings, "Show grid settings")
        });
    }
}

fn grid_settings(
    mut contexts: EguiContexts,
    state: Res<GameState>,
    debug_settings: ResMut<DebugSettings>,
) {
    let mut debug_settings = debug_settings;

    if *state == GameState::Editor && debug_settings.show_grid_settings {
        let grid_settings = &mut debug_settings.grid_settings;
        let mut lower_egui_hsva = bevy_hsva_to_egui_hsva(grid_settings.lower_color);
        let mut upper_egui_hsva = bevy_hsva_to_egui_hsva(grid_settings.upper_color);

        egui::Window::new("Grid Settings").show(contexts.ctx_mut(), |ui| {
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
            })
        });

        grid_settings.lower_color = egui_hsva_to_bevy_hsva(lower_egui_hsva);
        grid_settings.upper_color = egui_hsva_to_bevy_hsva(upper_egui_hsva);
    }
}

fn egui_hsva_to_bevy_hsva(hsva: egui::epaint::Hsva) -> Hsva {
    Hsva::new(hsva.h, hsva.s, hsva.v, hsva.a)
}

fn bevy_hsva_to_egui_hsva(hsva: Hsva) -> egui::epaint::Hsva {
    egui::epaint::Hsva::new(hsva.hue, hsva.saturation, hsva.value, hsva.alpha)
}
