use std::{fs};

use crate::{console::log, get_cwd};

pub fn initialize() {
  let cwd = get_cwd();
  let root_emakefile = cwd.join("Emakefile");
  if fs::exists(&root_emakefile).unwrap() {
    log::panic!("The project folder already contains a root Emakefile");
  }

  let mut emakefile_content = String::from("targets:\n");
  emakefile_content.push_str("  hello_world:\n");
  emakefile_content.push_str("    steps:\n");
  emakefile_content.push_str("    - description: My First Target\n");
  emakefile_content.push_str("      shell:\n");
  emakefile_content.push_str("        cmd: echo 'Hello world !'\n");

  fs::write(root_emakefile, emakefile_content).unwrap();
}