use crate::graph;

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