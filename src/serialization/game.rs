use super::ActiveLevel;
use bevy::prelude::*;

pub fn load_active_level(
    level_serialization_data: Res<super::LevelSerializationData>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    info!(
        "Loaded level as scene from {:?}",
        level_serialization_data.path
    );

    commands.spawn((
        DynamicSceneRoot(asset_server.load(level_serialization_data.path.clone())),
        ActiveLevel,
    ));
}

pub fn remove_active_level(
    mut commands: Commands,
    active_level: Single<Entity, With<ActiveLevel>>,
) {
    commands.entity(active_level.into_inner()).despawn();
}
