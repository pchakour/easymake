use std::{
    collections::{HashMap},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use crate::{
    console::log,
    emake,
    cache,
    graph,
};

const CACHE_DIR: &str = ".emake";

pub async fn run(cwd: &Path) {
    let clean_commands = graph::analysor::get_clean_commands(cwd);
    let mut step = 1;
    // log::step!(step, total_steps, "Cleaning cache");
    let mut path = PathBuf::from(cwd);
    path.push(CACHE_DIR);
    let _ = std::fs::remove_dir_all(path);

    let working_dir = cache::get_working_dir_path();
    let out_dir = cache::get_out_dir_path();
    let default_replacements = HashMap::from([
        (String::from("EMAKE_WORKING_DIR"), working_dir.to_owned()),
        (String::from("EMAKE_OUT_DIR"), out_dir.to_owned()),
        (
            String::from("EMAKE_CWD_DIR"),
            String::from(cwd.to_str().unwrap()),
        ),
    ]);

    for (_, (emakefile_path, command)) in &clean_commands {
        let compiled_command = emake::compiler::compile(
            cwd.to_str().unwrap(),
            command,
            emakefile_path,
            Some(&default_replacements),
        );
        step = step + 1;
        // log::step!(
        //     step,
        //     total_steps,
        //     "Running clean command if action {}",
        //     action_id
        // );

        let mut shell = "sh";
        let mut arg = "-c";

        if cfg!(target_os = "windows") {
            shell = "cmd";
            arg = "/C";
        }

        let mut output = Command::new(shell)
            .current_dir(cwd)
            .arg(arg) // Pass the command string to the shell
            .arg(compiled_command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to execute command");

        let stdout = output.stdout.take().unwrap();
        let stderr = output.stderr.take().unwrap();

        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        // Spawn threads to read both stdout and stderr
        let stdout_thread: std::thread::JoinHandle<()> = std::thread::spawn(move || {
            for line in stdout_reader.lines() {
                if let Ok(text) = line {
                    log::text!("{}{}", log::INDENT, text);
                }
            }
        });

        stdout_thread.join().unwrap();

        let stderr_buffer: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new())); // Mutex allows safe mutation
        // let stderr_buffer_clone = Arc::clone(&stderr_buffer);

        let stderr_thread = std::thread::spawn(move || {
            for line in stderr_reader.lines() {
                if let Ok(text) = line {
                    log::warning!("{}{}", log::INDENT, text);
                }
            }
        });

        stderr_thread.join().unwrap();

        let status = output.wait().expect("Failed to wait on child");

        if !status.success() {
            log::warning!("{}", stderr_buffer.lock().unwrap());
        }
    }
}
