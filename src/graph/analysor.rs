use crate::graph;

fn find_cycle(graph: &graph::Graph, target_id: &String, visited: &mut Vec<String>) {
    if visited.contains(target_id) {
        panic!("Find a cycle with target id {:?}!!", visited);
    }

    visited.push(target_id.clone());
    if let Some(target_node) = graph.nodes.get(target_id) {
        for neighbor in &target_node.out_neighbors {
            find_cycle(graph, neighbor, visited);
        }
    }

}

pub fn target_analysor(graph: &graph::Graph, target_id: &String) {
    let mut visited = Vec::new();
    // TODO check no cycle in the graph for this target
    find_cycle(graph, target_id, &mut visited);
    // TODO get all step to have a progression when run the target

    // A --> B --> C
    //   --> C

    // A --> B --> C --> B 
}