mod actions;
mod cache;
mod commands;
mod console;
mod doc;
mod emake;
mod errors;
mod graph;
mod secrets;
mod utils;

use clap::{arg, Command, Parser};
use indicatif::MultiProgress;
use std::{env, fs, path::{Path, PathBuf}, sync::{Arc, OnceLock, RwLock}};

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
use tokio::sync::Mutex;

use crate::{actions::ActionsStore, secrets::SecretsStore};

pub static GLOBAL_MUTEXES: Lazy<DashMap<String, Arc<Mutex<()>>>> = Lazy::new(DashMap::new);
pub static ACTIONS_STORE: Lazy<ActionsStore> = Lazy::new(|| actions::instanciate());
pub static CREDENTIALS_STORE: Lazy<SecretsStore> = Lazy::new(|| secrets::instanciate());
pub static CACHE_IN_FILE_TO_UPDATE: Lazy<DashSet<(String, String)>> = Lazy::new(DashSet::new);
pub static CACHE_OUT_FILE_TO_UPDATE: Lazy<DashSet<(String, String)>> = Lazy::new(DashSet::new);
pub static MULTI_PROGRESS: Lazy<Arc<MultiProgress>> = Lazy::new(|| Arc::new(MultiProgress::new()));
pub static CWD: OnceLock<RwLock<PathBuf>> = OnceLock::new();

fn init_cwd(cwd: PathBuf) {
    CWD.set(RwLock::new(cwd)).ok().unwrap();
}

/// Get current CWD (thread-safe)
pub fn get_cwd() -> PathBuf {
    CWD.get().unwrap().read().unwrap().clone()
}


pub async fn get_mutex_for_id(id: &str) -> Arc<Mutex<()>> {
    GLOBAL_MUTEXES
        .entry(id.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let matches = Command::new("MyApp")
        .version("0.0.1")
        .about("The tool to build everything")
        .arg(arg!(--cwd <PATH>).required(false))
        .arg(
            arg!(--log_level <LOG_LEVEL> "Log level")
                .required(false)
                .value_parser(["console", "info", "debug", "trace"])
                .default_value("console"),
        )
        .subcommand(Command::new("clean").about("Clean cache"))
        .subcommand(
            Command::new("graph")
                .about("Generate graphviz graph")
                .arg(arg!([target] "Target to analyze").required(true)),
        )
        .subcommand(
            Command::new("build")
                .about("Build a target")
                .arg(arg!([target] "Target to build").required(true)),
        )
        .subcommand(Command::new("doc").about("Generate documentation"))
        .subcommand(
            Command::new("keyring")
                .about("Manage password with local secrets manager")
                .subcommand(
                    Command::new("store")
                        .about("Store a secret")
                        .arg(arg!([service] "Service name").required(true))
                        .arg(arg!([name] "Secret name").required(true)),
                )
                .subcommand(
                    Command::new("clear")
                        .about("Remove a secret")
                        .arg(arg!([service] "Service name").required(true))
                        .arg(arg!([name] "Secret name").required(true)),
                ),
        )
        .get_matches();

    let mut cwd = env::current_dir().unwrap();

    if let Some(custom_cwd) = matches.get_one::<String>("cwd") {
        cwd = Path::new(&fs::canonicalize(&custom_cwd).unwrap().to_str().unwrap()).to_path_buf();
    }

    init_cwd(cwd.clone());

    cache::create_cache_dir().await;
    commands::run_command(&matches, &cwd).await;
}
