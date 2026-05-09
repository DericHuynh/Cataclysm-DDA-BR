use bevy_app::{App, Plugin};
use crate::combat::systems::combat_phase;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, _app: &mut App) {
        let _ = combat_phase;
    }
}
