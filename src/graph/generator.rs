use hex::encode;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use serde_yml::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::create_dir;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::process;

use crate::cache;
use crate::console::log;
use crate::emake;
use crate::emake::loader::extract_info_from_path;
use crate::graph;
use crate::graph::InFile;

const RESERVED_KEYWORDS: [&str; 5] = ["clean", "then", "in_files", "out_files", "checksum"];

fn get_absolute_file_path(cwd: &String, file: &String) -> std::path::PathBuf {
    if file.starts_with("/") || file.starts_with("{{") {
        std::path::PathBuf::from(&file)
    } else {
        let absolute_path = cwd.clone();
        let mut path = std::path::PathBuf::from(&absolute_path);
        path.push(file);
        path
    }
}

fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    encode(result)
}

fn create_action_node(
    plugin_id: &String,
    args: &Value,
    cwd: String,
    in_files: &Vec<InFile>,
    out_files: &Vec<String>,
    checksum: &Option<String>,
) -> graph::Node {
    let args_as_str = format!("{:?} {:?} {:?}", in_files, out_files, args);
    graph::Node {
        id: format!("{}|{}", plugin_id, compute_sha256(&args_as_str)),
        action: Some(graph::Action {
            plugin_id: plugin_id.clone(),
            args: args.clone(),
            checksum: checksum.to_owned(),
        }),
        out_neighbors: Vec::new(),
        in_neighbors: Vec::new(),
        in_files: Vec::new(),
        out_files: Vec::new(),
        cwd,
    }
}

fn extract_in_file_value(value: &Value) -> Vec<InFile> {
    if value.is_sequence() {
        let in_files = value.as_sequence().unwrap();

        let mut result: Vec<InFile> = Vec::new();
        for in_file in in_files {
            if in_file.is_string() {
                result.push(InFile {
                    file: String::from(in_file.as_str().unwrap()),
                    credentials: None,
                });
            } else if in_file.is_mapping() {
                let in_file_mapping = in_file.as_mapping().unwrap();
                match in_file_mapping.get("file") {
                    Some(file_name) => {
                        let credentials;
                        match in_file_mapping.get("credentials") {
                            Some(credentials_name) => {
                                credentials =
                                    Some(String::from(credentials_name.as_str().unwrap()));
                            }
                            None => {
                                credentials = None;
                            }
                        }

                        result.push(InFile {
                            file: String::from(file_name.as_str().unwrap()),
                            credentials,
                        });
                    }
                    None => {
                        log::error!("You must define a file key for {:?}", value);
                    }
                }
            } else {
                log::error!("Unknown in_file type {:?}", in_file);
                process::exit(1);
            }
        }

        return result;
    }

    extract_value(value)
        .iter()
        .map(|file| InFile {
            file: file.to_owned(),
            credentials: None,
        })
        .collect()
}

fn extract_value(value: &Value) -> Vec<String> {
    if value.is_sequence() {
        return value
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    } else if value.is_mapping() {
        let mut mapping = String::from("{{");
        mapping.push_str(
            value
                .as_mapping()
                .unwrap()
                .keys()
                .next()
                .unwrap()
                .as_mapping()
                .unwrap()
                .keys()
                .next()
                .unwrap()
                .as_str()
                .unwrap(),
        );
        mapping.push_str("}}");
        return Vec::from([mapping]);
    }

    Vec::from([value.as_str().unwrap().to_string()])
}

pub fn extract_in_files(cwd: &str, entry: &Option<Value>) -> Vec<InFile> {
    let mut in_files: Vec<InFile> = Vec::new();
    if let Some(in_files_value) = entry {
        in_files = extract_in_file_value(in_files_value);
    }

    in_files = in_files
        .iter()
        .map(|in_file| {
            if graph::common::is_downloadable_file(&in_file.file) {
                return in_file.clone();
            }

            InFile {
                file: String::from(
                    get_absolute_file_path(&String::from(cwd), &in_file.file)
                        .to_string_lossy(),
                ),
                credentials: in_file.credentials.clone(),
            }
        })
        .collect::<Vec<InFile>>();

    in_files
}

pub fn extract_out_files(entry: &Option<Value>) -> Vec<String> {
    let mut out_files = Vec::new();
    if let Some(out_files_value) = entry {
        out_files = extract_value(out_files_value);
    }

    out_files
}

