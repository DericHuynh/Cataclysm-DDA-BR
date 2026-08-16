//! [`HtnDomain`] — the parsed, validated network of tasks.

use bevy_reflect::{Reflect, TypeRegistry};

use crate::error::HtnResult;
use crate::operators::verify_operator;
use crate::tasks::{GoalTask, Task};

/// A parsed and validated HTN domain. Immutable after parse; both planners read
/// it.
#[derive(Debug, Clone)]
pub struct HtnDomain {
    /// The schema version string from the `.htn` file.
    pub schema: String,
    /// All tasks in declaration order.
    pub tasks: Vec<Task>,
}

impl HtnDomain {
    /// The schema version string.
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// The default forward-planning root: the first compound task. Falls back to
    /// the first task if no compound exists.
    pub fn root_task(&self) -> Option<&Task> {
        self.tasks
            .iter()
            .find(|t| t.is_root_compound())
            .or_else(|| self.tasks.first())
    }

    /// Look up a task by name.
    pub fn get_task(&self, name: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.name() == name)
    }

    /// Look up a goal task by name (for back-planning).
    pub fn goal(&self, name: &str) -> Option<&GoalTask> {
        match self.get_task(name) {
            Some(Task::Goal(g)) => Some(g),
            _ => None,
        }
    }

    /// Validate every task's conditions/effects reference existing state fields,
    /// enums are registered, and every operator is registered.
    pub fn verify<S: Reflect>(&self, state: &S, registry: &TypeRegistry) -> HtnResult<()> {
        let erased = state.as_reflect();
        for task in &self.tasks {
            task.verify(erased, registry)?;
            if let Task::Primitive(p) = task {
                verify_operator(registry, &p.operator.name, &p.operator.params)?;
            }
        }
        Ok(())
    }

    /// Validate tasks that don't reference operators (conditions + effects only).
    /// Useful for tests that check planner output without registered operators.
    pub fn verify_without_operators<S: Reflect>(
        &self,
        state: &S,
        registry: &TypeRegistry,
    ) -> HtnResult<()> {
        let erased = state.as_reflect();
        for task in &self.tasks {
            task.verify(erased, registry)?;
        }
        Ok(())
    }

    /// The name of every primitive task in the domain (for back-planning).
    pub fn primitive_names(&self) -> Vec<String> {
        self.tasks
            .iter()
            .filter_map(|t| match t {
                Task::Primitive(p) => Some(p.name.clone()),
                _ => None,
            })
            .collect()
    }
}
