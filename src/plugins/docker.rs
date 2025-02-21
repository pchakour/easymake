use serde_yml::Value;

use super::Plugin;
pub static ID: &str = "docker";

pub struct Docker;

impl Plugin for Docker {
    fn action(&self, _cwd: &String, _args: &Value, _in_files: &Vec<String>, _out_files: &Vec<String>, _working_dir: &String) -> () {
        println!("Run command docker");
    }

    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync> {
        Box::new(Self)
    }
}
