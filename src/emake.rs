use serde::{Deserialize, Serialize};
use serde_yml::{Value};
use std::collections::HashMap;

use crate::actions::{archive, cmd, copy, extract, mv, remove};

pub mod loader;
pub mod compiler;

pub type CredentialEntry = HashMap<String, Value>;
pub type VariableEntry = String;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Target {
    pub deps: Option<Vec<String>>,
    pub parallel: Option<bool>,
    pub steps: Option<Vec<Step>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum InFile {
    Simple(String),
    Detailed(InFileEntry),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InFileEntry {
    pub file: String,
    pub credentials: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Step {
    #[serde(flatten)]
    pub plugin: PluginAction, // The actual action like cmd/copy
    #[serde(default)]
    pub in_files: Option<Vec<InFile>>,  // or Vec<String>, or a custom type
    #[serde(default)]
    pub out_files: Option<Vec<String>>, // same here
    #[serde(default)]
    pub checksum: Option<String>,
    #[serde(default)]
    pub clean: Option<String>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum PluginAction {
    Cmd { cmd: cmd::CmdAction },
    Copy { copy: copy::CopyAction },
    Extract { extract: extract::ExtractAction },
    Move {
        #[serde(rename = "move")] 
        mv: mv::MoveAction
    },
    Remove {
        remove: remove::RemoveAction
    },
    Archive {
        archive: archive::ArchiveAction
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Emakefile {
    pub path: Option<String>,
    pub credentials: Option<HashMap<String, CredentialEntry>>,
    pub variables: Option<HashMap<String, VariableEntry>>,
    pub targets: HashMap<String, Target>,
}
