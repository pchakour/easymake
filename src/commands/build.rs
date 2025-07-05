use std::{path::{Path, PathBuf}};

use crate::{console::log, emake, graph::{self, generator::get_absolute_target_path}};

pub async fn run(target: &String, silent: &bool, cwd: &Path, find_root: bool) {
    let mut build_file = PathBuf::from(cwd);
    build_file.push("Emakefile");

    let mut emakefile: emake::Emakefile = emake::loader::load_file(build_file.to_str().unwrap());
    let target_path = get_absolute_target_path(target, &build_file.to_str().unwrap().to_string(), cwd);
    let maybe_root_target;
    if find_root {
        maybe_root_target = graph::analysor::find_root_target(cwd, &target_path);
    } else {
        maybe_root_target = Some(target_path);
    }

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