use crate::serialization;
use bevy::prelude::*;
use bevy_egui::{
    EguiContexts,
    egui::{self},
};

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CreateObject>();
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
    mut camera: Single<&mut Camera, With<crate::camera::GameCamera>>,
    mut load_events: EventWriter<serialization::LoadEvent>,
    mut save_events: EventWriter<serialization::SaveEvent>,
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
                        "assets/test_levels/level2.scn.ron",
                        "Abcdefg",
                    ));
                }
                if ui.button("Load level").clicked() {
                    load_events.write(serialization::LoadEvent::new("test_levels/level2.scn.ron"));
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
