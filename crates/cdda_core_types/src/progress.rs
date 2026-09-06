//! Presentation-independent operation reporting for GUI and terminal consumers.
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportLevel {
    Progress,
    Warning,
    Error,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportEvent {
    pub stage: String,
    pub message: String,
    pub level: ReportLevel,
    /// Completed units and known total. None means indeterminate, never fake percent.
    pub units: Option<(usize, usize)>,
}
impl ReportEvent {
    pub fn progress(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            message: message.into(),
            level: ReportLevel::Progress,
            units: None,
        }
    }
    pub fn units(mut self, done: usize, total: usize) -> Self {
        self.units = Some((done.min(total), total));
        self
    }
    pub fn level(mut self, level: ReportLevel) -> Self {
        self.level = level;
        self
    }
}
impl fmt::Display for ReportEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:?}] {}",
            self.level,
            self.stage
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect::<String>()
        )?;
        if let Some((done, total)) = self.units {
            write!(f, " {done}/{total}")?;
        }
        // File names and parser details must never inject terminal control codes.
        write!(
            f,
            ": {}",
            self.message
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_records_have_real_units_and_escape_control_characters() {
        let event = ReportEvent::progress("Parsing\n", "file\n\u{1b}[31m").units(7, 4);
        assert_eq!(event.units, Some((4, 4)));
        let text = event.to_string();
        assert!(text.contains("4/4"));
        assert!(!text.chars().any(char::is_control));
    }
}
