mod emake;
mod graph;
mod plugins;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let target_id = &args[1];

    let cwd = String::from("/home/hawk/development/easybuild/examples/infrastructure");
    // TODO save in the cache the last modification of Emakefile, if a change happened then rebuild everything
    let build_file = "./examples/infrastructure/Emakefile";
    let emakefile: emake::Emakefile = emake::loader::load_file(build_file);
    println!("Build graph");
    let graph_structure = graph::generator::generate(&emakefile, &target_id);
    println!("{:?}", &graph_structure);

    println!("Target analysor");
    graph::analysor::target_analysor(&graph_structure, &target_id);
    println!("{}", graph::viewer::as_graphviz(&graph_structure, &target_id));
    let plugins_store = plugins::instanciate();
    graph::runner::run_target(&target_id, graph_structure, plugins_store, &cwd).await;
}