use serde::Deserialize;
use serde_yml::Value;
use std::collections::HashMap;

pub mod loader;
pub mod compiler;

type TargetEntry = HashMap<String, Value>;

#[derive(Debug, Deserialize)]
pub struct Emakefile {
    pub path: Option<String>,
    pub targets: HashMap<String, Vec<TargetEntry>>,
}
