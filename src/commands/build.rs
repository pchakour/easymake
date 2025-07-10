use std::{path::{Path, PathBuf}};

use crate::{console::log, emake, graph::{self, generator::get_absolute_target_path}};

pub async fn run(target: &String, silent: &bool, cwd: &Path, find_root: bool) {    
    // graph::generator::generate2(cwd).await;
    // log::info!("Tree generated");

    let mut build_file = PathBuf::from(cwd);
    build_file.push("Emakefile");

    let target_path = get_absolute_target_path(target, &build_file.to_str().unwrap().to_string(), &cwd.to_string_lossy().to_string());
    // let maybe_root_target;
    // if find_root {
    //     let find_root_result = graph::analysor::find_root_target(cwd, &target_path);
    //     if find_root_result.is_err() {
    //         log::error!("{}", find_root_result.err().unwrap());
    //         std::process::exit(1);
    //     } else {
    //         maybe_root_target = find_root_result.ok().unwrap();
    //     }
    // } else {
    //     maybe_root_target = Some(target_path);
    // }

    // println!("ROOT TARGET = {:?}", maybe_root_target);
    // if let Some(root_target) = &maybe_root_target {
    graph::runner::run_target3(
        target_path,
        String::from(cwd.to_str().unwrap()),
    ).await;
    // } else {
    //     log::error!("Can't find target {}", target);
    // }
}