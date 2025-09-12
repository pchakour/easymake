use clap::ArgMatches;
use std::path::Path;

pub mod build;
pub mod clean;
pub mod graph;
pub mod doc;
pub mod keyring;

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
    } else if let Some(_matches) = matches.subcommand_matches("doc") {
        doc::generate();
    } else if let Some(matches) = matches.subcommand_matches("keyring") {
        if let Some(submatches) = matches.subcommand_matches("store") {
            let service = submatches.get_one::<String>("service").expect("required");
            let name = submatches.get_one::<String>("name").expect("required");
            keyring::store(service, name);
        } else if let Some(submatches) = matches.subcommand_matches("clear") {
            let service = submatches.get_one::<String>("service").expect("required");
            let name = submatches.get_one::<String>("name").expect("required");
            keyring::clear(service, name);
        }
    }
}
