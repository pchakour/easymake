use std::path::{Path, PathBuf};

use crate::{emake, graph};

pub fn run(target: &String, cwd: &Path) {
    let mut build_file = PathBuf::from(cwd);
    build_file.push("Emakefile");

    let mut emakefile: emake::Emakefile = emake::loader::load_file(build_file.to_str().unwrap());
    let graph_structure = graph::generator::generate(cwd, &mut emakefile, &target);

    println!("{}", graph::viewer::as_graphviz(&graph_structure, &target));
}