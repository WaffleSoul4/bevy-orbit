use avian2d::{math::*, prelude::*};
use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_egui::{
    EguiContexts, EguiPlugin,
    egui::{self},
};
use gravity::GravityLayers;
use level::{EntityType, LevelDescriptor};
use std::{collections::VecDeque, path::PathBuf, str::FromStr};

mod gravity;
mod level;

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
    grid_settings: GridSettings,
}

#[derive(Event)]
struct SaveEvent {
    file: PathBuf,
    level_name: String,
}

#[derive(Event)]
struct LoadEvent {
    file: PathBuf,
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
            grid_draw_dimensions: Vec2::new(20.0, 16.0),
            recursive_depth: 2,
            lower_color: Hsva::new(0.0, 0.0, 0.2, 1.0),
            upper_color: Hsva::new(0.0, 0.0, 0.45, 1.0),
        }
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct ArrowGizmos;

#[derive(Resource, Deref)]
struct MousePos(Vec2);

#[derive(Component)]
struct Selected;

#[derive(Component)]
struct Trigger {
    pub state: bool,
}

// Basically I want to be able to store data ([Gravitable], [Gravitator])
// but not actually have them enabled
// This is a terrible solution
#[derive(Component)]
struct SelectedDynamicConfig {
    gravitable: bool,
    gravitator: bool,
    radius: f32,
    // Mass being here just feels more consistent ig
    mass: f32,
}

impl SelectedDynamicConfig {
    fn new(gravitable: bool, gravitator: bool, radius: f32, mass: f32) -> Self {
        SelectedDynamicConfig {
            gravitable,
            gravitator,
            radius,
            mass,
        }
    }
}

#[derive(Event, Clone, Copy)]
enum CreateObject {
    Static {
        mass: f32,
        position: Vec2,
        radius: f32,
    },
    Dynamic {
        mass: f32,
        position: Vec2,
        radius: f32,
        gravitable: bool,
        gravitator: bool,
        selected: bool,
    },
    Trigger {
        position: Vec2,
    },
}

impl CreateObject {
    fn new_static(mass: f32, position: Vec2, radius: f32) -> Self {
        CreateObject::Static {
            mass,
            position,
            radius,
        }
    }

    fn new_dynamic(mass: f32, position: Vec2, radius: f32) -> Self {
        CreateObject::Dynamic {
            mass,
            position,
            radius,
            gravitable: true,
            gravitator: true,
            selected: false,
        }
    }

    fn new_trigger(position: Vec2) -> Self {
        CreateObject::Trigger { position }
    }

    // Only run this on dynamics please <3
    fn set_selected(&mut self) -> &mut Self {
        match self {
            CreateObject::Dynamic { selected, .. } => {
                *selected = true;
            }
            _ => panic!("Called set_select on a non-dynamic object"),
        }

        self
    }
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
enum GameLayer {
    #[default]
    Main,
    Triggers,
}

// I guess I'll re-use the [PhysicsLayer] trait instead of
// copy pasting more code
#[derive(PhysicsLayer, Default, Copy, Clone)]
enum GravityLayer {
    #[default]
    Main,
    Static,
}

#[derive(Resource, Debug)]
struct PastMouseMotions(VecDeque<Vec2>);

impl Default for PastMouseMotions {
    fn default() -> Self {
        let mut past_mouse_motions: VecDeque<Vec2> = VecDeque::new();
        for _ in 0..5 {
            past_mouse_motions.push_back(Vec2::ZERO)
        }
        PastMouseMotions(past_mouse_motions)
    }
}

// I had some errors with LinearVelocity, so this will do until I'm not lazy
#[derive(Component, Deref, DerefMut)]
struct CameraVelocity(Vec2);

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
            (move_camera_around_main_object, release_selected, game_binds)
                .run_if(|state: Res<GameState>| *state == GameState::Play),
        )
        .add_systems(
            Update,
            (
                apply_gravity,
                create_objects,
                mouse_tracker_sys,
                draw_velocity_arrows,
                global_binds,
                toggle_gamestate,
                draw_selected_velocity_arrows,
                update_triggers,
                pan_camera_keys,
                zoom_camera,
                pan_camera_mouse,
                draw_grid,
                simulation_settings,
                apply_camera_velocity,
                save_level,
                load_level,
            ),
        )
        .add_systems(PostUpdate, (clear_level, reset_level))
        .add_event::<CreateObject>()
        .add_event::<SaveEvent>()
        .add_event::<LoadEvent>()
        .run();
}

