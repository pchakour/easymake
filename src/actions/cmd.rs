use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap},
    future::Future,
    io::{BufRead, BufReader},
    pin::Pin,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use crate::{
    console::log,
    emake::{self, InFile, PluginAction},
};

use super::Action;
pub static ID: &str = "cmd";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdAction(pub String);

pub struct Cmd;

impl Action for Cmd {
    fn insert_in_files<'a>(
        &'a self,
        _action: &'a PluginAction,
        _in_files: &'a mut Vec<InFile>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {})
    }

    fn insert_out_files<'a>(
        &'a self,
        _action: &'a PluginAction,
        _out_files: &'a mut Vec<String>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {})
    }

    fn run<'a>(
        &'a self,
        cwd: &'a str,
        emakefile_cwd: &'a str,
        silent: bool,
        action: &'a PluginAction,
        in_files: &'a Vec<String>,
        out_files: &'a Vec<String>,
        _working_dir: &'a String,
        maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::Cmd { cmd } => {
                    // println!("Run command {:?}", args);
                    let mut command = cmd.0.clone();
                    let in_files_string = in_files.join(" ");
                    let out_files_string = out_files.join("");

                    let mut replacements: HashMap<String, String> = HashMap::from([
                        (String::from("in_files"), in_files_string),
                        (String::from("out_files"), out_files_string),
                    ]);

                    for (index, in_file) in in_files.iter().enumerate() {
                        let key = format!("in_files[{}]", index);
                        replacements.insert(key, in_file.clone());
                    }

                    for (index, in_file) in out_files.iter().enumerate() {
                        let key = format!("out_files[{}]", index);
                        replacements.insert(key, in_file.clone());
                    }

                    if let Some(default_replacements) = maybe_replacements {
                        replacements.extend(default_replacements.to_owned());
                    }

                    command = emake::compiler::compile(
                        cwd,
                        &command,
                        &emakefile_cwd.to_string(),
                        Some(&replacements),
                    );
                    log::text!("Run command {:?}", command);

                    let mut shell = "sh";
                    let mut arg = "-c";

                    if cfg!(target_os = "windows") {
                        shell = "cmd";
                        arg = "/C";
                    }

                    let mut output = Command::new(shell)
                        .current_dir(cwd)
                        .arg(arg) // Pass the command string to the shell
                        .arg(command)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                        .expect("Failed to execute command");

                    let stdout = output.stdout.take().unwrap();
                    let stderr = output.stderr.take().unwrap();

                    let stdout_reader = BufReader::new(stdout);
                    let stderr_reader = BufReader::new(stderr);

                    // Spawn threads to read both stdout and stderr
                    let stdout_thread: std::thread::JoinHandle<()> =
                        std::thread::spawn(move || {
                            for line in stdout_reader.lines() {
                                if let Ok(text) = line {
                                    if false {
                                        // if !silent {
                                        log::text!("{}{}", log::INDENT, text);
                                    }
                                }
                            }
                        });

                    stdout_thread.join().unwrap();

                    let stderr_buffer = Arc::new(Mutex::new(String::new())); // Mutex allows safe mutation
                    let stderr_buffer_clone = Arc::clone(&stderr_buffer);

                    let stderr_thread = std::thread::spawn(move || {
                        for line in stderr_reader.lines() {
                            if let Ok(text) = line {
                                if silent {
                                    let mut buffer = stderr_buffer_clone.lock().unwrap(); // Lock before modifying
                                    buffer.push_str(&text);
                                    buffer.push('\n');
                                } else {
                                    log::error!("{}{}", log::INDENT, text);
                                }
                            }
                        }
                    });

                    stderr_thread.join().unwrap();

                    let status = output.wait().expect("Failed to wait on child");

                    if !status.success() {
                        log::error!("Command return an error status {}", status.code().unwrap());
                        log::error!("{}", stderr_buffer.lock().unwrap());
                        return true;
                    }

                    return false;
                }
                _ => false,
            }
        })
    }

    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
}
