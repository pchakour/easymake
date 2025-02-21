use hex::encode;
use serde_yml::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;

use crate::emake;
use crate::graph;

fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    encode(result)
}


fn create_action_node(plugin_id: &String, args: &Value) -> graph::Node {
    let args_as_str = format!("{:?}", args);
    graph::Node {
        id: format!("{}|{}", plugin_id, compute_sha256(&args_as_str)),
        action: Some(graph::Action { plugin_id: plugin_id.clone(), args: args.clone() }),
        out_neighbors: Vec::new(),
        in_neighbors: Vec::new(),
        in_files: Vec::new(),
        out_files: Vec::new(),
    }
}

fn get_node_leaves(graph: &graph::Graph, node_id: &String) -> HashSet<String> {
    let mut leaves = HashSet::new();
    let maybe_node = graph.nodes.get(node_id);

    if let Some(node) = maybe_node {
        if node.out_neighbors.is_empty() {
            leaves.insert(node_id.clone());
        } else {
            for neighbor_id in &node.out_neighbors {
                let neighbor_leaves = get_node_leaves(graph, neighbor_id);
                leaves.extend(neighbor_leaves);
            }
        }
    }

    return leaves;
}

fn create_target_node(
    emakefile: &emake::Emakefile,
    target_id: &String,
    graph: &mut graph::Graph,
    visited: &mut HashSet<String>,
) -> graph::Node {
    visited.insert(target_id.clone());
    let maybe_target_configuration = emakefile.targets.get(target_id);
    let mut out_neighbors: Vec<String> = Vec::new();
    let in_neighbors: Vec<String> = Vec::new();

    if let Some(target_configuration) = maybe_target_configuration {
        for entry in target_configuration {
            let mut then_targets: Vec<String> = Vec::new();
            let mut maybe_subtarget: Option<String> = None;
            let mut is_subtarget_action = false;
            let mut in_files: Vec<String> = Vec::new();
            let mut out_files: Vec<String> = Vec::new();

            if let Some(in_files_value) = entry.get("in_files") {
                if in_files_value.is_sequence() {
                    in_files = in_files_value
                        .as_sequence()
                        .unwrap()
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                } else if in_files_value.is_mapping() {
                    let mut mapping = String::from("{{");
                    mapping.push_str(in_files_value.as_mapping().unwrap().keys().next().unwrap().as_mapping().unwrap().keys().next().unwrap().as_str().unwrap());
                    mapping.push_str("}}");
                    in_files = Vec::from([mapping]);
                } else {
                    in_files = Vec::from([in_files_value.as_str().unwrap().to_string()]);
                }
            }

            if let Some(out_files_value) = entry.get("out_files") {
                if out_files_value.is_sequence() {
                    out_files = out_files_value
                        .as_sequence()
                        .unwrap()
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                } else if out_files_value.is_mapping() {
                    let mut mapping = String::from("{{");
                    mapping.push_str(out_files_value.as_mapping().unwrap().keys().next().unwrap().as_mapping().unwrap().keys().next().unwrap().as_str().unwrap());
                    mapping.push_str("}}");
                    out_files = Vec::from([mapping]);
                } else {
                    out_files = Vec::from([out_files_value.as_str().unwrap().to_string()]);
                }
            }

            if let Some(then_value) = entry.get("then") {
                let l:  Vec<String>;
                if then_value.is_sequence() {
                    l = then_value
                        .as_sequence()
                        .unwrap()
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                } else if then_value.is_mapping() {
                    let mut mapping = String::from("{{");
                    mapping.push_str(then_value.as_mapping().unwrap().keys().next().unwrap().as_mapping().unwrap().keys().next().unwrap().as_str().unwrap());
                    mapping.push_str("}}");
                    l = Vec::from([mapping]);
                } else {
                    l = Vec::from([then_value.as_str().unwrap().to_string()]);
                }
        
                for s in l {
                    if !visited.contains(&s) {
                        let mut n = create_target_node(emakefile, &s, graph, &mut visited.clone());
                        n.in_files = in_files.clone();
                        n.out_files = out_files.clone();
                        graph.nodes.insert(n.id.clone(), n);
                        then_targets.push(s);
                    }
                }
            }

            if let Some(target_value) = entry.get("target") {
                let subtarget_id = target_value.as_str().unwrap().to_string();

                if !visited.contains(&subtarget_id) {
                    visited.insert(subtarget_id.clone());
                    let mut n = create_target_node(emakefile, &subtarget_id, graph, &mut visited.clone());
                    n.in_neighbors.push(target_id.clone());
                    n.in_files = in_files.clone();
                    n.out_files = out_files.clone();
                    graph.nodes.insert(n.id.clone(), n);
                    maybe_subtarget = Some(subtarget_id);
                }
            }

            for (key, value) in entry {
                if key != "then" && key != "target" && key != "in_files" && key != "out_files" {
                    is_subtarget_action = true;
                    let plugin_id = key;
                    let args = value;
                    let mut n = create_action_node(&plugin_id, args);
                    n.in_neighbors.push(target_id.clone());
                    n.in_files = in_files.clone();
                    n.out_files = out_files.clone();
                    let id = n.id.clone();
                    graph.nodes.insert(id.clone(), n);
                    maybe_subtarget = Some(id.clone());
                }
            }

            if let Some(subtarget_id) = maybe_subtarget {
                if is_subtarget_action {
                    if let Some(subtarget) = graph.nodes.get_mut(&subtarget_id) {
                        for then_target in then_targets {
                            subtarget.out_neighbors.push(then_target.clone());
                        }
                    }
                } else {
                    let leaves = get_node_leaves(graph, &subtarget_id);
                    for leaf_id in &leaves {
                        let maybe_node = graph.nodes.get_mut(leaf_id);
                        if let Some(node) = maybe_node {
                            for then_target in &then_targets {
                                node.out_neighbors.push(then_target.clone());
                            }
                            
                            for then_target in &then_targets {
                                graph.nodes.get_mut(then_target).unwrap().in_neighbors.push(leaf_id.clone());
                            }
                        }

                    }
                }

                out_neighbors.push(subtarget_id);
            }
        }
    }

    graph::Node {
        id: target_id.clone(),
        out_neighbors,
        in_neighbors,
        action: None,
        in_files: Vec::new(),
        out_files: Vec::new()
    }
}


pub fn generate(emakefile: &emake::Emakefile, target_id: &String) -> graph::Graph {
    let mut graph = graph::Graph { nodes: HashMap::new(), root: String::from("") };
    let mut visited = HashSet::new();
    let root = create_target_node(emakefile, target_id, &mut graph, &mut visited);
    graph.root = target_id.clone();
    graph.nodes.insert(target_id.clone(), root);

    return graph;
}
 
