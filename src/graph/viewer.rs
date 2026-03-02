use std::collections::HashSet;

use crate::{
    emake::{self, loader::extract_info_from_path, Target},
    graph::{
        generator::{get_absolute_target_path, to_emakefile_path},
    },
};

fn target_visitor<F>(
    parent_target: &str,
    target_absolute_path: &str,
    visitor: &mut F,
)
where
    F: FnMut(&str, &str, &Target),
{
    let emakefile_path = to_emakefile_path(target_absolute_path);
    let emakefile = emake::loader::load_file(&emakefile_path.to_string_lossy().to_string());
    let target_info = extract_info_from_path(
        target_absolute_path,
        &emakefile_path.to_string_lossy().to_string(),
    );

    if let Some(target) = emakefile.targets.get(&target_info.unwrap().target_name) {
        visitor(parent_target, target_absolute_path, target);

        if let Some(deps) = &target.deps {
            for dep in deps {
                let dep_target_absolute_path =
                    get_absolute_target_path(dep, &emakefile_path.to_string_lossy().to_string());
                target_visitor(target_absolute_path, &dep_target_absolute_path, visitor);
            }
        }
    }
}

pub fn as_graphviz(target_absolute_path: &str) -> String {
    let mut edges: HashSet<String> = HashSet::new();

    
    // pass a mutable closure reference into the traversal
    let mut collect_edges = |parent: &str, current: &str, target: &Target| {
        let mut previous = String::from(current);
        if parent != "//" {
            edges.insert(format!("    \"{}\" -> \"{}\";", parent, current));
        }

        if target.steps.is_none() {
            return;
        }

        for (index, step) in target.steps.as_ref().unwrap().iter().enumerate() {
            let step_id = format!("{current}.steps[{index}]");
            let mut step_name = &step.description;
            if step.description == "" {
                step_name = &step_id;
            }

            let step_node = format!("\"{step_id}\"[shape=\"parallelogram\", label=\"{step_name}\"]");
            edges.insert(step_node);

            if target.parallel.unwrap_or(false) {
                edges.insert(format!("    \"{}\" -> \"{}\";", current, step_id));
            } else {
                edges.insert(format!("    \"{}\" -> \"{}\";", previous, step_id));
                previous = step_id;
            }
        }
        
    };

    target_visitor("//", target_absolute_path, &mut collect_edges);
    

    let list_edges: Vec<String> = edges.into_iter().collect();
    let body = list_edges.join("\n");
    
    format!("digraph G {{\n{}\n}}", body)
}
