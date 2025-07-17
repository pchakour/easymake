use std::{
    io::{stdout, Write},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    },
    thread::sleep,
    time::Duration,
};

use console::{style, truncate_str};
use crossterm::{
    cursor::{MoveDown, MoveToColumn, MoveUp, RestorePosition, SavePosition}, execute, queue, style::Print, terminal::{Clear, ClearType}
};
use dashmap::DashMap;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use terminal_size::{terminal_size, Width};
use textwrap::wrap;

use crate::{actions::mv::Move, console::log};

type TargetId = String;
type StepId = String;
type ActionId = String;
type StepDescription = Option<String>;
type TargetDescription = Option<String>;

#[derive(Debug, PartialEq, Clone)]
pub enum ProgressStatus {
    Done,
    Progress,
    Failed,
    Skipped,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ActionProgressType {
    Spinner,
    Bar,
}

#[derive(Debug, Clone)]
pub struct LogAction {
    pub id: String,
    pub description: String,
    pub status: ProgressStatus,
    pub progress: ActionProgressType,
    pub percent: Option<usize>,
}

pub struct LogStep {
    pub id: String,
    pub description: String,
    pub actions: Vec<LogAction>,
    pub status: ProgressStatus,
}

pub struct LogTarget {
    pub id: String,
    pub description: Option<String>,
    pub steps: Vec<LogStep>,
    pub status: ProgressStatus,
}

static GLOBAL_OUTPUT: Lazy<Mutex<Vec<LogTarget>>> = Lazy::new(|| Mutex::new(Vec::new()));
static GLOBAL_ITERATIONS: Lazy<DashMap<String, usize>> = Lazy::new(DashMap::new);
static PRINTED_LINES: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));
const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
static BUFFER_OUTPUT: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub struct Logger;

fn get_terminal_width() -> usize {
    terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80)
}

fn log_line(text: &str, buffer: &mut Vec<String>) {
    let width = get_terminal_width();
    let truncated_line = truncate_str(text, width, "...").to_string();
    buffer.push(truncated_line);
}

fn flush_output(stdout: &mut std::io::Stdout, buffer: &Vec<String>) {
    let mut previous_buffer = BUFFER_OUTPUT.lock().unwrap();

    // Move up to the top to start drawing
    if !previous_buffer.is_empty() {
        queue!(stdout, MoveUp(previous_buffer.len() as u16)).unwrap();
    }

    for (index, line) in buffer.iter().enumerate() {
        queue!(stdout, MoveToColumn(0)).unwrap();

        if previous_buffer.len() > index {
            if previous_buffer[index] != *line {
                queue!(stdout, Clear(ClearType::CurrentLine)).unwrap();
                queue!(stdout, Print(format!("{}\n", line))).unwrap();
                previous_buffer[index] = line.to_owned();
            } else {
                // Skip writing unchanged line but still move to next line
                queue!(stdout, MoveDown(1)).unwrap();
            }
        } else {
            // New line
            queue!(stdout, Print(format!("{}\n", line))).unwrap();
            previous_buffer.push(line.to_owned());
        }
    }

    // Trim lines if new buffer is shorter
    if previous_buffer.len() > buffer.len() {
        for _ in buffer.len()..previous_buffer.len() {
            queue!(stdout, MoveToColumn(0)).unwrap();
            queue!(stdout, Clear(ClearType::CurrentLine)).unwrap();
            queue!(stdout, MoveDown(1)).unwrap();
        }
        previous_buffer.truncate(buffer.len());
    }

    stdout.flush().unwrap();
}

impl Logger {
    pub fn set_target(log_target: LogTarget) {
        let mut output = GLOBAL_OUTPUT.lock().unwrap();

        match output.iter_mut().find(|target| target.id == log_target.id) {
            Some(target) => {
                target.description = log_target.description;
                target.status = log_target.status;
            }
            None => {
                output.push(log_target);
            }
        }
    }

    pub fn set_step(target_id: TargetId, log_step: LogStep) {
        match GLOBAL_OUTPUT
            .lock()
            .unwrap()
            .iter_mut()
            .find(|item| item.id == target_id)
        {
            Some(target) => {
                match target.steps.iter_mut().find(|step| step.id == log_step.id) {
                    Some(step) => {
                        // Copy step
                        step.description = log_step.description;
                        step.status = log_step.status;
                    }
                    None => target.steps.push(log_step),
                };
            }
            None => {
                log::error!(
                    "Can't find target id {} to add step logging {}",
                    target_id,
                    log_step.id
                );
                std::process::exit(1);
            }
        };
    }