fn setup(
    mut commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((Camera2d, GameCamera, CameraVelocity(Vec2::ZERO)));

    commands.insert_resource(DebugSettings {
        show_grid: false,
        show_velocity_arrows: false,
        grid_settings: GridSettings::default(),
    });
    commands.insert_resource(MousePos(Vec2::ZERO));
    commands.insert_resource(GameState::Editor);

    commands.insert_resource(PastMouseMotions::default());

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
    gravitator: Query<(&Mass, &Transform, Option<&GravityLayers>), With<Gravitator>>,
    mut gravitated: Query<
        (
            &Mass,
            &Transform,
            &mut LinearVelocity,
            Option<&GravityLayers>,
        ),
        With<Gravitable>,
    >,
    time: Res<Time>,
) {
    for (mass, transform, gravity_layers) in &gravitator {
        for (gravitated_mass, gravitated_transform, mut velocity, gravitated_gravity_layers) in
            &mut gravitated
        {
            let default_gravity_layer = GravityLayers::default();
            let gravitator_layers = gravity_layers.unwrap_or(&default_gravity_layer);
            let gravitated_layers = gravitated_gravity_layers.unwrap_or(&default_gravity_layer);

            if gravitator_layers.interacts_with(*gravitated_layers) {
                let diff_vector =
                    transform.translation.xy() - gravitated_transform.translation.xy();

                let dist = diff_vector.length();

                // This is still necessary for some reason
                // It throws quite the ambigous error when not included
                // Something along the lines of:
                // "The given sine and cosine produce an invalid rotation"
                // Probably has to do with normalization, might look into it

                if dist > 0.01 {
                    velocity.0 += diff_vector.normalize()
                        * (gravitated_mass.0 * mass.0 / dist.powi(2))
                        * 10000.0
                        * GRAVITATIONAL_CONSTANT
                        * time.delta_secs()
                }
            }
        }
    }
}

fn move_camera_around_main_object(
    state: Res<GameState>,
    main_object_query: Query<&Transform, (Without<GameCamera>, With<CameraTrackable>)>,
    mut camera_query: Query<(&mut Transform, &OrthographicProjection), With<GameCamera>>,
    window_query: Query<&Window, Without<GameCamera>>,
) {
    if main_object_query.iter().count() != 0 && *state == GameState::Play {
        let main_object_translation = main_object_query
            .get_single()
            .expect("Multiple main objects detected")
            .translation;

        let (mut camera_transform, camera_projection) = camera_query
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

            update_xy(
                &mut camera_transform.translation,
                (-dist_between / 60.0) / camera_projection.scale,
            );
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

fn pan_camera_keys(
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

fn global_binds(
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<MousePos>,
    keys: Res<ButtonInput<KeyCode>>,
    mut object_events: EventWriter<CreateObject>,
    mut save_events: EventWriter<SaveEvent>,
    mut load_events: EventWriter<LoadEvent>,
) {
    if mouse.just_pressed(MouseButton::Right) {
        object_events.send(CreateObject::new_static(10.0, mouse_pos.0, 10.0));
    }

    if keys.just_pressed(KeyCode::KeyZ) {
        object_events.send(CreateObject::new_trigger(mouse_pos.0));
    }

    if keys.just_pressed(KeyCode::KeyP) {
        save_events.send(SaveEvent {
            file: PathBuf::from_str("test_levels/level").unwrap(),
            level_name: "Interesting".to_string(),
        });
    }

    if keys.just_pressed(KeyCode::KeyL) {
        load_events.send(LoadEvent {
            file: PathBuf::from_str("test_levels/level").unwrap(),
        });
    }
}

fn game_binds(
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<MousePos>,
    _keys: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<CreateObject>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        events.send(*CreateObject::new_dynamic(10.0, mouse_pos.0, 10.0).set_selected());
    }
}

fn create_objects(
    mut events: EventReader<CreateObject>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for event in events.read() {
        match event {
            CreateObject::Static {
                mass,
                position,
                radius,
            } => {
                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(*radius))),
                    Transform {
                        translation: position.extend(0.0),
                        ..default()
                    },
                    MeshMaterial2d(materials.add(Color::oklab(1.0, 0.7, 0.3))),
                    Mass(*mass),
                    Gravitator,
                    Collider::circle(*radius as Scalar),
                    RigidBody::Static,
                    GravityLayers::new(GravityLayer::Static, [GameLayer::Main]),
                ));
            }
            CreateObject::Dynamic {
                mass,
                position,
                radius,
                gravitable,
                gravitator,
                selected,
            } => {
                commands
                    .spawn((
                        Mesh2d(meshes.add(Circle::new(10.0))),
                        Transform::from_translation(position.extend(0.0)),
                        MeshMaterial2d(materials.add(Color::oklab(1.0, 0.7, 0.3))),
                        SelectedDynamicConfig::new(*gravitable, *gravitator, *radius, *mass),
                    ))
                    .insert_if(Selected, || *selected);
            }
            CreateObject::Trigger { position } => {
                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(10.0))),
                    Transform::from_translation(position.extend(-1.0)),
                    MeshMaterial2d(materials.add(Color::srgb(0.1, 0.3, 0.7))),
                    Trigger::new(false),
                    Collider::circle(10.0),
                    CollisionLayers::new(GameLayer::Triggers, [GameLayer::Main]),
                ));
            }
        }
    }
}

