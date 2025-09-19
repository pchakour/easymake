use std::path::{Path, PathBuf};

use crate::{graph::{self, generator::get_absolute_target_path}};

pub fn run(target: &String, cwd: &Path) {
    let mut build_file = PathBuf::from(cwd);
    build_file.push("Emakefile");

    let target_path = get_absolute_target_path(
        target,
        &build_file.to_str().unwrap().to_string(),
        &cwd.to_string_lossy().to_string(),
    );

    
    // let target_path = get_absolute_target_path(target, &build_file.to_str().unwrap().to_string(), cwd);
    // let maybe_root_target = graph::analysor::find_root_target(cwd, &target_path);

    // if let Some(root_target) = maybe_root_target {
    println!("{}", graph::viewer::as_graphviz(&target_path, &cwd.to_str().unwrap()));
    // }
}