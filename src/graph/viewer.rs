use std::collections::HashSet;

use crate::{
    emake::{self, loader::extract_info_from_path, Target},
    graph::{
        self,
        generator::{get_absolute_target_path, to_emakefile_path},
    },
};

fn escape_quotes(input: &str) -> String {
    let mut escaped = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '"' {
            // Check if the quote is already escaped
            if escaped.ends_with('\\') {
                // It's already escaped, keep it as is
                escaped.push(c);
            } else {
                // It's not escaped, add a backslash
                escaped.push('\\');
                escaped.push(c);
            }
        } else if c == '\\' {
            // Push the backslash and check the next character
            escaped.push(c);
            if let Some(&next) = chars.peek() {
                if next == '"' {
                    // Already escaped quote, keep it as is
                    escaped.push(chars.next().unwrap());
                }
            }
        } else {
            escaped.push(c);
        }
    }

    escaped
}

fn as_graphviz_target(
    graph: &graph::Graph,
    target_id: &String,
    visited: &mut HashSet<String>,
) -> String {
    let mut graphviz = String::new();
    let maybe_target_node = graph.nodes.get(target_id);

    if let Some(target_node) = maybe_target_node {
        if !visited.contains(target_id) {
            visited.insert(target_id.clone());

            match &target_node.action {
                None => (),
                Some(action) => {
                    graphviz.push_str("\n");
                    let action_str = format!("{:?}", action);
                    graphviz.push_str(
                        &format!(
                            "\"{}\"[shape=rectangle,style=filled,label=\"{}\"]",
                            &target_id,
                            escape_quotes(&action_str)
                        )
                        .to_string(),
                    );
                }
            }

            for neighbor_id in &target_node.out_neighbors {
                graphviz.push_str("\n");
                graphviz.push_str(&format!("\"{}\" -> \"{}\"", target_id, neighbor_id));
                let neighbor_graphviz = as_graphviz_target(graph, neighbor_id, visited);
                graphviz.push_str(&neighbor_graphviz);
            }
        } else {
            println!("Target {} duplicated !!", target_id);
        }
    }

    return graphviz;
}

pub fn as_graphviz_bak(graph: &graph::Graph, target_id: &String) -> String {
    let mut visited: HashSet<String> = HashSet::new();
    let mut graphviz = as_graphviz_target(graph, target_id, &mut visited);

    graphviz = format!("digraph G {{{}\n}}", &graphviz);
    return graphviz;
}

fn target_visitor<F>(
    parent_target: &str,
    target_absolute_path: &str,
    cwd: &str,
    visitor: &mut F,
)
where
    F: FnMut(&str, &str, &Target),
{
    let emakefile_path = to_emakefile_path(target_absolute_path, cwd);
    let emakefile = emake::loader::load_file(&emakefile_path.to_string_lossy().to_string());
    let target_info = extract_info_from_path(
        target_absolute_path,
        cwd,
        &emakefile_path.to_string_lossy().to_string(),
    );

    if let Some(target) = emakefile.targets.get(&target_info.target_name) {
        visitor(parent_target, target_absolute_path, target);

        if let Some(deps) = &target.deps {
            for dep in deps {
                let dep_target_absolute_path =
                    get_absolute_target_path(dep, &emakefile_path.to_string_lossy().to_string(), cwd);
                target_visitor(target_absolute_path, &dep_target_absolute_path, cwd, visitor);
            }
        }
    }
}

pub fn as_graphviz(target_absolute_path: &str, cwd: &str) -> String {
    let mut edges = Vec::new();

    
    // pass a mutable closure reference into the traversal
    let mut collect_edges = |parent: &str, current: &str, target: &Target| {
        let mut previous = String::from(current);
        if parent != "//" {
            edges.push(format!("    \"{}\" -> \"{}\";", parent, current));
        }

        for (index, step) in target.steps.as_ref().unwrap().iter().enumerate() {
            let step_id = format!("{current}.steps[{index}]");
            let mut step_name = &step.description;
            if step.description == "" {
                step_name = &step_id;
            }

            let step_node = format!("\"{step_id}\"[shape=\"parallelogram\", label=\"{step_name}\"]");
            edges.push(step_node);

            if target.parallel.unwrap_or(false) {
                edges.push(format!("    \"{}\" -> \"{}\";", current, step_id));
            } else {
                edges.push(format!("    \"{}\" -> \"{}\";", previous, step_id));
                previous = step_id;
            }
        }
        
    };

    target_visitor("//", target_absolute_path, cwd, &mut collect_edges);
    

    let body = edges.join("\n");
    format!("digraph G {{\n{}\n}}", body)
}
