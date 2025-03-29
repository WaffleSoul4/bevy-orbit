use bevy::{math::{bounding::{BoundingCircle, BoundingVolume}, VectorSpace}, prelude::*};

const GRAVITATIONAL_CONSTANT: f32 = 1.0;

#[derive(Component, Debug)]
struct Mass(f32);

#[derive(Component, Debug)]
struct Velocity(Vec2);

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

#[derive(Component, Clone, Debug)]
struct CircleCollider(BoundingCircle);

fn main() {
    App::new()
        .init_gizmo_group::<ArrowGizmos>()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (update_bounding_circles, apply_velocity, apply_gravity, mouse_tracker_sys, draw_arrows, mouse_input, detect_collisions, keyboard_input),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    commands.insert_resource(ShowArrows(true));
    commands.insert_resource(MousePos(Vec2::ZERO));

    commands.spawn((
        Mesh2d(meshes.add(Circle::new(10.0))),
        Transform::from_xyz(0.0, 0.0, 0.0),
        MeshMaterial2d(materials.add(Color::oklab(1.0, 0.3, 0.7))),
        Gravitator,
        Mass(20.0),
        CircleCollider(BoundingCircle::new(Vec2::ZERO, 10.0))
    ));

    commands.spawn((
        Mesh2d(meshes.add(Circle::new(10.0))),
        Transform::from_xyz(100.0, 0.0, 0.0),
        MeshMaterial2d(materials.add(Color::oklab(1.0, 0.7, 0.3))),
        Mass(10.0),
        Gravitable,
        Velocity(Vec2::new(0.0, -100.0)),
        CircleCollider(BoundingCircle::new(Vec2::new(100.0, 0.0), 10.0)),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Circle::new(10.0))),
        Transform::from_xyz(-100.0, 50.0, 0.0),
        MeshMaterial2d(materials.add(Color::oklab(1.0, 0.7, 0.3))),
        Mass(10.0),
        Gravitable,
        Gravitator,
        Velocity(Vec2::new(0.0, -100.0)),
        CircleCollider(BoundingCircle::new(Vec2::new(-100.0, 50.0), 10.0)),
    ));
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    let _ = query.iter_mut().for_each(|(mut transform, velocity)| {
        transform.translation += to_vec3(&velocity.0) * time.delta_secs()
    });
}

fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut show_arrows: ResMut<ShowArrows>,
) {
    if keys.just_pressed(KeyCode::KeyQ) {
        show_arrows.0 = !show_arrows.0
    }
}

fn apply_gravity(
    gravitator: Query<(&Mass, &Transform), With<Gravitator>>,
    mut gravitee: Query<(&Mass, &Transform, &mut Velocity)>,
    time: Res<Time>,
) {
    for (mass, transform) in &gravitator {
        for (gravitee_mass, gravitee_transform, mut velocity) in &mut gravitee {
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

fn to_vec3(vec: &Vec2) -> Vec3 {
    Vec3::new(vec.x, vec.y, 0.0)
} 

fn mouse_tracker_sys(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut mouse_pos: ResMut<MousePos>,
) {
    let window = windows.single();
    let (camera, camera_transform) = camera_q.single();

    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok()) {
            //eprintln!("Mouse coords are: {}, {}", world_position.x, world_position.y)
            *mouse_pos = MousePos(Vec2::new(world_position.x, world_position.y))
        }
}

fn update_bounding_circles(
    mut query: Query<(&mut CircleCollider, &Transform)>
) {
    for (mut collider, transform) in &mut query {
        collider.0 = BoundingCircle {
            center: transform.translation.xy(),
            ..collider.0
        }
    }
}

fn mouse_input(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<MousePos>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if mouse_input.just_pressed(MouseButton::Left) {
        create_gravitable(&mut commands, &mut meshes, &mut materials, mouse_pos.0);
    }

    if mouse_input.just_pressed(MouseButton::Right) {
        create_stationary(&mut commands, &mut meshes, &mut materials, mouse_pos.0);
    }
}

fn create_gravitable(
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
        Gravitable,
        Gravitator,
        Velocity(Vec2::new(0.0, 0.0)),
        CircleCollider(BoundingCircle::new( pos, 10.0))
    ));
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
        CircleCollider(BoundingCircle::new( pos, 10.0))
    ));
}

fn detect_collisions(
    mut circle_one: Query<(&Transform, Option<&mut Velocity>, &CircleCollider)>,
) {
    let mut others = vec![];

    for (transform, _, collider) in circle_one.iter() {
        others.push((transform.clone(), collider.clone()))
    }

    for (pos_one, mut velocity_one, collider_one) in &mut circle_one {
        for (pos_two, collider_two) in others.iter() {

            let are_colliding = pos_two.translation.distance(pos_one.translation) < collider_two.0.radius() + collider_one.0.radius();

            if pos_one != pos_two && are_colliding {
                let normal = (pos_one.translation.xy() - pos_two.translation.xy()).normalize();

                if let Some(ref mut velo) = velocity_one {
                    velo.0 = velo.0.reflect(normal) * 1.0
                }
            }
        }
    }
}

fn draw_arrows(
    show_arrows: Res<ShowArrows>,
    mut gizmos: Gizmos,
    query: Query<(&Velocity, &Transform)>,
) {
    if show_arrows.0 {
        query.iter().for_each(|(velocity, transform)| {
            gizmos.arrow_2d(
                transform.translation.xy(),
                transform.translation.xy() + velocity.0.xy() / 5.0,
                Color::srgb(0.1, 0.4, 0.6),
            );
        })
    }
}
