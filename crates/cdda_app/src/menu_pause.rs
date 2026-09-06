//! Pause-menu adapter preserves a pre-existing simulation pause.
use bevy_ecs::prelude::*;
use cdda_sim::runtime::SimulationControl;
#[derive(Resource)]
struct PreviousPause(bool);
pub fn enter(world: &mut World) {
    let previous = world.resource::<SimulationControl>().paused;
    world.insert_resource(PreviousPause(previous));
    world.resource_mut::<SimulationControl>().paused = true;
}
pub fn exit(world: &mut World) {
    if let Some(previous) = world.remove_resource::<PreviousPause>() {
        world.resource_mut::<SimulationControl>().paused = previous.0;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closing_pause_menu_restores_the_previous_pause_state() {
        for paused in [false, true] {
            let mut world = World::new();
            world.init_resource::<SimulationControl>();
            world.resource_mut::<SimulationControl>().paused = paused;
            enter(&mut world);
            assert!(world.resource::<SimulationControl>().paused);
            exit(&mut world);
            assert_eq!(world.resource::<SimulationControl>().paused, paused);
        }
    }
}
