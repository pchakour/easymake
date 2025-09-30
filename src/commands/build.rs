use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex,
    },
    thread::{self, JoinHandle},
};

use crate::{
    cache,
    console::log,
    graph::{self, generator::get_absolute_target_path},
};
use crossbeam_channel::{bounded, Receiver};

lazy_static::lazy_static! {
    static ref LOGGER_HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);
}

fn ctrl_channel() -> Result<Receiver<()>, ctrlc::Error> {
    let (sender, receiver) = bounded(1);
    ctrlc::set_handler(move || {
        let _ = sender.send(());
    })?;
    Ok(receiver)
}

pub async fn run(target: &String, cwd: &Path, _find_root: bool) {
    let ctrl_c_events = ctrl_channel().unwrap();

    // Spawn ctrl+c handler in background thread
    let _ = thread::spawn(async move || {
        ctrl_c_events.recv().unwrap();
        exit(1);
    });

    // run the main async task
    main_task(target, cwd).await;

    exit(0);
}

async fn main_task(target: &String, cwd: &Path) {
    let mut build_file = PathBuf::from(cwd);
    build_file.push("Emakefile");

    let target_path = get_absolute_target_path(
        target,
        &build_file.to_str().unwrap().to_string(),
        &cwd.to_string_lossy().to_string(),
    );

    graph::runner::run_target(target_path, cwd.to_string_lossy().to_string()).await;
}

pub fn exit(code: i32) {
    if code == 0 {
        log::success!("Build successfully done");
    }

    cache::write_cache(&(code != 0));
    std::process::exit(code);
}
