use bevy::{
    ecs::{component::HookContext, world::DeferredWorld},
    prelude::*,
};

#[derive(Component)]
pub struct Traceable;

// Traces the path of an entity and spawns the path as its children
#[derive(Component)]
#[component(on_add = get_starting_position)]
pub struct PathTracer {
    previous: Vec2,
    precision: u32, // Zero is every frame
    min_length: f32,
    width: f32,
    color: Color,
    precision_counter: u32,
    target: Option<Entity>, // No entity means disabled
}

// Mmm tasty scopes
fn get_starting_position(mut world: DeferredWorld, context: HookContext) {
    let tracer_target_entity = {
        let mut tracer_commands = world.entity_mut(context.entity);

        tracer_commands
            .get_mut::<PathTracer>()
            .expect("What...")
            .target
            .clone()
    };

    let target_transform = {
        world
            .get_entity(
                tracer_target_entity.expect("Please provide a target when initializing tracers"),
            )
            .expect("Invalid target entity found for tracer")
            .get::<Transform>()
            .expect("Tracer target doesn't have a transfor to trace")
            .clone()
    };

    // Duplication out of necessity

    let mut tracer_commands = world.entity_mut(context.entity);

    let mut tracer = tracer_commands
        .get_mut::<PathTracer>()
        .expect("This is a hook for if this component was added ofc it's here");

    tracer.previous = target_transform.translation.xy();
}

impl PathTracer {
    pub fn new(target: Entity) -> Self {
        PathTracer {
            previous: Vec2::ZERO,
            precision: 1, // Every other frame
            min_length: 3.0,
            width: 2.0,
            color: Color::srgb(0.1, 0.3, 0.7),
            precision_counter: 0,
            target: Some(target),
        }
    }

    pub fn increment(&mut self) {
        self.precision_counter += 1
    }

    pub fn reset(&mut self) {
        self.precision_counter = 0
    }
}

#[derive(Component)]
pub struct PathSegment;

pub fn trace_object_paths(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    mut tracers: Query<(Entity, &mut PathTracer)>,
    traceable: Query<&GlobalTransform, With<Traceable>>,
) {
    tracers
        .iter_mut()
        .filter(|(_, tracer)| tracer.target.is_some())
        .for_each(|(entity, mut tracer)| {
            if tracer.precision_counter >= tracer.precision {
                match traceable.get(tracer.target.unwrap()) {
                    Ok(transform) => {
                        let difference = transform.translation().xy() - tracer.previous;

                        if difference.length() > tracer.min_length {
                            let length = difference.length() * 1.5;

                            let rectangle = Rectangle::from_size(Vec2::new(tracer.width, length));

                            let angle = difference.to_angle() + std::f32::consts::PI / 2.0; // Add 90 degrees

                            let segment = commands
                                .spawn((
                                    MeshMaterial2d(materials.add(tracer.color)),
                                    Mesh2d(meshes.add(rectangle)),
                                    Transform::from_rotation(Quat::from_rotation_z(angle))
                                        .with_translation(
                                            transform.translation().xy().extend(-1.0),
                                        ),
                                    PathSegment,
                                ))
                                .id();

                            commands.entity(entity).add_child(segment);

                            tracer.previous = transform.translation().xy();
                        }
                    }
                    Err(e) => {
                        info!("Tracer failed to find target: {}, Disabling", e);

                        tracer.target = None;
                    }
                }

                tracer.reset();
            } else {
                tracer.increment();
            }
        });
}
