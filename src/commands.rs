use clap::ArgMatches;

use crate::console::log::{set_log_level, LogLevel};

pub mod build;
pub mod clean;
pub mod graph;
pub mod doc;
pub mod keyring;

pub async fn run_command(matches: &ArgMatches) {
    let log_level = matches.get_one::<String>("log_level").unwrap();
    set_log_level(LogLevel::from_str(log_level));

    if let Some(matches) = matches.subcommand_matches("build") {
        let target = matches.get_one::<String>("target").expect("required");
        build::run(target, true).await;
    } else if let Some(_matches) = matches.subcommand_matches("clean") {
        clean::run().await;
    } else if let Some(matches) = matches.subcommand_matches("graph") {
        let target = matches.get_one::<String>("target").expect("required");
        graph::run(target);
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
