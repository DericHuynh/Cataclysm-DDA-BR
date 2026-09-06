//! Retained operation read model. Producers publish facts; presenters choose layout.
use bevy_ecs::prelude::*;
pub use cdda_core_types::progress::{ReportEvent, ReportLevel};
use std::collections::VecDeque;

#[derive(Resource, Default, Debug)]
pub struct OperationReport {
    pub current: Option<ReportEvent>,
    pub history: VecDeque<ReportEvent>,
    pub warnings: usize,
    pub errors: usize,
    pub finished: bool,
    pub cancelled: bool,
}
impl OperationReport {
    pub fn record(&mut self, event: ReportEvent) {
        match event.level {
            ReportLevel::Warning => self.warnings += 1,
            ReportLevel::Error => self.errors += 1,
            ReportLevel::Complete => self.finished = true,
            ReportLevel::Progress => {}
        }
        if self.history.back() != Some(&event) {
            if self.history.len() == 128 {
                self.history.pop_front();
            }
            self.history.push_back(event.clone());
        }
        self.current = Some(event);
    }
    pub fn failed(&self) -> bool {
        self.errors > 0
    }
    pub fn summary(&self) -> String {
        format!("{} warnings · {} errors", self.warnings, self.errors)
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub enum OperationCommand {
    Retry,
    ReturnToMenu,
}
