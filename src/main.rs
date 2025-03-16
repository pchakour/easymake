mod emake;
mod graph;
mod plugins;
mod console;
mod commands;

use std::{env, fs, path::Path};
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

#[tokio::main]
async fn main() {
    let matches = Command::new("MyApp")
        .version("0.0.1")
        .about("Tool to build everut")
        .arg(arg!(--cwd <PATH>).required(false))
        .subcommand(Command::new("clean").about("Clean cache"))
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