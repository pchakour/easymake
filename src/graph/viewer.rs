use std::collections::HashSet;

use crate::graph;

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

pub fn as_graphviz(graph: &graph::Graph, target_id: &String) -> String {
    let mut visited: HashSet<String> = HashSet::new();
    let mut graphviz = as_graphviz_target(graph, target_id, &mut visited);

    graphviz = format!("digraph G {{{}\n}}", &graphviz);
    return graphviz;
}
