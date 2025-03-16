use std::path::{Path, PathBuf};

use crate::{console::log, emake, graph, plugins};

pub async fn run(target: &String, silent: &bool, cwd: &Path) {
    let mut build_file = PathBuf::from(cwd);
    build_file.push("Emakefile");

    let emakefile: emake::Emakefile = emake::loader::load_file(build_file.to_str().unwrap());
    let graph_structure = graph::generator::generate(&emakefile, &target);

    // println!("{}", graph::viewer::as_graphviz(&graph_structure, &target_id));
    let plugins_store = plugins::instanciate();
    graph::runner::run_target(&target, graph_structure, plugins_store, silent, &String::from(cwd.to_str().unwrap())).await;
    log::success!("Everything is ok");
}