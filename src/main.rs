use avian2d::{math::*, prelude::*};
use bevy::prelude::*;

const GRAVITATIONAL_CONSTANT: f32 = 1.0;

#[derive(Component, Debug)]
struct Mass(f32);

#[derive(Component)]
struct Gravitable;

#[derive(Component)]
struct Gravitator;

#[derive(Resource)]
struct ShowArrows(bool);

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

#[derive(Component)]
struct Triggerer;

#[derive(PhysicsLayer, Default)]
enum Layer {
    #[default]
    Main,
    Triggers,
}

#[derive(Component)]
struct TriggerIndicator;

fn main() {
    App::new()
        .init_gizmo_group::<ArrowGizmos>()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                apply_gravity,
                mouse_tracker_sys,
                draw_velocity_arrows,
                mouse_input,
                keyboard_input,
                release_object,
                draw_selected_velocity_arrows,
                create_trigger,
                update_triggers,
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
    commands.spawn(Camera2d);

    commands.insert_resource(ShowArrows(true));
    commands.insert_resource(MousePos(Vec2::ZERO));

    //Disable Avian Gravity
    commands.insert_resource(Gravity::ZERO);
}

fn keyboard_input(keys: Res<ButtonInput<KeyCode>>, mut show_arrows: ResMut<ShowArrows>) {
    if keys.just_pressed(KeyCode::KeyQ) {
        show_arrows.0 = !show_arrows.0
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

            // TODO: Dist implementation being quite odd...
            // Progress: Normilization could work but also being wierd
            // Fixed by making sure the dist is greater than 0 so the normilization doesn't fail

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

fn update_triggers(
    mut trigger_query: Query<(Entity, &mut Trigger, &Transform), With<Collider>>,
    collisions: Res<Collisions>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let _ = trigger_query
        .iter_mut()
        .filter(
            |x| {
                collisions
                    .iter()
                    .any(|y| y.entity1 == x.0 || y.entity2 == x.0)
            }, // Check if the trigger is among the collisions
        )
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
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if mouse_input.just_pressed(MouseButton::Right) {
        create_stationary(&mut commands, &mut meshes, &mut materials, mouse_pos.0);
    }

    if mouse_input.just_pressed(MouseButton::Left) {
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

fn release_object(
    selected_query: Query<(Entity, &Transform), With<Selected>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<MousePos>,
    mut commands: Commands,
) {
    if mouse_input.just_released(MouseButton::Left) {
        for selected in selected_query.iter() {
            let dif = -(mouse_pos.0 - selected.1.translation.xy());

            commands
                .entity(selected.0)
                .insert((
                    Mass(10.0),
                    Gravitable,
                    Gravitator,
                    LinearVelocity(dif),
                    Collider::circle(10.0 as Scalar),
                    RigidBody::Dynamic,
                    Triggerer,
                ))
                .remove::<Selected>();
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

fn create_stationary(
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
    show_arrows: Res<ShowArrows>,
    mut gizmos: Gizmos,
    query: Query<(&LinearVelocity, &Transform)>,
) {
    if show_arrows.0 {
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
    show_arrows: Res<ShowArrows>,
    mouse_pos: Res<MousePos>,
    mut gizmos: Gizmos,
    transform_query: Query<&Transform, With<Selected>>,
) {
    if show_arrows.0 {
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
