use std::collections::HashMap;

use serde_yml::Value;

mod cmd;
mod docker;

pub trait Plugin: Send + Sync {
    fn action(&self, cwd: &String, args: &Value, in_files: &Vec<String>, out_file: &Vec<String>, working_dir: &String) -> ();
    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync>;
}

impl Clone for Box<dyn Plugin + Send + Sync> {
    fn clone(&self) -> Box<dyn Plugin + Send + Sync> {
        self.clone_box()
    }
}

pub struct PluginsStore {
    plugins: HashMap<String, Box<dyn Plugin + Send + Sync>>
}

impl PluginsStore {
    pub fn add(mut self, key: &String, plugin: Box<dyn Plugin + Send + Sync>) -> PluginsStore {
        self.plugins.insert(key.clone(), plugin);
        self
    }

    pub fn get(&self, plugin_id: &String) -> Option<&Box<dyn Plugin + Send + Sync>> {
        self.plugins.get(plugin_id)
    }
}

pub fn instanciate() -> PluginsStore {
    PluginsStore {
        plugins: HashMap::new(),
    }
    .add(&String::from(cmd::ID), Box::new(cmd::Cmd))
    .add(&String::from(docker::ID), Box::new(docker::Docker))
}
