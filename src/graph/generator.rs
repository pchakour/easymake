use hex::encode;
use serde_yml::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::process;

use crate::console::log;
use crate::emake;
use crate::graph;
use crate::graph::InFile;

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

fn create_action_node(plugin_id: &String, args: &Value, cwd: String, in_files: &Vec<InFile>, out_files: &Vec<String>, checksum: &Option<String>) -> graph::Node {
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
        let in_files = value
            .as_sequence()
            .unwrap();

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
                                credentials = Some(String::from(credentials_name.as_str().unwrap()));
                            },
                            None => {
                                credentials = None;
                            }
                        }

                        result.push(InFile {
                            file: String::from(file_name.as_str().unwrap()),
                            credentials,
                        });
                    },
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
    
    extract_value(value).iter().map(|file| InFile {
        file: file.to_owned(),
        credentials: None,
    }).collect()
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


fn extract_in_files(cwd: &Path, entry: &HashMap<String, Value>) -> Vec<InFile> {
    let mut in_files: Vec<InFile> = Vec::new();
    if let Some(in_files_value) = entry.get("in_files") {
        in_files = extract_in_file_value(in_files_value);
    }

    in_files = in_files.iter().map(|in_file| {
        if graph::common::is_downloadable_file(&in_file.file) {
            return in_file.clone();
        }
        
        InFile {
            file: String::from(get_absolute_file_path(&String::from(cwd.to_string_lossy()), &in_file.file).to_string_lossy()),
            credentials: in_file.credentials.clone(),
        }
    }).collect::<Vec<InFile>>();

    in_files
}

fn extract_out_files(_cwd: &Path, entry: &HashMap<String, Value>) -> Vec<String> {
    let mut out_files = Vec::new();
    if let Some(out_files_value) = entry.get("out_files") {
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

fn create_target_node(
    cwd: &Path,
    emakefile: &mut emake::Emakefile,
    target_name: &String,
    graph: &mut graph::Graph,
    visited: &mut HashSet<String>,
) -> graph::Node {
    println!("Generate target target_name={} cwd={} emakefile_path={}", target_name, cwd.to_str().unwrap(), emakefile.path.clone().unwrap());
    
    let emakefile_target = emake::loader::get_target(cwd, target_name, emakefile);
    let target_path = get_absolute_target_path(target_name, &emakefile.path.as_ref().unwrap().to_string(), cwd);
    println!("Computed target path {}", target_path);
    let mut out_neighbors: Vec<String> = Vec::new();
    let in_neighbors: Vec<String> = Vec::new(); 

    for entry in &emakefile_target {
        let mut maybe_subtarget: Option<String> = None;
        let then_targets: Vec<String> = extract_then_targets(entry);
        let in_files: Vec<InFile> = extract_in_files(cwd, entry);
        let out_files: Vec<String> = extract_out_files(cwd, entry);
        let checksum: Option<String> = extract_checksum(entry);
        
        for then_target in &then_targets {
            if !visited.contains(then_target) {
                let mut n = create_target_node(
                    cwd,
                    emakefile,
                    then_target,
                    graph,
                    visited,
                );
                visited.insert(n.id.clone());
                n.in_files = in_files.clone();
                n.out_files = out_files.clone();
                graph.nodes.insert(n.id.clone(), n);
            }
        }

        for (key, value) in entry {
            if key != "clean" && key != "checksum" && key != "then" && key != "in_files" && key != "out_files" {
                let plugin_id = key;
                let args = value;
                let mut n = create_action_node(
                    &plugin_id, 
                    args, 
                    emakefile.path.as_ref().unwrap().to_string(), 
                    &in_files, 
                    &out_files,
                    &checksum,
                );
                n.in_neighbors.push(target_path.clone());
                n.in_files = in_files.clone();
                n.out_files = out_files.clone();
                let id = n.id.clone();
                graph.nodes.insert(id.clone(), n);
                maybe_subtarget = Some(id.clone());
            }
        }

        if let Some(action_id) = maybe_subtarget {
            if let Some(action) = graph.nodes.get_mut(&action_id) {
                for then_target in &then_targets {
                    let then_target_path = get_absolute_target_path(then_target, &emakefile.path.as_ref().unwrap().to_string(), cwd);
                    action.out_neighbors.push(then_target_path);
                }

                for then_target in &then_targets {
                    let then_target_path = get_absolute_target_path(then_target, &emakefile.path.as_ref().unwrap().to_string(), cwd);
                    let maybe_then_target_node = graph
                        .nodes
                        .get_mut(&then_target_path);
                    
                    if let Some(then_target_node) = maybe_then_target_node {
                        then_target_node
                            .in_neighbors
                            .push(action_id.clone());
                    }
                }
            }

            out_neighbors.push(action_id);
        }
    }

    graph::Node {
        id: target_path.clone(),
        out_neighbors,
        in_neighbors,
        action: None,
        in_files: Vec::new(),
        out_files: Vec::new(),
        cwd: emakefile.path.as_ref().unwrap().clone(),
    }
}

pub fn get_absolute_target_path(path: &String, emakefile_current_path: &String, cwd: &Path) -> String {
    let path_separator = String::from("/");
    if path.starts_with("//") {
        let mut path_parts: Vec<&str> = path.split(&path_separator).collect();
        let mut target_key = path_parts.pop().unwrap();
        let mut target_key_parts: Vec<&str> = target_key.split(':').collect();
        target_key = target_key_parts.pop().unwrap();

        path_parts.join(&path_separator) + "/targets:" + target_key
    } else {
        let mut path_parts: Vec<&str> = path.split(&path_separator).filter(|part| !part.is_empty()).collect();
        let target_key = path_parts.pop().unwrap();
        let mut target_key_parts: Vec<&str> = target_key.split(':').collect();
        let target_key = target_key_parts.pop().unwrap();
        let parent_target_path = (path_separator + Path::new(emakefile_current_path).parent().unwrap().to_str().unwrap()).replace(cwd.to_str().unwrap(), "");
        let target_path = format!("{}/{}targets:{}", &parent_target_path, path_parts.join("/"), target_key);
        target_path
    }
}

pub fn generate(cwd: &Path, emakefile: &mut emake::Emakefile, target_id: &String) -> graph::Graph {
    let mut graph = graph::Graph {
        nodes: HashMap::new(),
        root: String::from(""),
    };
    let mut visited = HashSet::new();
    let root = create_target_node(cwd, emakefile, target_id, &mut graph, &mut visited);
    let target_path = get_absolute_target_path(target_id, &emakefile.path.as_ref().unwrap(), cwd);
    graph.root = target_path.clone();
    graph.nodes.insert(target_path, root);

    return graph;
}