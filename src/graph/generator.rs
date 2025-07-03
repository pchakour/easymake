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
use crate::ACTIONS_STORE;

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

fn create_action_node(plugin_id: &String, args: &Value, cwd: String, in_files: &Vec<InFile>, out_files: &Vec<String>) -> graph::Node {
    let args_as_str = format!("{:?} {:?} {:?}", in_files, out_files, args);
    graph::Node {
        id: format!("{}|{}", plugin_id, compute_sha256(&args_as_str)),
        action: Some(graph::Action {
            plugin_id: plugin_id.clone(),
            args: args.clone(),
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

fn create_target_node(
    cwd: &Path,
    emakefile: &mut emake::Emakefile,
    target_name: &String,
    graph: &mut graph::Graph,
    visited: &mut HashSet<String>,
) -> graph::Node {
    println!("Generate target target_name={} cwd={} emakefile_path={}", target_name, cwd.to_str().unwrap(), emakefile.path.clone().unwrap());
    
    let emakefile_target = emake::loader::get_target(cwd, target_name, emakefile);
    let mut out_neighbors: Vec<String> = Vec::new();
    let in_neighbors: Vec<String> = Vec::new(); 

    for entry in &emakefile_target {
        let mut maybe_subtarget: Option<String> = None;
        let then_targets: Vec<String> = extract_then_targets(entry);
        let in_files: Vec<InFile> = extract_in_files(cwd, entry);
        let out_files: Vec<String> = extract_out_files(cwd, entry);
        
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
            if key != "then" && key != "in_files" && key != "out_files" {
                let plugin_id = key;
                let args = value;
                let mut n = create_action_node(&plugin_id, args, emakefile.path.as_ref().unwrap().to_string(), &in_files, &out_files);
                n.in_neighbors.push(target_name.clone());
                n.in_files = in_files.clone();
                n.out_files = out_files.clone();
                let id = n.id.clone();
                graph.nodes.insert(id.clone(), n);
                maybe_subtarget = Some(id.clone());
            }
        }

        if let Some(subtarget_id) = maybe_subtarget {
            if let Some(subtarget) = graph.nodes.get_mut(&subtarget_id) {
                for then_target in &then_targets {
                    subtarget.out_neighbors.push(then_target.clone());
                }

                for then_target in &then_targets {
                    let maybe_then_target_node = graph
                        .nodes
                        .get_mut(then_target);
                    
                    if let Some(then_target_node) = maybe_then_target_node {
                        then_target_node
                            .in_neighbors
                            .push(subtarget_id.clone());
                    } else {
                        log::panic!("Loop detected on target {}", then_target);
                    }
                }
            }

            out_neighbors.push(subtarget_id);
        }
    }

    graph::Node {
        id: target_name.clone(),
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
        let mut path_parts: Vec<&str> = path.split('/').collect();
        let mut target_key = path_parts.pop().unwrap();
        let mut target_key_parts: Vec<&str> = target_key.split(':').collect();
        target_key = target_key_parts.pop().unwrap();

        path_parts.join(&path_separator) + "/targets:" + target_key
    } else {
        let mut path_parts: Vec<&str> = path.split('/').collect();
        let target_key = path_parts.pop().unwrap();
        let mut target_key_parts: Vec<&str> = target_key.split(':').collect();
        let target_key = target_key_parts.pop().unwrap();
        (path_separator + Path::new(emakefile_current_path).parent().unwrap().join(path_parts.join("/")).join(String::from("targets:") + target_key).to_str().unwrap()).replace(cwd.to_str().unwrap(), "")
    }
}

pub fn create_target_node2(
    cwd: &Path,
    emakefile: &mut emake::Emakefile,
    target_key: &String,
    graph: &mut graph::Graph,
    visited: &mut HashSet<String>,
) -> graph::Node {
    println!("Generate target target_key={} cwd={} emakefile_path={}", target_key, cwd.to_str().unwrap(), emakefile.path.clone().unwrap());
    
    let target_path = get_absolute_target_path(target_key, &emakefile.path.clone().unwrap(), &cwd);
    let emakefile_target = emake::loader::get_target2(cwd, &target_path, emakefile);
    println!("TARGET path {}", target_path);
    let mut out_neighbors: Vec<String> = Vec::new();
    let in_neighbors: Vec<String> = Vec::new(); 

    for entry in &emakefile_target {
        let mut maybe_subtarget: Option<String> = None;
        let then_targets: Vec<String> = extract_then_targets(entry);
        let in_files: Vec<InFile> = extract_in_files(cwd, entry);
        let out_files: Vec<String> = extract_out_files(cwd, entry);
        
        for then_target in &then_targets {
            let then_target_path = get_absolute_target_path(then_target, &emakefile.path.clone().unwrap(), &cwd);
            println!("Then target {}", then_target_path);
            if !visited.contains(&then_target_path) {
                let mut n = create_target_node2(
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
            if key != "then" && key != "in_files" && key != "out_files" {
                let action_type = key;
                let args = value;
                let action = ACTIONS_STORE.get(action_type).unwrap();
                let maybe_n = action.generate(cwd, args, emakefile, graph, visited);
                let mut n;

                if let Some(action_node) = maybe_n {
                    println!("NODE {:?}", action_node);
                    n = action_node;
                } else {
                    n = create_action_node(&action_type, args, emakefile.path.as_ref().unwrap().to_string(), &in_files, &out_files);
                    n.in_neighbors.push(target_path.clone());
                    n.in_files = in_files.clone();
                    n.out_files = out_files.clone();
                }

                maybe_subtarget = Some(n.id.clone());
                graph.nodes.insert(n.id.clone(), n);

            }
        }

        if let Some(subtarget_id) = maybe_subtarget {
            if let Some(subtarget) = graph.nodes.get_mut(&subtarget_id) {
                for then_target in &then_targets {
                    subtarget.out_neighbors.push(then_target.clone());
                }

                for then_target in &then_targets {
                    let maybe_then_target_node = graph
                        .nodes
                        .get_mut(then_target);
                    
                    if let Some(then_target_node) = maybe_then_target_node {
                        then_target_node
                            .in_neighbors
                            .push(subtarget_id.clone());
                    }
                    // else {
                    //     log::panic!("Loop detected on target {}", then_target);
                    // }
                }
            }

            out_neighbors.push(subtarget_id);
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

fn find_root(node_id: &String, graph: &graph::Graph, roots: &mut Vec<String>) {
    println!("Find root of node {}", node_id);
    println!("GRAPH {:?}", graph);
    let node = graph.nodes.get(node_id).unwrap();
    if node.in_neighbors.len() == 0 {
        roots.push(node_id.to_owned());
    } else {
        for current_node_id in &node.in_neighbors {
            find_root(current_node_id, graph, roots);
        }
    }
}

pub fn generate2(cwd: &Path, emakefile: &mut emake::Emakefile, user_target_name: &String) -> graph::Graph {
    let mut graph = graph::Graph {
        nodes: HashMap::new(),
        root: String::from(""),
    };
    let mut visited = HashSet::new();

    println!("AAAAAAAAAAAAAA user_target_name {}", user_target_name);
    // let user_target_node = create_target_node2(cwd, &mut emakefile.clone(), user_target_name, &mut graph, &mut visited);
    for (target_name, _) in &emakefile.targets {
        let mut workspace_emakefile = emakefile.clone();
        let target_node = create_target_node2(cwd, &mut workspace_emakefile, target_name, &mut graph, &mut visited);
        graph.nodes.insert(target_node.id.clone(), target_node);
    }

    // Find all actions that point to user_target_id
    // let mut roots = Vec::new();
    // find_root(&user_target_node.id, &graph, &mut roots);

    println!("Graph {:?}", graph);

    graph
}

pub fn generate(cwd: &Path, emakefile: &mut emake::Emakefile, target_id: &String) -> graph::Graph {
    let mut graph = graph::Graph {
        nodes: HashMap::new(),
        root: String::from(""),
    };
    let mut visited = HashSet::new();
    let root = create_target_node(cwd, emakefile, target_id, &mut graph, &mut visited);
    graph.root = target_id.clone();
    graph.nodes.insert(target_id.clone(), root);

    return graph;
}