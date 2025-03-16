use std::fs;
use serde_yml;
use crate::emake;
use crate::console::log;

fn read_file_content(path: &str) -> String {
    log::info!("Loading file {:?}", path);
    let content = fs::read_to_string(path).unwrap();
    return content;
}


pub fn load_file(root: &str) -> emake::Emakefile {
    let build_file_content = read_file_content(root);
    let emakefile: emake::Emakefile = serde_yml::from_str(&build_file_content).unwrap();
    // println!("{:?}", emakefile);
    return emakefile;
}