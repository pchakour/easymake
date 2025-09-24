use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    cache,
    console::log,
    graph::{self, generator::get_absolute_target_path},
};
use crossbeam_channel::{bounded, Receiver};
use crossterm::{cursor::SavePosition, execute};

static LOGGER_RUNNING: AtomicBool = AtomicBool::new(true);

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

    let cwd_string = cwd.to_string_lossy().to_string();
    // Spawn ctrl+c handler in background thread
    let _ = thread::spawn(async move || {
        ctrl_c_events.recv().unwrap();
        exit(&cwd_string, 1).await;
    });

    // run the main async task
    main_task(target, cwd).await;

    exit(cwd.to_str().unwrap(), 0).await;
}

async fn main_task(target: &String, cwd: &Path) {
    let mut build_file = PathBuf::from(cwd);
    build_file.push("Emakefile");

    let target_path = get_absolute_target_path(
        target,
        &build_file.to_str().unwrap().to_string(),
        &cwd.to_string_lossy().to_string(),
    );

    // let mut stdout = std::io::stdout();
    // execute!(stdout, SavePosition).unwrap();

    // Logger::init();

    // LOGGER_RUNNING.store(true, Ordering::SeqCst);
    // let logger_thread: JoinHandle<()> = thread::spawn(|| {
    //     while LOGGER_RUNNING.load(Ordering::SeqCst) {
    //         Logger::write();
    //         thread::sleep(Duration::from_millis(150));
    //     }
    // });

    // *LOGGER_HANDLE.lock().unwrap() = Some(logger_thread);

    graph::runner::run_target(target_path, cwd.to_string_lossy().to_string()).await;
}

pub async fn exit(cwd: &str, code: i32) {
    // LOGGER_RUNNING.store(false, Ordering::SeqCst);

    // if let Some(handle) = LOGGER_HANDLE.lock().unwrap().take() {
    //     handle.join().unwrap();
    // }

    // Logger::close();

    // if code == 0 { // TODO
    log::info!("Build done");
    // }

    force_exit(cwd, code).await;
}

pub async fn force_exit(cwd: &str, code: i32) {
    cache::write_cache(cwd).await;
    std::process::exit(code);
}
