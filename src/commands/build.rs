use std::path::{Path, PathBuf};

use crate::{console::log, emake, graph, actions, credentials};

pub async fn run(target: &String, silent: &bool, cwd: &Path) {
    let mut build_file = PathBuf::from(cwd);
    build_file.push("Emakefile");

    let mut emakefile: emake::Emakefile = emake::loader::load_file(build_file.to_str().unwrap());
    let graph_structure = graph::generator::generate(cwd, &mut emakefile, &target);

    let actions_store = actions::instanciate();
    let credentials_store = credentials::instanciate();

    graph::runner::run_target(
        &target,
        graph_structure,
        emakefile,
        actions_store,
        credentials_store,
        silent,
        &String::from(cwd.to_str().unwrap())
    ).await;
    log::success!("Everything is ok");
}