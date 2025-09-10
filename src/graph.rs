use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use serde_yml::Value;

pub mod generator;
pub mod runner;
pub mod viewer;
pub mod common;
pub mod analysor;

#[derive(Debug, Clone)]
struct Action {
    plugin_id: String,
    args: Value,
    checksum: Option<String>,
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {:?})", self.plugin_id, self.args)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InFile {
    pub file: String,
    pub secrets: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    in_neighbors: Vec<String>,
    out_neighbors: Vec<String>,
    action: Option<Action>,
    in_files: Vec<InFile>,
    out_files: Vec<String>,
    cwd: String,
}

#[derive(Debug)]
pub struct Graph {
    pub nodes: HashMap<String, Node>,
    pub root: String,
}
