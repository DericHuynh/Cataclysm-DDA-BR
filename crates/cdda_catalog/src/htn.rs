use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Compound {
    pub parameters: Vec<String>,
    pub methods: Vec<Method>,
}
#[derive(Clone, Debug)]
pub struct Method {
    pub id: Option<String>,
    pub when: Vec<Predicate>,
    pub steps: Vec<Step>,
}
#[derive(Clone, Debug)]
pub struct Predicate {
    pub predicate: String,
    pub args: Value,
}
#[derive(Clone, Debug)]
pub struct Step {
    pub operator: Option<String>,
    pub task: Option<String>,
    pub args: Value,
}
#[derive(Clone, Default)]
pub struct HtnProgram {
    pub items: HashSet<String>,
    pub item_categories: HashSet<String>,
    pub htn_compounds: HashMap<String, Arc<Compound>>,
}
pub trait HtnSource {
    fn htn_program(&self) -> HtnProgram;
}
impl HtnSource for HtnProgram {
    fn htn_program(&self) -> HtnProgram {
        self.clone()
    }
}
