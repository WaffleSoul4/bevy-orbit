use crate::{
    gravity::Gravitator, level::{EntityType, LevelDescriptor}, GameCamera, StaticObject
};
use avian2d::prelude::Mass;
use bevy::prelude::*;
use bevy_egui::{egui::{self}, EguiContexts};
use std::{ops::Mul, path::PathBuf};

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

 // I'm going to wait until avian and bevy egui have 0.16 support
 // because Serializing right now is much less fun.
#[derive(Component)]
struct GameSerializable;

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

pub fn save_level(
    statics: Query<
        (
            &Transform,
            &MeshMaterial2d<ColorMaterial>,
            Option<&Gravitator>,
            &Mass,
        ),
        With<StaticObject>,
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

pub fn load_level(
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
                    save_events.send(SaveEvent::new(
                        PathBuf::from("test_levels/level2"),
                        "Abcdefg",
                    ));
                }
                if ui.button("Load level").clicked() {
                    load_events.send(LoadEvent::new(
                        PathBuf::from("test_levels/level2"),
                    ));
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