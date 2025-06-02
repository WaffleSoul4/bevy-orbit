use avian2d::prelude::OnCollisionStart;
use bevy::prelude::*;

use crate::helper::add_observer_on_hook;

#[derive(Debug)]
pub enum DeathSource {
    Reset,
    Collision,
}

/// Defines whether an entity can give a death event (it can die)
#[derive(Component)]
#[component(on_add = add_observer_on_hook(death_event_handler))]
pub struct DeathEventsEnabled;

/// Defines whether an entity can kill entities with death events enabled on collision
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct KillOnCollision;

/// Defines whether an entity will die upon collision (I might replace this with a DeathLayers thing later)
#[derive(Component)]
#[component(on_add = add_observer_on_hook(collision_observer))]
#[require(avian2d::prelude::CollisionEventsEnabled)]
pub struct DieOnCollision;

fn collision_observer(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    query: Query<(), With<KillOnCollision>>,
) {
    if query.contains(trigger.collider) {
        commands.trigger_targets(DeathEvent::new(DeathSource::Collision), trigger.target());
    }
}

fn death_event_handler(trigger: Trigger<DeathEvent>, mut commands: Commands) {
    info!("Object died from {:?}", trigger.source);

    commands.entity(trigger.target()).despawn();
}

#[derive(Event, Debug)]
pub struct DeathEvent {
    source: DeathSource,
}

impl DeathEvent {
    pub fn new(source: DeathSource) -> Self {
        DeathEvent { source }
    }
}
