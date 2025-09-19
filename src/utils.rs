use crate::emake;
use std::{
    collections::HashMap,
    io::Read,
    path::Path,
    process::{Command, Stdio},
};

pub fn run_command(
    command: &String,
    cwd: &Path,
    emakefile_path: &Path,
    replacements: Option<&HashMap<String, String>>,
) -> (
    std::process::ExitStatus,
    String,
    String,
) {
    let compiled_command = emake::compiler::compile(
        cwd.to_str().unwrap(),
        command,
        &String::from(emakefile_path.to_str().unwrap()),
        replacements,
    );

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

    // let stdout_reader: BufReader<std::process::ChildStdout> = BufReader::new(stdout);
    // let stderr_reader: BufReader<std::process::ChildStderr> = BufReader::new(stderr);

    // Spawn threads to read both stdout and stderr
    // let stdout_thread: std::thread::JoinHandle<()> = std::thread::spawn(move || {
    //     for line in stdout_reader.lines() {
    //         if let Ok(text) = line {
    //             log::text!("{}{}", log::INDENT, text);
    //         }
    //     }
    // });

    // stdout_thread.join().unwrap();

    // let stderr_buffer: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new())); // Mutex allows safe mutation
    // let stderr_buffer_clone = Arc::clone(&stderr_buffer);

    // let stderr_thread = std::thread::spawn(move || {
    //     for line in stderr_reader.lines() {
    //         if let Ok(text) = line {
    //             log::warning!("{}{}", log::INDENT, text);
    //         }
    //     }
    // });

    // stderr_thread.join().unwrap();

    let status: std::process::ExitStatus = output.wait().expect("Failed to wait on child");

    // if !status.success() {
    //     log::warning!("{}", stderr_buffer.lock().unwrap());
    // }

    let mut stderr_output = String::new();
    if let Some(mut stderr) = output.stderr.take() {
        stderr.read_to_string(&mut stderr_output).expect("Failed to read stderr");
    }
    let mut stdout_output = String::new();
    if let Some(mut stdout) = output.stdout.take() {
        stdout.read_to_string(&mut stdout_output).expect("Failed to read stdout");
    }

    (status, stdout_output, stderr_output)
}

pub fn get_absolute_file_path(cwd: &str, file: &str) -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(&file);
    if !path.is_absolute() {
        path = std::path::PathBuf::from(cwd);
        path.push(file);
    }
    path
}