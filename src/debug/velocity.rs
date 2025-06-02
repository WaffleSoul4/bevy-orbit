use avian2d::prelude::*;
use bevy::prelude::*;

use crate::game::launch::Launching;

pub fn draw_velocity_arrows(
    mut gizmos: Gizmos,
    mouse_pos: Res<crate::cursor::CursorPosition>,
    dynamic_object_query: Query<(&LinearVelocity, &Transform), With<Mesh2d>>,
    selected_object_query: Query<&Transform, With<Launching>>,
) {
    dynamic_object_query
        .iter()
        .for_each(|(velocity, transform)| {
            gizmos.arrow_2d(
                transform.translation.xy(),
                transform.translation.xy() + velocity.0.xy() / 6.0,
                Color::srgb(0.1, 0.4, 0.6),
            );
        });

    // Draw arrows for objects that are currently [Selected]
    selected_object_query.iter().for_each(|transform| {
        let dif = transform.translation.xy() - mouse_pos.unwrap_or(transform.translation.xy());

        gizmos.arrow_2d(
            transform.translation.xy(),
            transform.translation.xy() + dif / 6.0,
            Color::srgb(0.1, 0.4, 0.6),
        );
    });
}
