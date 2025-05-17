use crate::{
    gravity::{Gravitable, Gravitator, GravityLayer, GravityLayers}, DynamicObject, GameCamera, SerializableCollider, StaticObject
};
use avian2d::prelude::{Collider, ColliderConstructor, Mass};
use bevy::{prelude::*, reflect::TypeRegistry};
use bevy_egui::{
    EguiContexts,
    egui::{self},
};
use std::{
    fs::File,
    io::{BufWriter, Read, Write},
    ops::Mul,
    path::PathBuf,
};

#[derive(Event)]
pub struct SaveEvent {
    file: PathBuf,
    level_name: String,
}

impl SaveEvent {
    pub fn new(file: PathBuf, level_name: &str) -> Self {
        SaveEvent {
            file,
            level_name: level_name.to_string(),
        }
    }
}

#[derive(Event)]
pub struct LoadEvent {
    file: PathBuf,
}

impl LoadEvent {
    pub fn new(file: PathBuf) -> Self {
        LoadEvent { file }
    }
}

/// Put types that need to be serialized in here
pub struct SerializeableTypeRegistrationPlugin;

impl Plugin for SerializeableTypeRegistrationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Gravitable>()
            .register_type::<Gravitator>()
            .register_type::<GravityLayers>()
            .register_type::<crate::Trigger>()
            .register_type::<StaticObject>()
            .register_type::<DynamicObject>()
            .register_type::<SerializableCollider>();
    }
}

#[derive(Component)]
pub struct GameSerializable;

pub fn serialize_objects(
    mut events: EventReader<SaveEvent>,
    world: &World,
    entities: Query<Entity, With<GameSerializable>>,
    type_registry: Res<AppTypeRegistry>,
) {
    let type_registry = type_registry.read();

    for event in events.read() {
        let scene_builder = DynamicSceneBuilder::from_world(world)
            .deny_all()
            .allow_component::<SerializableCollider>()
            .allow_component::<crate::Trigger>()
            .allow_component::<Gravitable>()
            .allow_component::<Gravitator>()
            .allow_component::<Mass>()
            .allow_component::<StaticObject>()
            .allow_component::<DynamicObject>()
            .allow_component::<Transform>();

        let scene = scene_builder
            .extract_entities(entities.iter())
            .build();

        let serialized = scene.serialize(&type_registry).unwrap_or_else(|err| {
            error!("Failed to serialize scene: {err}");
            String::from("Error Serializing")
        });

        // @TODO: Add error handling
        File::create(event.file.clone())
            .unwrap_or_else(|err| panic!("Failed to open file '{}': {err}", event.file.display()))
            .write_all(serialized.as_bytes())
            .unwrap();
    }
}

pub fn deserialize_objects(
    mut save_events: EventReader<LoadEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for event in save_events.read() {
        commands.spawn(DynamicSceneRoot(asset_server.load(event.file.clone())));
    }
}

// Basically I want to be able to store data ([Gravitable], [Gravitator])
// but not actually have them enabled
// This is a mediocre solution
#[derive(Component)]
pub struct SelectedDynamicConfig {
    pub gravitable: bool,
    pub gravitator: bool,
    pub radius: f32,
    // Mass being here just feels more consistent ig
    pub mass: f32,
}

impl SelectedDynamicConfig {
    pub fn new(gravitable: bool, gravitator: bool, radius: f32, mass: f32) -> Self {
        SelectedDynamicConfig {
            gravitable,
            gravitator,
            radius,
            mass,
        }
    }
}

#[derive(Event, Clone, Copy)]
pub enum CreateObject {
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
    pub fn new_static(mass: f32, position: Vec2, radius: f32) -> Self {
        CreateObject::Static {
            mass,
            position,
            radius,
        }
    }

    pub fn new_dynamic(mass: f32, position: Vec2, radius: f32) -> Self {
        CreateObject::Dynamic {
            mass,
            position,
            radius,
            gravitable: true,
            gravitator: true,
            selected: false,
        }
    }

    pub fn new_trigger(position: Vec2) -> Self {
        CreateObject::Trigger { position }
    }

    // Only run this on dynamics please <3
    pub fn set_selected(&mut self) -> &mut Self {
        match self {
            CreateObject::Dynamic { selected, .. } => {
                *selected = true;
            }
            _ => panic!("Called set_select on a non-dynamic object"),
        }

        self
    }
}

pub fn side_menu(
    mut contexts: EguiContexts,
    window: Single<&Window>,
    mut camera: Single<&mut Camera, With<GameCamera>>,
    mut load_events: EventWriter<LoadEvent>,
    mut save_events: EventWriter<SaveEvent>,
) {
    let contexts = contexts.ctx_mut();

    let ui_width = egui::SidePanel::left("Editor Panel")
        .resizable(true)
        .show(contexts, |ui| {
            ui.label("This will be where the editor is!");
            ui.horizontal(|ui| {
                if ui.button("Save level").clicked() {
                    eprintln!("Pressed button");
                    save_events.write(SaveEvent::new(
                        PathBuf::from("assets/test_levels/level2.scn.ron"),
                        "Abcdefg",
                    ));
                }
                if ui.button("Load level").clicked() {
                    load_events.write(LoadEvent::new(PathBuf::from("test_levels/level2.scn.ron")));
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
