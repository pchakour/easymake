use std::path::{Path, PathBuf};

use graphviz_rust::{
    cmd::{CommandArg, Format},
    dot_structures::Graph,
    exec, parse,
    printer::PrinterContext,
};

use crate::{
    console::log, get_cwd, graph::{self, generator::get_absolute_target_path}
};

pub fn run(target: &String, path: &String) {
    let build_file = get_cwd().join("Emakefile");
    let target_path = get_absolute_target_path(target, &build_file.to_str().unwrap().to_string());
    let graphviz = graph::viewer::as_graphviz(&target_path);
    let g: Graph = parse(&graphviz).unwrap();
    let mut graphviz_path = PathBuf::from(path);
    if !graphviz_path.is_file() {
        graphviz_path = graphviz_path.join("graphviz.png");
    }

    let generation_result = exec(
        g,
        &mut PrinterContext::default(),
        vec![Format::Png.into(), CommandArg::Output(graphviz_path.to_string_lossy().to_string())],
    );
    
    if generation_result.is_err() {
        log::warning!("Something's wrong when generating the graphviz image, Is graphviz installed ?");
        log::warning!("Error: {}", generation_result.err().unwrap().to_string());
        log::warning!("\nIn meantime, we provide you the graphviz string :-)");
        log::info!("{}", graphviz);
    }
}
