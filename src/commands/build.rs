use std::{path::{Path, PathBuf}, sync::Mutex, thread, time::Duration};

use crossterm::{cursor::SavePosition, execute};

use crate::{cache, console::{log, logger::Logger}, emake, graph::{self, generator::get_absolute_target_path}};

use std::sync::atomic::{AtomicBool, Ordering};

static LOGGER_RUNNING: AtomicBool = AtomicBool::new(true);

pub async fn run(target: &String, silent: &bool, cwd: &Path, find_root: bool) {
    let mut build_file = PathBuf::from(cwd);
    build_file.push("Emakefile");

    let target_path = get_absolute_target_path(
        target,
        &build_file.to_str().unwrap().to_string(),
        &cwd.to_string_lossy().to_string(),
    );

    let mut stdout = std::io::stdout();
    execute!(stdout, SavePosition).unwrap();

    // Start the logger thread
    LOGGER_RUNNING.store(true, Ordering::SeqCst);
    let handle = thread::spawn(|| {
        while LOGGER_RUNNING.load(Ordering::SeqCst) {
            Logger::write();
            thread::sleep(Duration::from_millis(150));
        }
        Logger::close();
    });

    // Run the actual task
    graph::runner::run_target(target_path, cwd.to_string_lossy().to_string()).await;
    cache::write_out_cache(cwd.to_str().unwrap()).await;

    // Stop the logger thread
    LOGGER_RUNNING.store(false, Ordering::SeqCst);
    handle.join().unwrap();
}