    pub fn set_action(target_id: TargetId, step_id: StepId, log_action: LogAction) {
        match GLOBAL_OUTPUT
            .lock()
            .unwrap()
            .iter_mut()
            .find(|item| item.id == target_id)
        {
            Some(target) => {
                match target.steps.iter_mut().find(|item| item.id == step_id) {
                    Some(step) => {
                        if let Some(action) = step
                            .actions
                            .iter_mut()
                            .find(|item| item.id == log_action.id)
                        {
                            action.id = log_action.id;
                            action.description = log_action.description;
                            action.percent = log_action.percent;
                            action.progress = log_action.progress;
                            action.status = log_action.status;
                        } else {
                            GLOBAL_ITERATIONS
                                .insert(target_id + step_id.as_str() + log_action.id.as_str(), 0);
                            step.actions.push(log_action);
                        }
                    }
                    None => {
                        log::error!("Can't find step id {}", step_id);
                        std::process::exit(1);
                    }
                };
            }
            None => {
                log::error!("Can't find target id {}", target_id);
                std::process::exit(1);
            }
        };
    }

    pub fn write() {
        let mut stdout = std::io::stdout();
        let mut lines_number = PRINTED_LINES.load(Ordering::SeqCst);
        let mut buffer = Vec::new();
        lines_number = 0;

        for target in GLOBAL_OUTPUT.lock().unwrap().iter_mut() {
            let log_target = style(format!("Building target {}", target.id)).blue().bold().to_string();
            lines_number += 1;
            log_line(&log_target, &mut buffer);

            for step in target.steps.iter_mut() {
                if step.status == ProgressStatus::Skipped {
                    let log_action =
                        format!("  {} {}", style("✔").black().bold(), style(&step.description).black());
                    lines_number += 1;
                    log_line(&log_action, &mut buffer);
                } else {
                    let step_log = format!(
                        "  {}",
                        style(&step.description).green()
                    );
                    lines_number += 1;
                    log_line(&step_log, &mut buffer);
                }

                for action in step.actions.iter_mut() {
                    if action.status == ProgressStatus::Done {
                        let log_action =
                            format!("    {} {}", style("✔").green().bold(), action.description);
                        lines_number += 1;
                        log_line(&log_action, &mut buffer);
                    } else if action.status == ProgressStatus::Skipped {
                        let log_action = format!(
                            "    {} {}",
                            style("✔").magenta().bold(),
                            style(&action.description).magenta()
                        );
                        lines_number += 1;
                        log_line(&log_action, &mut buffer);
                    } else if action.status == ProgressStatus::Failed {
                        let log_action = format!(
                            "    {} {}",
                            style("❌").red().bold(),
                            style(&action.description).red()
                        );
                        lines_number += 1;
                        log_line(&log_action, &mut buffer);
                    } else if action.progress == ActionProgressType::Spinner {
                        let iteration_action_id =
                            target.id.clone() + step.id.as_str() + action.id.as_str();

                        if let Some(iteration) = GLOBAL_ITERATIONS.get(&iteration_action_id) {
                            let i = *iteration;
                            drop(iteration); // explicitly drop before mutation

                            let log_action = format!(
                                "    {} {}",
                                SPINNER[i % SPINNER.len()],
                                action.description
                            );
                            lines_number += 1;
                            log_line(&log_action, &mut buffer);

                            GLOBAL_ITERATIONS.insert(iteration_action_id, i + 1);
                        }
                    } else if action.progress == ActionProgressType::Bar {
                        let percent = action.percent.clone().unwrap_or(0);
                        let bar_width = 20; // width of the progress bar

                        let mut filled = percent * bar_width / 100;
                        if filled > 0 {
                            filled -= 1; // Leave space for > symbol
                        }

                        let mut empty = 0;
                        if bar_width >= filled {
                            empty = bar_width - filled;
                        }

                        let bar = format!(
                            "{}{}{}",
                            "=".repeat(filled as usize),
                            ">",
                            " ".repeat(empty as usize)
                        );
                        let log_action =
                            format!("    [{}] {:>3}% {}", bar, percent, action.description);
                        lines_number += 1;
                        log_line(&log_action, &mut buffer);
                    } else {
                        let log_action = format!("    {}", action.description);
                        lines_number += 1;
                        log_line(&log_action, &mut buffer);
                    }
                }
            }
        }

        flush_output(&mut stdout, &buffer);
        PRINTED_LINES.store(lines_number, Ordering::SeqCst);
    }
}
