mod emake;
mod graph;
mod actions;
mod console;
mod commands;
mod credentials;
mod utils;

use std::{env, fs, path::Path, sync::Arc};
use clap::{arg, Command, Parser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(long)]
    cwd: Option<String>,
    command: String,
    target: Option<String>,
}

use dashmap::DashMap;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

use crate::{actions::ActionsStore, credentials::CredentialsStore};

pub static GLOBAL_MUTEXES: Lazy<DashMap<String, Arc<Mutex<()>>>> = Lazy::new(DashMap::new);
pub static ACTIONS_STORE: Lazy<ActionsStore> = Lazy::new(|| actions::instanciate());
pub static CREDENTIALS_STORE: Lazy<CredentialsStore> = Lazy::new(|| credentials::instanciate());

pub async fn get_mutex_for_id(id: &str) -> Arc<Mutex<()>> {
    use dashmap::mapref::entry::Entry;

    match GLOBAL_MUTEXES.entry(id.to_string()) {
        Entry::Occupied(entry) => entry.get().clone(),
        Entry::Vacant(entry) => entry.insert(Arc::new(Mutex::new(()))).clone(),
    }
}

#[tokio::main]
async fn main() {
    let matches = Command::new("MyApp")
        .version("0.0.1")
        .about("The tool to build everything")
        .arg(arg!(--cwd <PATH>).required(false))
        .subcommand(Command::new("clean").about("Clean cache"))
        .subcommand(
            Command::new("graph")
                .about("Generate graphviz graph")
                .arg(arg!([target] "Target to analyze").required(true))
            )
        .subcommand(
            Command::new("build")
                .about("Build a target")
                .arg(arg!([target] "Target to build").required(true))
                .arg(arg!(--notsilent "Dispay all outputs").required(false)),
        )
        .get_matches();

    let mut cwd = env::current_dir().unwrap();

    if let Some(custom_cwd) = matches.get_one::<String>("cwd") {
        cwd = Path::new(&fs::canonicalize(&custom_cwd).unwrap().to_str().unwrap()).to_path_buf();
    }

    commands::run_command(&matches, &cwd).await;
}