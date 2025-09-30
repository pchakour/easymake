use std::{collections::HashMap, path::Path};

use crate::{
    emake::{self},
    graph::generator::get_absolute_target_path,
};

pub fn get_clean_commands(cwd: &Path) -> HashMap<String, (String, String)> {
    let mut commands = HashMap::new();
    // let glob_result = glob::glob(cwd.join("**").join("Emakefile").to_str().unwrap());

    // if let Ok(glob_paths) = glob_result {
    //     for path_result in glob_paths {
    //         if let Ok(path) = path_result {
    //             let emakefile = emake::loader::load_file(path.to_str().unwrap());
    //             for (target_name, target) in &emakefile.targets {
    //                 let target_path = get_absolute_target_path(
    //                     target_name,
    //                     &emakefile.path.clone().unwrap(),
    //                     &cwd.to_string_lossy().to_string(),
    //                 );

    //                 if target.steps.is_some() {
    //                     for (step_index, step) in target.steps.clone().unwrap().iter().enumerate() {
    //                         if step.clean.is_some() {
    //                             commands.insert(
    //                                 format!("{}@{}", target_path, step_index),
    //                                 (emakefile.path.clone().unwrap(), step.clean.clone().unwrap()),
    //                             );
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    commands
}
