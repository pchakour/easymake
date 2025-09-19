use std::{cmp, io::Write, sync::Mutex};

use console::style;
use crossterm::{
    cursor::{MoveToColumn, MoveUp},
    queue,
    terminal::{Clear, ClearType},
};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use terminal_size::{terminal_size, Width};
use textwrap::wrap;

use crate::console::log;

type TargetId = String;
type StepId = String;

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
    None,
}

#[derive(Debug, Clone)]
pub struct LogAction {
    pub id: String,
    pub description: String,
    pub status: ProgressStatus,
    pub progress: ActionProgressType,
    pub percent: Option<usize>,
}

#[derive(Debug)]
pub struct LogStep {
    pub id: String,
    pub description: String,
    pub actions: Vec<LogAction>,
    pub status: ProgressStatus,
}

#[derive(Debug)]
pub struct LogTarget {
    pub id: String,
    pub description: Option<String>,
    pub steps: Vec<LogStep>,
    pub status: ProgressStatus,
}

pub static GLOBAL_OUTPUT: Lazy<Mutex<Vec<LogTarget>>> = Lazy::new(|| Mutex::new(Vec::new()));
static GLOBAL_ITERATIONS: Lazy<DashMap<String, usize>> = Lazy::new(DashMap::new);
const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
static BUFFER_OUTPUT: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));
static MUTEX_WRITE_CONSOLE: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct Logger;

fn get_terminal_width() -> usize {
    terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80)
}

fn log_line(text: &str, buffer: &mut Vec<String>) {
    let width: usize = get_terminal_width();
    let wrapped_lines = wrap(text, width);

    for line in wrapped_lines {
        buffer.push(line.to_string());
    }

    // let line = truncate_str(text, width, "...");
    // buffer.push(line.to_string());
}

fn flush_output(stdout: &mut std::io::Stdout, buffer: &Vec<String>) {
    let mut previous_buffer = BUFFER_OUTPUT.lock().unwrap();
    let (_, rows) = crossterm::terminal::size().unwrap();
    let displayed_lines = cmp::min(buffer.len(), rows as usize);

    let start = buffer.len() - displayed_lines;
    let visible_lines = &buffer[start..];

    // Move cursor to top of printed output
    let move_up = previous_buffer.len().min(rows as usize) as u16;
    if move_up > 0 {
        queue!(stdout, MoveUp(move_up)).unwrap();
        queue!(stdout, Clear(ClearType::FromCursorDown)).unwrap();
    }

    for (i, line) in buffer.iter().enumerate() {
        if previous_buffer.len() > i {
            previous_buffer[i] = line.clone();
        } else {
            previous_buffer.push(line.clone());
        }
    }

    for (_, line) in visible_lines.iter().enumerate() {
        queue!(stdout, MoveToColumn(0)).unwrap();

        // if previous_buffer.len() > i {
        // if previous_buffer[i] != *line {
        queue!(stdout, Clear(ClearType::CurrentLine)).unwrap();
        println!("{}", line);
        // previous_buffer[i] = line.clone();
        // } else {
        //     execute!(stdout, MoveDown(0)).unwrap();
        // }
        // } else {
        //     execute!(stdout, Clear(ClearType::CurrentLine)).unwrap();
        //     println!("{}", line);
        //     previous_buffer.push(line.clone());
        // }
    }

    // Clear extra old lines if buffer shrunk
    if previous_buffer.len() > buffer.len() {
        previous_buffer.truncate(visible_lines.len());
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

    pub fn close() {
        Logger::write();
    }

    pub fn write() {
        let _guard = MUTEX_WRITE_CONSOLE.lock().unwrap();
        let mut stdout = std::io::stdout();
        let mut buffer = Vec::new();

        for target in GLOBAL_OUTPUT.lock().unwrap().iter_mut() {
            // if target.status != ProgressStatus::Progress {
            //     continue;
            // }

            // println!("TARGET {:?}", target);

            let mut log_target = style(format!("Building target {}", target.id))
                .blue()
                .bold()
                .to_string();
            if target.status == ProgressStatus::Done {
                log_target = style(format!(
                    "{} {}",
                    style("✔").green().bold(),
                    style(format!("Building target {}", target.id)).blue()
                ))
                .to_string();
            }
            log_line(&log_target, &mut buffer);

            for step in target.steps.iter_mut() {
                if step.status == ProgressStatus::Skipped {
                    let log_action = format!("  {}", style(&step.description).black());
                    log_line(&log_action, &mut buffer);
                } else if step.status == ProgressStatus::Done {
                    let log_action =
                        format!("  {} {}", style("✔").green().bold(), &step.description);
                    log_line(&log_action, &mut buffer);
                } else {
                    let step_log = format!("  {}", style(&step.description).green());
                    log_line(&step_log, &mut buffer);
                }

                for action in step.actions.iter_mut() {
                    // if action.status == ProgressStatus::Done {
                    //     continue;
                    // }
                    // let log_action =
                    //     format!("    {} {}", style("✔").green().bold(), action.description);
                    // log_line(&log_action, &mut buffer);
                    if action.status == ProgressStatus::Skipped {
                        let log_action = format!(
                            "    {} {}",
                            style("✔").magenta().bold(),
                            style(&action.description).magenta()
                        );
                        log_line(&log_action, &mut buffer);
                    } else if action.status == ProgressStatus::Failed {
                        let log_action = format!(
                            "    {} {}",
                            style("❌").red().bold(),
                            style(&action.description).red()
                        );
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
                        log_line(&log_action, &mut buffer);
                    } else {
                        let log_action = format!("    {}", action.description);
                        log_line(&log_action, &mut buffer);
                    }
                }
            }
        }

        flush_output(&mut stdout, &buffer);
    }
}
