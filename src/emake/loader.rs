use std::collections::HashMap;
use std::fs;
use serde_yml;
use crate::emake;
use crate::console::log;
use std::path::Path;

fn read_file_content(path: &str) -> String {
    log::info!("Loading file {:?}", path);
    let content = fs::read_to_string(path).unwrap();
    return content;
}


pub fn get_target(cwd: &Path, target: &String, emakefile: &mut emake::Emakefile) -> Vec<HashMap<std::string::String, serde_yml::Value>> {
    // Check if the target exists in the current Emakefile
    let target_split: Vec<&str> = target.split('/').collect();
    let mut target_emakefile_path = Vec::new();
    let from_root = target.starts_with("//");
    
    for path_part in target_split {
        target_emakefile_path.push(path_part);
    }

    let maybe_real_target: Option<&str> = target_emakefile_path.pop();

    if let Some(real_target) = maybe_real_target {
        if from_root {
            target_emakefile_path.remove(0);
            target_emakefile_path.remove(0);
            let mut path = cwd.join(target_emakefile_path.join("/"));
            path = path.join("Emakefile");
            *emakefile = emake::loader::load_file(path.to_str().unwrap());
            if emakefile.targets.contains_key(real_target) {
                return emakefile.targets.get(real_target).unwrap().clone();
            }
        } else if emakefile.path.as_ref().unwrap().ends_with(&target_emakefile_path.join("/").to_string()) && emakefile.targets.contains_key(real_target) {
            return emakefile.targets.get(real_target).unwrap().clone();
        } else {
            if target_emakefile_path.len() > 0 {
                target_emakefile_path.remove(0);
            }
            let current_emakefile_path = emakefile.path.as_ref().unwrap().clone();
            let mut path = Path::new(&current_emakefile_path).parent().unwrap().join(target_emakefile_path.join("/"));
            path = path.join("Emakefile");
            *emakefile = emake::loader::load_file(path.to_str().unwrap());
            if emakefile.targets.contains_key(real_target) {
                return emakefile.targets.get(real_target).unwrap().clone();
            }
        }
    }
        
    log::panic!("No target {} found", target);
}

pub fn load_file(root: &str) -> emake::Emakefile {
    let build_file_content = read_file_content(root);
    let mut emakefile: emake::Emakefile = serde_yml::from_str(&build_file_content).unwrap();
    emakefile.path = Some(String::from(root));
    // println!("{:?}", emakefile);
    return emakefile;
}