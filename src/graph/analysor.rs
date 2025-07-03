use std::{collections::HashSet, path::Path};

use crate::{emake::{self, loader::get_target_name}, graph::{self, generator::{extract_then_targets, get_absolute_target_path}}};

fn count_steps(graph: &graph::Graph, node: &graph::Node, current_steps_size: usize) -> usize {
    let mut steps_size = current_steps_size;
    for neighbor in &node.out_neighbors {
        let neighbor_node = graph.nodes.get(neighbor).unwrap();
        if let Some(_) = &neighbor_node.action {
            steps_size += 1;
        }
        steps_size = count_steps(graph, neighbor_node, steps_size);
    }

    steps_size
}

pub fn steps_len(graph: &graph::Graph) -> usize {
    let root_node = graph.nodes.get(&graph.root).unwrap();
    count_steps(graph, root_node, 0)
}


pub fn find_root_target(cwd: &Path, target_absolute_path: &String) -> Option<String> {
    let glob_result = glob::glob(cwd.join("**").join("Emakefile").to_str().unwrap());
    let mut has_parent = false;

    if let Ok(glob_paths) = glob_result {
        for path_result in glob_paths {
            if let Ok(path) = path_result  {
                let emakefile = emake::loader::load_file(path.to_str().unwrap());
                for (current_target_name, actions) in emakefile.targets {
                    for action in actions {
                        let then_targets = extract_then_targets(&action);
                        for then_target_name in then_targets {
                            let then_target_path = get_absolute_target_path(&then_target_name, &String::from(path.to_str().unwrap()), cwd);
                            if then_target_path == *target_absolute_path {
                                has_parent = true;
                                let current_target_path = get_absolute_target_path(&current_target_name, &String::from(path.to_str().unwrap()), cwd);
                                return find_root_target(cwd, &current_target_path);
                            }
                        }
                    }
                }
            }
        }
    }

    if !has_parent {
        return Some(target_absolute_path.clone());
    }

    None
}