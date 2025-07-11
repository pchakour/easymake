mod actions;
mod cache;
mod commands;
mod console;
mod credentials;
mod emake;
mod graph;
mod utils;
mod errors;

use clap::{arg, Command, Parser};
use indicatif::MultiProgress;
use std::{env, fs, path::Path, sync::Arc};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(long)]
    cwd: Option<String>,
    command: String,
    target: Option<String>,
}

use dashmap::{DashMap, DashSet};
use once_cell::sync::Lazy;
use tokio::{
    sync::{Mutex, Semaphore},
};

use crate::{actions::ActionsStore, credentials::CredentialsStore};

pub static GLOBAL_MUTEXES: Lazy<DashMap<String, Arc<Mutex<()>>>> = Lazy::new(DashMap::new);
pub static GLOBAL_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(15));
pub static ACTIONS_STORE: Lazy<ActionsStore> = Lazy::new(|| actions::instanciate());
pub static CREDENTIALS_STORE: Lazy<CredentialsStore> = Lazy::new(|| credentials::instanciate());
pub static CACHE_TO_UPDATE: Lazy<DashSet<(String, String)>> = Lazy::new(DashSet::new);
pub static MULTI_PROGRESS: Lazy<Arc<MultiProgress>> = Lazy::new(|| Arc::new(MultiProgress::new()));

pub async fn get_mutex_for_id(id: &str) -> Arc<Mutex<()>> {
    GLOBAL_MUTEXES.entry(id.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}


#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let matches = Command::new("MyApp")
        .version("0.0.1")
        .about("The tool to build everything")
        .arg(arg!(--cwd <PATH>).required(false))
        .subcommand(Command::new("clean").about("Clean cache"))
        .subcommand(
            Command::new("graph")
                .about("Generate graphviz graph")
                .arg(arg!([target] "Target to analyze").required(true)),
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

    cache::create_cache_dir(cwd.to_str().unwrap()).await;
    commands::run_command(&matches, &cwd).await;
    // cache::write_cache(cwd.to_str().unwrap()).await;
}
