use std::{path::{Path, PathBuf}};

use crate::{console::log, emake, graph};

pub async fn run(target: &String, silent: &bool, cwd: &Path) {
    let mut build_file = PathBuf::from(cwd);
    build_file.push("Emakefile");

    let mut emakefile: emake::Emakefile = emake::loader::load_file(build_file.to_str().unwrap());
    let maybe_root_target = graph::analysor::find_root_target(cwd, &target);

    if let Some(root_target) = maybe_root_target {
        let graph_structure = graph::generator::generate(cwd, &mut emakefile, &root_target);
    
        graph::runner::run_target(
            &root_target,
            graph_structure,
            emakefile,
            silent,
            &String::from(cwd.to_str().unwrap())
        ).await;
    } else {
        log::error!("Can't find target {}", target);
    }
}