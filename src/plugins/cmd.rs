use serde_yml::Value;
use std::{
    collections::HashMap, io::{BufRead, BufReader}, process::{Command, Stdio}, sync::{Arc, Mutex}
};

use crate::{console::log, emake};

use super::Plugin;
pub static ID: &str = "cmd";

pub struct Cmd;

impl Plugin for Cmd {
    fn action(
        &self,
        cwd: &str,
        silent: bool,
        args: &Value,
        in_files: &Vec<String>,
        out_files: &Vec<String>,
        _working_dir: &String,
        maybe_replacements: Option<&HashMap<&str, &str>>,
    ) -> () {
        // println!("Run command {:?}", args);
        let mut command = args.as_str().unwrap_or("").to_string();
        let in_files_string = in_files.join(" ");
        let out_files_string = out_files.join("");

        let replacements = HashMap::from([
            ("in_files", in_files_string.as_str()),
            ("out_files", out_files_string.as_str()),
        ]);

        command = emake::compiler::compile(cwd, &command, Some(&replacements));
        command = emake::compiler::compile(cwd, &command, maybe_replacements);

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
        let stdout_thread: std::thread::JoinHandle<()> = std::thread::spawn(move || {
            for line in stdout_reader.lines() {
                if let Ok(text) = line {
                    if !silent {
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
            log::error!("{}", stderr_buffer.lock().unwrap());
        }
    }

    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync> {
        Box::new(Self)
    }
}
