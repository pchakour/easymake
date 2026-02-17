use serde::{Deserialize, Serialize};

pub mod generator;
pub mod runner;
pub mod viewer;
pub mod common;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InFile {
    pub file: String,
    pub secrets: Option<String>,
}