pub fn extract_then_targets(entry: &HashMap<String, Value>) -> Vec<String> {
    let mut then_targets = Vec::new();
    if let Some(then) = entry.get("then") {
        then_targets = extract_value(then);
    }

    then_targets
}

pub fn extract_clean(entry: &HashMap<String, Value>) -> Option<String> {
    let mut checksum = None;
    if let Some(checksum_command) = entry.get("clean") {
        let list = extract_value(&checksum_command);
        if list.len() > 0 {
            checksum = Some(list[0].clone());
        }
    }

    checksum
}

fn extract_type_and_args(action: &HashMap<String, Value>) -> Option<(&String, &Value)> {
    action
        .iter()
        .find(|(key, _)| !RESERVED_KEYWORDS.contains(&key.as_str()))
}

pub fn extract_checksum(entry: &HashMap<String, Value>) -> Option<String> {
    let mut checksum = None;
    if let Some(checksum_command) = entry.get("checksum") {
        let list = extract_value(&checksum_command);
        if list.len() > 0 {
            checksum = Some(list[0].clone());
        }
    }

    checksum
}

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

pub fn to_footprint_path(target_absolute_path: &str, cwd: &str) -> PathBuf {
    let footprints_dir = cache::get_footprints_dir_path(cwd);
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

// async fn create_target_arborescence2(
//     target_absolute_path: &String,
//     target_content: &Vec<HashMap<String, Value>>,
//     cwd: &Path,
// ) {
//     let target_tree_path = to_tree_path(target_absolute_path, &cache::get_tree_dir_path(&cwd.to_string_lossy().to_string()));
//     let target_emakefile_path = to_emakefile_path(target_absolute_path, cwd);
//     let mut target_properties = TargetProperties {
//         actions: Vec::new(),
//     };

//     tokio::fs::create_dir_all(&target_tree_path.parent().unwrap()).await.unwrap();

//     for action in target_content {
//         let plugin = extract_type_and_args(action);
//         let then_targets: Vec<String> = extract_then_targets(action)
//             .iter()
//             .map(|then_target| {
//                 get_absolute_target_path(
//                     then_target,
//                     &String::from(target_emakefile_path.to_str().unwrap()),
//                     cwd,
//                 )
//             })
//             .collect();
//         let in_files: Vec<InFile> =
//             extract_in_files(target_emakefile_path.parent().unwrap(), action);
//         let out_files: Vec<String> =
//             extract_out_files(target_emakefile_path.parent().unwrap(), action);
//         let checksum: Option<String> = extract_checksum(action);
//         let clean: Option<String> = extract_clean(action);

//         if plugin.is_none() {
//             panic!("No plugin specified for target {target_absolute_path}");
//         }

//         let action_properties = ActionProperties {
//             plugin: ActionPlugin {
//                 type_id: plugin.unwrap().0.to_owned(),
//                 args: plugin.unwrap().1.to_owned(),
//             },
//             in_files,
//             out_files,
//             checksum,
//             then_targets,
//             clean,
//         };

//         target_properties.actions.push(action_properties);
//     }

//     let serialized_properties = serde_yml::to_string(&target_properties).unwrap();
//     tokio::fs::write(target_tree_path, &serialized_properties)
//         .await
//         .unwrap();
// }

// pub async fn generate2(cwd: &Path) {
//     let glob_result = glob::glob(cwd.join("**").join("Emakefile").to_str().unwrap());
//     let mut visited_targets = Vec::new();
//     if let Ok(glob_paths) = glob_result {
//         for path_result in glob_paths {
//             if let Ok(current_emakefile_path) = path_result {
//                 let emakefile = emake::loader::load_file(current_emakefile_path.to_str().unwrap());

//                 for (target_name, target_content) in &emakefile.targets {
//                     let target_absolute_path = get_absolute_target_path(
//                         target_name,
//                         &String::from(current_emakefile_path.to_str().unwrap()),
//                         cwd,
//                     );

//                     if !visited_targets.contains(&target_absolute_path) {
//                         visited_targets.push(target_absolute_path.clone());
//                         create_target_arborescence2(&target_absolute_path, target_content, cwd).await;
//                     }
//                 }
//             }
//         }
//     }
// }
