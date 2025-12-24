use std::{collections::{HashMap}, future::Future, pin::Pin};
use crate::{emake::{InFile, PluginAction}, graph::generator::to_footprint_path};

pub mod shell;
pub mod copy;
pub mod extract;
pub mod mv;
pub mod remove;
pub mod archive;
pub mod git_clone;
pub mod yaml;

pub fn compute_action_footprint(action: &PluginAction) -> String {
    let serialized = serde_json::to_vec(action).expect("Failed to serialize PluginAction");
    blake3::hash(&serialized).to_hex().to_string()
}

pub async fn get_registered_action_footprint(id: &str, cwd: &str) -> Option<String> {
    let footprint_path = to_footprint_path(id, cwd);
    let footprint_file_exists = tokio::fs::try_exists(&footprint_path).await.unwrap();
    if footprint_file_exists {
        let footprint = tokio::fs::read_to_string(&footprint_path).await.unwrap();
        return Some(footprint);
    }

    None
}

pub async fn register_action_footprint(id: &str, footprint: &str, cwd: &str) {
    let footprint_path = to_footprint_path(id, cwd);
    tokio::fs::create_dir_all(&footprint_path.parent().unwrap()).await.unwrap();
    tokio::fs::write(footprint_path, footprint).await.unwrap();
}

pub trait Action: Send + Sync {
    fn run<'a>(
        &'a self,
        cwd: &'a str,
        target_id: &'a str,
        step_id: &'a str,
        emakefile_cwd: &'a str,
        silent: bool,
        action: &'a PluginAction,
        in_files: &'a Vec<String>,
        out_file: &'a Vec<String>,
        working_dir: &'a String,
        default_replacments: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>>;
    fn get_checksum(&self) -> Option<String>;
    fn insert_in_files<'a>(&'a self, action: &'a PluginAction, in_files: &'a mut Vec<InFile>) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
    fn insert_out_files<'a>(&'a self, action: &'a PluginAction, out_files: &'a mut Vec<String>) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
    fn clone_box(&self) -> Box<dyn Action + Send + Sync>;
}

impl Clone for Box<dyn Action + Send + Sync> {
    fn clone(&self) -> Box<dyn Action + Send + Sync> {
        self.clone_box()
    }
}

pub struct ActionsStore {
    actions: HashMap<String, Box<dyn Action + Send + Sync>>,
}

impl ActionsStore {
    pub fn add(mut self, key: &String, action: Box<dyn Action + Send + Sync>) -> ActionsStore {
        self.actions.insert(key.clone(), action);
        self
    }

    pub fn get(&self, action: &PluginAction) -> Option<&Box<dyn Action + Send + Sync>> {
        match action {
            PluginAction::Shell{ shell: _ } => self.actions.get(shell::ID),
            PluginAction::Copy { copy: _ } => self.actions.get(copy::ID),
            PluginAction::Extract { extract: _ } => self.actions.get(extract::ID),
            PluginAction::Move{ mv: _} => self.actions.get(mv::ID),
            PluginAction::Remove{ remove: _} => self.actions.get(remove::ID),
            PluginAction::Archive{ archive: _} => self.actions.get(archive::ID),
            PluginAction::GitClone{ git_clone: _} => self.actions.get(git_clone::ID),
            PluginAction::Yaml{ yaml: _} => self.actions.get(yaml::ID),
        }
    }
}

pub fn instanciate() -> ActionsStore {
    ActionsStore {
        actions: HashMap::new(),
    }
    .add(&String::from(shell::ID), Box::new(shell::Shell))
    .add(&String::from(copy::ID), Box::new(copy::Copy))
    .add(&String::from(extract::ID), Box::new(extract::Extract))
    .add(&String::from(mv::ID), Box::new(mv::Move))
    .add(&String::from(remove::ID), Box::new(remove::Remove))
    .add(&String::from(archive::ID), Box::new(archive::Archive))
    .add(&String::from(git_clone::ID), Box::new(git_clone::GitClone))
    .add(&String::from(yaml::ID), Box::new(yaml::Yaml))
}
