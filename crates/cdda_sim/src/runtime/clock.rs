//! Wall-time adapter for the logical simulation, not a game clock.
//!
//! The default driver is turn-based. Only explicit real-time mode consumes this
//! accumulator. A long frame may yield several turns; bounded work carries its
//! remainder forward instead of losing elapsed time. Pausing clears wall debt.

use bevy_ecs::prelude::Resource;
use std::time::Duration;

/// Wall pacing of the optional real-time driver. One logical turn is one game
/// second regardless of this setting (see `GameTime`).
pub const DEFAULT_SIM_ROUND: Duration = Duration::from_millis(100);

#[derive(Resource, Debug)]
pub struct SimClock {
    step: Duration,
    accumulator: Duration,
}

impl Default for SimClock {
    fn default() -> Self {
        Self::new(DEFAULT_SIM_ROUND)
    }
}

impl SimClock {
    pub fn new(step: Duration) -> Self {
        assert!(!step.is_zero(), "simulation wall step must be positive");
        Self {
            step,
            accumulator: Duration::ZERO,
        }
    }

    pub fn step(&self) -> Duration {
        self.step
    }

    pub fn advance(&mut self, elapsed: Duration) {
        self.accumulator = self.accumulator.saturating_add(elapsed);
    }

    /// Consume at most `limit` complete steps, retaining all unconsumed time.
    pub fn take_steps(&mut self, limit: u32) -> u32 {
        let steps =
            (self.accumulator.as_nanos() / self.step.as_nanos()).min(u128::from(limit)) as u32;
        self.accumulator -= self.step * steps;
        steps
    }

    pub fn reset(&mut self) {
        self.accumulator = Duration::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_fail_open_turns_and_fractional_time_carries() {
        let mut clock = SimClock::default();
        assert_eq!(clock.take_steps(8), 0);
        clock.advance(Duration::from_millis(250));
        assert_eq!(clock.take_steps(8), 2);
        clock.advance(Duration::from_millis(50));
        assert_eq!(clock.take_steps(8), 1);
    }

    #[test]
    fn bounded_catchup_retains_backlog() {
        let mut clock = SimClock::default();
        clock.advance(Duration::from_millis(1050));
        assert_eq!(clock.take_steps(3), 3);
        assert_eq!(clock.take_steps(8), 7);
        clock.reset();
        clock.advance(Duration::from_millis(50));
        assert_eq!(clock.take_steps(8), 0);
    }

    #[test]
    #[should_panic(expected = "must be positive")]
    fn zero_step_is_invalid() {
        SimClock::new(Duration::ZERO);
    }
}
