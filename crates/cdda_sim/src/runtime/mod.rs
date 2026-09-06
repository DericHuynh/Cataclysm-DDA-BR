//! Runtime harness — `AppState`, `GameTime`, the simulation clock, `TestBed`.

pub mod clock;
pub mod plugin;
pub mod state;
pub mod test_utils;

pub use clock::{SimClock, DEFAULT_SIM_ROUND};
pub use plugin::{
    drive_simulation, step_simulation, SimulationControl, SimulationMode, SimulationPlugin,
};
