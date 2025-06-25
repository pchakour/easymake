use std::{collections::HashMap, fmt::Display};

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
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {:?})", self.plugin_id, self.args)
    }
}

#[derive(Debug, Clone)]
struct InFile{
    file: String,
    credentials: Option<String>,
}

#[derive(Debug, Clone)]
struct Node {
    id: String,
    in_neighbors: Vec<String>,
    out_neighbors: Vec<String>,
    action: Option<Action>,
    in_files: Vec<InFile>,
    out_files: Vec<String>,
    cwd: String,
}

#[derive(Debug)]
pub struct Graph {
    nodes: HashMap<String, Node>,
    root: String,
}
