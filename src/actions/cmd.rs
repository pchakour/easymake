use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    future::Future,
    io::{BufRead, BufReader, Read},
    pin::Pin,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use crate::{
    console::{
        log,
        logger::{ActionProgressType, LogAction, Logger, ProgressStatus},
    },
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
        target_id: &'a str,
        step_id: &'a str,
        emakefile_cwd: &'a str,
        _silent: bool,
        action: &'a PluginAction,
        in_files: &'a Vec<String>,
        out_files: &'a Vec<String>,
        _working_dir: &'a String,
        maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            if let PluginAction::Cmd { cmd } = action {
                let mut command = cmd.0.clone();
                let in_files_string = in_files.join(" ");
                let out_files_string = out_files.join(" ");

                let mut replacements = HashMap::from([
                    ("in_files".into(), in_files_string),
                    ("out_files".into(), out_files_string),
                ]);

                for (i, f) in in_files.iter().enumerate() {
                    replacements.insert(format!("in_files[{}]", i), f.clone());
                }
                for (i, f) in out_files.iter().enumerate() {
                    replacements.insert(format!("out_files[{}]", i), f.clone());
                }

                if let Some(defaults) = maybe_replacements {
                    replacements.extend(defaults.clone());
                }

                command = emake::compiler::compile(
                    cwd,
                    &command,
                    &emakefile_cwd.to_string(),
                    Some(&replacements),
                );

                let action_id = format!(
                    "{}{}{}{}{}",
                    target_id,
                    step_id,
                    ID,
                    in_files.join(";"),
                    out_files.join(";")
                );

                Logger::set_action(
                    target_id.to_string(),
                    step_id.to_string(),
                    LogAction {
                        id: action_id.clone(),
                        status: ProgressStatus::Progress,
                        description: format!("Running command: {}", command),
                        progress: ActionProgressType::Spinner,
                        percent: None,
                    },
                );

                let (shell, arg) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };

                let mut child = Command::new(shell)
                    .current_dir(cwd)
                    .arg(arg)
                    .arg(&command)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("Failed to execute command");

                let stdout = child.stdout.take().unwrap();
                let stderr = child.stderr.take().unwrap();

                let stdout_reader = BufReader::new(stdout);
                let stderr_reader = BufReader::new(stderr);

                let tid_stdout = target_id.to_string();
                let sid_stdout = step_id.to_string();
                let aid_stdout = action_id.clone();
                let cmd_stdout = command.clone();

                let stdout_thread = std::thread::spawn(move || {
                    for line in stdout_reader.lines() {
                        if let Ok(text) = line {
                            Logger::set_action(
                                tid_stdout.clone(),
                                sid_stdout.clone(),
                                LogAction {
                                    id: aid_stdout.clone(),
                                    status: ProgressStatus::Progress,
                                    description: format!("{}\n[stdout] {}", cmd_stdout, text),
                                    progress: ActionProgressType::Spinner,
                                    percent: None,
                                },
                            );
                        }
                    }
                });

                let tid_stderr = target_id.to_string();
                let sid_stderr = step_id.to_string();
                let aid_stderr = action_id.clone();
                let cmd_stderr = command.clone();

                let stderr_thread = std::thread::spawn(move || {
                    let mut errors = Vec::new();
                    for line in stderr_reader.lines() {
                        if let Ok(text) = line {
                            errors.push(text.clone());
                            Logger::set_action(
                                tid_stderr.clone(),
                                sid_stderr.clone(),
                                LogAction {
                                    id: aid_stderr.clone(),
                                    status: ProgressStatus::Progress,
                                    description: format!("{}\n[stderr] {}", cmd_stderr, text),
                                    progress: ActionProgressType::Spinner,
                                    percent: None,
                                },
                            );
                        }
                    }
                    errors
                });

                let status = child.wait().expect("Failed to wait on child");

                stdout_thread.join().unwrap();
                let errors = stderr_thread.join().unwrap();

                if !status.success() {
                    Logger::set_action(
                        target_id.to_string(),
                        step_id.to_string(),
                        LogAction {
                            id: action_id.clone(),
                            status: ProgressStatus::Failed,
                            description: format!(
                                "Command `{}` failed with exit code {}.\n{}",
                                command,
                                status.code().unwrap_or(-1),
                                errors.join("\n")
                            ),
                            progress: ActionProgressType::Spinner,
                            percent: None,
                        },
                    );
                    return true;
                }

                Logger::set_action(
                    target_id.to_string(),
                    step_id.to_string(),
                    LogAction {
                        id: action_id,
                        status: ProgressStatus::Done,
                        description: format!("Command `{}` completed successfully", command),
                        progress: ActionProgressType::Spinner,
                        percent: None,
                    },
                );

                false
            } else {
                false
            }
        })
    }

    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
}