fn pan_camera_mouse(
    mouse_input: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut past_mouse_motions: ResMut<PastMouseMotions>,
    mut mouse_motion_reader: EventReader<MouseMotion>,
    mut camera_query: Query<
        (&mut Transform, &OrthographicProjection, &mut CameraVelocity),
        With<GameCamera>,
    >,
) {
    if !keys.pressed(KeyCode::ShiftLeft) {
        *past_mouse_motions = PastMouseMotions::default();
    }

    let (mut camera_transform, projection, mut velocity) = camera_query.single_mut();

    let mut mouse_move_counter = Vec2::ZERO;

    if mouse_input.pressed(MouseButton::Left) && keys.pressed(KeyCode::ShiftLeft) {
        for mouse_motion in mouse_motion_reader.read() {
            let corrected_mouse_motion = Vec2::new(-mouse_motion.delta.x, mouse_motion.delta.y);

            past_mouse_motions.0.push_back(corrected_mouse_motion);
            past_mouse_motions.0.pop_front();

            update_xy(
                &mut camera_transform.translation,
                corrected_mouse_motion * projection.scale,
            );
            mouse_move_counter += mouse_motion.delta;
        }
    }

    if mouse_input.just_released(MouseButton::Left) {
        velocity.0 += past_mouse_motions.0.iter().sum::<Vec2>() * projection.scale;
    }

    if mouse_input.just_pressed(MouseButton::Left) {
        velocity.0 = Vec2::ZERO
    }

    velocity.0 /= 1.05
}

fn apply_camera_velocity(
    mut camera_query: Query<(&mut Transform, &CameraVelocity), With<GameCamera>>,
) {
    let (mut transform, velocity) = camera_query.single_mut();

    transform.translation += Vec3::new(velocity.x, velocity.y, 0.0);
}

fn release_selected(
    selected_query: Query<(Entity, &Transform, &SelectedDynamicConfig), With<Selected>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<MousePos>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if mouse_input.just_released(MouseButton::Left) {
        for (entity, transform, selected_dynamic_config) in selected_query.iter() {
            let dif = -(mouse_pos.0 - transform.translation.xy());

            let mut entity_commands = commands.entity(entity);

            entity_commands
                .insert((
                    Mass(selected_dynamic_config.mass),
                    LinearVelocity(dif),
                    Collider::circle(selected_dynamic_config.radius as Scalar),
                    RigidBody::Dynamic,
                ))
                .insert_if(Gravitable, || selected_dynamic_config.gravitable)
                .insert_if(Gravitator, || selected_dynamic_config.gravitator)
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
}

fn simulation_settings(
    mut contexts: EguiContexts,
    state: Res<GameState>,
    debug_settings: ResMut<DebugSettings>,
    mut camera_query: Query<(&mut Transform, &mut CameraVelocity), With<GameCamera>>,
) {
    if *state == GameState::Editor {
        let mut debug_settings = debug_settings;
        let mut camera = camera_query.single_mut();

        egui::Window::new("Editor Mode").show(contexts.ctx_mut(), |ui| {
            ui.checkbox(&mut debug_settings.show_grid, "Show grid");
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
    camera_velocity: &mut CameraVelocity,
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

fn save_level(
    statics: Query<
        (
            &Transform,
            &MeshMaterial2d<ColorMaterial>,
            Option<&Gravitator>,
            &Mass,
        ),
        Without<Gravitable>,
    >,
    materials: Res<Assets<ColorMaterial>>,
    mut save_events: EventReader<SaveEvent>,
) {
    for save_event in save_events.read() {
        let mut level_descriptor = LevelDescriptor::new(Vec2::ZERO, &save_event.level_name);

        for (transform, color, gravitator, mass) in statics.iter() {
            level_descriptor.add_entity(EntityType::new_static(
                transform.translation.xy(),
                mass.0,
                gravitator.is_some(),
                materials.get(color.id()).unwrap().color,
            ));
        }

        level_descriptor
            .save_to_file(save_event.file.clone())
            .expect("Error handling might happen one day :3");
    }
}

fn load_level(
    mut load_events: EventReader<LoadEvent>,
    mut object_events: EventWriter<CreateObject>,
) {
    for load_event in load_events.read() {
        let level_descriptor = LevelDescriptor::load_from_file(load_event.file.clone()).unwrap();

        for entity in level_descriptor.entities {
            match entity {
                EntityType::StaticObject {
                    position,
                    mass,
                    gravitator: _,
                    color: _,
                } => {
                    object_events.send(CreateObject::new_static(mass, position, 10.0));
                    // Ye there are definently some incompatabilities, these types should be very similar
                }
                EntityType::Trigger { position: _ } => todo!(),
            }
        }
    }
}
