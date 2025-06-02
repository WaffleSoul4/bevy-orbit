use crate::serialization;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

pub fn side_menu(
    mut contexts: EguiContexts,
    window: Single<&Window>,
    mut camera: Single<&mut Camera, With<crate::camera::GameCamera>>,
    mut save_events: EventWriter<serialization::SaveEvent>,
    mut serialization_data: ResMut<serialization::LevelSerializationData>,
) {
    // It makes the code look so much better
    use std::ops::Mul;

    let contexts = contexts.ctx_mut();

    let ui_width = egui::SidePanel::left("Editor Panel")
        .resizable(true)
        .show(contexts, |ui| {
            let mut path_buffer = serialization_data.path.display().to_string();

            ui.text_edit_singleline(&mut path_buffer);
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

            serialization_data.path = path_buffer.into();
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
