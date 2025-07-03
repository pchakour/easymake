use serde::Deserialize;
use serde_yml::Value;
use std::collections::HashMap;

pub mod loader;
pub mod compiler;

pub type TargetEntry = HashMap<String, Value>;
pub type CredentialEntry = HashMap<String, Value>;
pub type VariableEntry = String;

#[derive(Debug, Deserialize, Clone)]
pub struct Emakefile {
    pub path: Option<String>,
    pub credentials: Option<HashMap<String, CredentialEntry>>,
    pub variables: Option<HashMap<String, VariableEntry>>,
    pub targets: HashMap<String, Vec<TargetEntry>>,
}
