use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use serde_yml::Value;
use std::path::Path;
use std::path::PathBuf;

use crate::cache;
use crate::graph::InFile;

pub fn get_absolute_target_path(
    path: &String,
    emakefile_current_path: &String,
    cwd: &str,
) -> String {
    let path_separator = String::from("/");
    if path.starts_with("//") {
        let mut path_parts: Vec<&str> = path.split(&path_separator).collect();
        let mut target_key = path_parts.pop().unwrap();
        let mut target_key_parts: Vec<&str> = target_key.split(':').collect();
        target_key = target_key_parts.pop().unwrap();

        path_parts.join(&path_separator) + "/targets:" + target_key
    } else {
        let mut path_parts: Vec<&str> = path
            .split(&path_separator)
            .filter(|part| !part.is_empty())
            .collect();
        let target_key = path_parts.pop().unwrap();
        let mut target_key_parts: Vec<&str> = target_key.split(':').collect();
        let target_key = target_key_parts.pop().unwrap();
        let parent_target_path = (path_separator
            + Path::new(emakefile_current_path)
                .parent()
                .unwrap()
                .to_str()
                .unwrap())
        .replace(cwd, "");
        let target_path;
        if path_parts.len() > 0 {
            target_path = format!(
                "{}/{}/targets:{}",
                &parent_target_path,
                path_parts.join("/"),
                target_key
            );
        } else {
            target_path = format!("{}/targets:{}", &parent_target_path, target_key);
        }
        // println!(
        //     "Transform target {} to {} with cwd={:?} and emakefile={}",
        //     path, target_path, cwd, emakefile_current_path
        // );
        target_path
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlugin {
    pub type_id: String,
    pub args: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionProperties {
    pub plugin: ActionPlugin,
    pub in_files: Vec<InFile>,
    pub out_files: Vec<String>,
    pub checksum: Option<String>,
    pub clean: Option<String>,
    pub then_targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetProperties {
    pub actions: Vec<ActionProperties>,
}

pub fn to_footprint_path(target_absolute_path: &str, _cwd: &str) -> PathBuf {
    let footprints_dir = cache::get_footprints_dir_path();
    let footprint_path: String = target_absolute_path
        .replace("targets:", "_targets_/")
        .replace("//", "");
    Path::new(&footprints_dir).join(footprint_path)
}

pub fn to_emakefile_path(target_absolute_path: &str, root_folder: &str) -> PathBuf {
    let re = Regex::new(r"targets:.+$").unwrap();
    let mut result = re.replace(target_absolute_path, "").to_string();
    result = result.replace("//", "");
    let target_path_parts = Path::new(root_folder).join(result).join("Emakefile");
    target_path_parts
}
