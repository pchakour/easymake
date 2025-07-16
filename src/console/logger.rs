use std::{
    io::{stdout, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    thread::sleep,
    time::Duration,
};

use console::style;
use crossterm::{
    cursor::{MoveToColumn, MoveUp, RestorePosition, SavePosition},
    execute,
    style::Print,
    terminal::{Clear, ClearType},
};
use dashmap::DashMap;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use terminal_size::{terminal_size, Width};
use textwrap::wrap;

use crate::console::log;

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
    pub description: Option<String>,
    pub actions: Vec<LogAction>,
    pub status: ProgressStatus,
}

pub struct LogTarget {
    pub id: String,
    pub description: Option<String>,
    pub steps: Vec<LogStep>,
    pub status: ProgressStatus,
}

enum LogType {
    Step,
    Target,
    Action,
}

static GLOBAL_OUTPUT: Lazy<Mutex<Vec<LogTarget>>> = Lazy::new(|| Mutex::new(Vec::new()));
static GLOBAL_ITERATIONS: Lazy<DashMap<String, usize>> = Lazy::new(DashMap::new);
static PRINTED_LINES: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));
const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct Logger;

fn get_terminal_width() -> usize {
    terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80)
}

fn log_line(log_type: LogType, text: &str) -> usize {
    let width = get_terminal_width();
    let wrapped_lines = wrap(text, width);

    match log_type {
        LogType::Step => {
            for line in &wrapped_lines {
                log::step!("{}", line);
            }
        }
        LogType::Target => {
            for line in &wrapped_lines {
                log::info!("{}", line);
            }
        }
        LogType::Action => {
            for line in &wrapped_lines {
                log::text!("{}", line);
            }
        }
    }

    wrapped_lines.len()
}

impl Logger {
    pub fn set_target(log_target: LogTarget) {
        let mut guard = GLOBAL_OUTPUT.lock().unwrap();
        guard.push(log_target);
    }

    pub fn set_step(target_id: TargetId, log_step: LogStep) {
        match GLOBAL_OUTPUT
            .lock()
            .unwrap()
            .iter_mut()
            .find(|item| item.id == target_id)
        {
            Some(target) => {
                target.steps.push(log_step);
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

        if lines_number > 0 {
            execute!(stdout, MoveUp(lines_number as u16)).unwrap();
        }

        execute!(stdout, Clear(ClearType::FromCursorDown)).unwrap();
        lines_number = 0;

        for target in GLOBAL_OUTPUT.lock().unwrap().iter_mut() {
            let log_target = format!("Building target {}", target.id);
            lines_number += log_line(LogType::Target, &log_target);

            for step in target.steps.iter_mut() {
                let step_log = format!(
                    "  Running step {}",
                    step.description.clone().unwrap_or(step.id.clone())
                );
                lines_number += log_line(LogType::Step, &step_log);

                for action in step.actions.iter_mut() {
                    if action.status == ProgressStatus::Done {
                        let log_action =
                            format!("    {} {}", style("✔").green().bold(), action.description);
                        lines_number += log_line(LogType::Action, &log_action);
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
                            lines_number += log_line(LogType::Action, &log_action);

                            GLOBAL_ITERATIONS.insert(iteration_action_id, i + 1);
                        }
                    } else if action.progress == ActionProgressType::Bar {
                        let percent = action.percent.clone().unwrap_or(0);
                        let bar_width = 20; // width of the progress bar

                        let mut filled = percent * bar_width / 100;
                        if filled > 0 {
                            filled -= 1;
                        }

                        let empty = bar_width - filled;

                        let bar = format!(
                            "{}{}{}",
                            "=".repeat(filled as usize),
                            ">",
                            " ".repeat(empty as usize)
                        );
                        let log_action =
                            format!("    [{}] {:>3}% {}", bar, percent, action.description);
                        lines_number += log_line(LogType::Action, &log_action);
                    } else {
                        let log_action = format!("    {}", action.description);
                        lines_number += log_line(LogType::Action, &log_action);
                    }
                }
            }
        }

        PRINTED_LINES.store(lines_number, Ordering::SeqCst);
        stdout.flush().unwrap();
    }
}
