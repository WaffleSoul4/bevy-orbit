use crate::{
    cursor::CursorPosition, gravity::{Gravity, GravityLayer, GravityLayers}, serialization::{
        self, GameSerializable, SerializableCollider, SerializableMesh, SerilializableMeshMaterial,
    }, state_is, GameLayer, LevelObject
};
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_egui::{
    EguiContexts,
    egui::{self},
};

#[derive(Bundle)]
struct LevelObjectBundle {
    transform: Transform,
    mesh: SerializableMesh,
    material: SerilializableMeshMaterial,
    mass: Mass,
    gravity: Gravity,
    collider: SerializableCollider,
    rigid_body: RigidBody,
    gravity_layers: GravityLayers,
    velocity: LinearVelocity,
    level_object: LevelObject,
    game_serializable: GameSerializable,
}

impl LevelObjectBundle {
    fn from_circle(value: Circle) -> Self {
        LevelObjectBundle {
            mesh: SerializableMesh::primitive(value.clone()),
            collider: SerializableCollider::from(value.clone()),
            ..default()
        }
    }

    fn with_translation(mut self, translation: Vec3) -> Self {
        self.transform.translation = translation;
        self
    }

    fn with_translation_2d(self, translation: Vec2) -> Self {
        self.with_translation(translation.extend(0.0))
    }

    fn with_velocity(mut self, velocity: Vec2) -> Self {
        self.velocity = LinearVelocity(velocity);
        self.rigid_body = RigidBody::Dynamic;
        self
    }
}

impl Default for LevelObjectBundle {
    fn default() -> Self {
        LevelObjectBundle {
            transform: Transform::from_translation(Vec3::ZERO),
            mesh: SerializableMesh::primitive(Circle::new(10.0)),
            material: SerilializableMeshMaterial::color(Color::oklab(1.0, 0.7, 0.3)),
            mass: Mass(5.0),
            gravity: Gravity,
            collider: SerializableCollider::new(ColliderConstructor::Circle { radius: 10.0 }),
            rigid_body: RigidBody::Static,
            gravity_layers: GravityLayers::new(
                [GravityLayer::Level],
                [GravityLayer::Main, GravityLayer::Level],
            ),
            velocity: LinearVelocity(Vec2::ZERO),
            level_object: LevelObject,
            game_serializable: GameSerializable,
        }
    }
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, editor_input_handler.run_if(state_is(crate::GameState::Editor)));
    }
}


pub fn editor_input_handler(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_position: Res<CursorPosition>,
    mut commands: Commands,
) {
    if let Some(cursor_position) = **cursor_position {
        if keys.pressed(KeyCode::ShiftLeft) {
            // Stuff will go here!
        } else {
            if mouse.just_pressed(MouseButton::Right) {
                commands.spawn(
                    LevelObjectBundle::from_circle(Circle::new(10.0))
                        .with_translation_2d(cursor_position),
                );
            }

            if keys.just_pressed(KeyCode::KeyZ) {
                // I'll add a bundle for this later
                commands.spawn((
                    SerializableMesh::primitive(Circle::new(10.0)),
                    Transform::from_translation(cursor_position.extend(-1.0)),
                    SerilializableMeshMaterial::color(Color::srgb(0.1, 0.3, 0.7)),
                    crate::Trigger::new(false),
                    SerializableCollider::new(ColliderConstructor::Circle { radius: 10.0 }),
                    CollisionLayers::new(GameLayer::Triggers, [GameLayer::Main]),
                    GameSerializable,
                ));
            }
        }
    }
}

pub fn side_menu(
    mut contexts: EguiContexts,
    window: Single<&Window>,
    mut camera: Single<&mut Camera, With<crate::camera::GameCamera>>,
    mut save_events: EventWriter<serialization::SaveEvent>,
    serialization_data: Res<serialization::LevelSerializationData>,
) {
    // It makes the code look so much better
    use std::ops::Mul;

    let contexts = contexts.ctx_mut();

    let ui_width = egui::SidePanel::left("Editor Panel")
        .resizable(true)
        .show(contexts, |ui| {
            ui.label("This will be where the editor is!");
            ui.horizontal(|ui| {
                if ui.button("Save level").clicked() {
                    save_events.write(serialization::SaveEvent::new(
                        serialization_data.path.clone(),
                    ));
                }
                if ui.button("Load level").clicked() {
                    warn!("Nothing here yet!");
                }
            });

            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .width()
        .mul(window.scale_factor())
        .round() as u32;

    // Spent a long time here
    // I pulled this ui code from the examples, and just expected
    // it to work. It does work... sort of. The rect seems to be provided
    // in logical pixels, not physical pixels.

    // Edit: I just found the part in the code where they multiply by
    // logical pixels, so I'm going to learn how to read properly 3:

    camera.viewport = Some(bevy::render::camera::Viewport {
        physical_position: UVec2::new(ui_width, 0),
        physical_size: UVec2::new(window.physical_width() - ui_width, window.physical_height()),
        ..default()
    });
}
