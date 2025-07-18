use std::path::Path;
use clap::ArgMatches;

pub mod graph;
pub mod clean;
pub mod build;

pub async fn run_command(matches: &ArgMatches, cwd: &Path) {
    if let Some(matches) = matches.subcommand_matches("build") {
        let target = matches.get_one::<String>("target").expect("required");
        let notsilent = matches.get_one::<bool>("notsilent").unwrap_or(&false);
        build::run(target, &!notsilent, cwd, true).await;
    } else if let Some(_matches) = matches.subcommand_matches("clean") {
        clean::run(cwd).await;
    } else if let Some(matches) = matches.subcommand_matches("graph") {
        let target = matches.get_one::<String>("target").expect("required");
        graph::run(target, cwd);
    }
}
