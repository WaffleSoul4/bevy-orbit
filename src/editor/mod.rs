mod ui;

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use crate::{
    game::{
        death::KillOnCollision,
        gravity::{Gravity, GravityLayer, GravityLayers},
    },
    helper::app_state_is,
    serialization::{
        GameSerializable, LevelObject, colliders::SerializableCollider,
        materials::SerilializableMeshMaterial, meshes::SerializableMesh,
    },
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
    kill_on_collision: KillOnCollision,
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

    pub fn with_position(self, translation: Vec2) -> Self {
        self.with_translation(translation.extend(0.0))
    }

    #[allow(dead_code)]
    pub fn with_velocity(mut self, velocity: Vec2) -> Self {
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
            kill_on_collision: KillOnCollision,
        }
    }
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin {
                enable_multipass_for_primary_context: false,
            });
        }

        app.add_systems(
            Update,
            (editor_input_handler, ui::side_menu).run_if(app_state_is(crate::AppState::Editor)),
        );
    }
}

pub fn editor_input_handler(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_position: Res<crate::cursor::CursorPosition>,
    mut starting_position: ResMut<crate::StartPoint>,
    mut commands: Commands,
) {
    if let Some(cursor_position) = **cursor_position {
        if keys.pressed(KeyCode::ShiftLeft) {
            // Stuff will go here!
        } else {
            if mouse.just_pressed(MouseButton::Right) {
                commands.spawn(
                    LevelObjectBundle::from_circle(Circle::new(10.0))
                        .with_position(cursor_position),
                );
            }

            if mouse.just_pressed(MouseButton::Middle) {
                **starting_position = Some(cursor_position)
            }

            if keys.just_pressed(KeyCode::KeyZ) {
                // I'll add a bundle for this later
                commands.spawn(
                    crate::game::trigger::GameTriggerBundle::default()
                        .with_position(cursor_position),
                );
            }
        }
    }
}
