use std::{
    path::{Path, PathBuf}, sync::atomic::{AtomicUsize, Ordering}, thread
};

use crate::{
    cache,
    console::{log, progress_bar::{self, set_loader_message}},
    graph::{self, generator::get_absolute_target_path},
};
use crossbeam_channel::{bounded, Receiver};

static RUNNING_STEPS: AtomicUsize = AtomicUsize::new(0);
static DONE_STEPS: AtomicUsize = AtomicUsize::new(0);

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
        log::info!("Receive CTRL-C signal from user");
        exit(1);
    });

    // run the main async task
    main_task(target, cwd).await;

    exit(0);
}

pub fn update_progress(increment_running: bool, increment_done: bool) {
    let mut running_steps = RUNNING_STEPS.load(Ordering::Relaxed);
    let mut done_steps = DONE_STEPS.load(Ordering::Relaxed);

    if increment_running {
        RUNNING_STEPS.fetch_add(1, Ordering::Relaxed);
        running_steps += 1;
    } else {
        RUNNING_STEPS.fetch_sub(1, Ordering::Relaxed);
        running_steps -= 1;
    }

    if increment_done {
        DONE_STEPS.fetch_add(1, Ordering::Relaxed);
        done_steps += 1;
    }

    let running_step_text = format!("{} running {}", running_steps, if running_steps > 1 { "steps" } else { "step" });
    let done_step_text = format!("{} finished {}", done_steps, if done_steps > 1 { "steps" } else { "step" });


    set_loader_message(&format!("Building [{} | {}]", running_step_text, done_step_text));
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
    progress_bar::finish();
    std::process::exit(code);
}
