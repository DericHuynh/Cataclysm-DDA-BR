use crate::systems::combat_phase;
use bevy_app::{App, Plugin};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, _app: &mut App) {
        let _ = combat_phase;
    }
}
