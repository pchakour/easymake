use fs_extra::{
    copy_items_with_progress,
    dir::{CopyOptions, TransitProcessResult},
};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, pin::Pin};

use crate::{
    console::{
        log,
        logger::{ActionProgressType, LogAction, Logger, ProgressStatus},
    },
    emake::{InFile, PluginAction},
    MULTI_PROGRESS,
};

use super::Action;
pub static ID: &str = "move";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveAction {
    from: Vec<String>,
    to: String,
}

pub struct Move;

impl Action for Move {
    fn insert_in_files<'a>(
        &'a self,
        action: &'a PluginAction,
        in_files: &'a mut Vec<InFile>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::Move { mv } => {
                    for file in &mv.from {
                        in_files.push(InFile::Simple(file.clone()));
                    }
                }
                _ => {}
            }
        })
    }

    fn insert_out_files<'a>(
        &'a self,
        action: &'a PluginAction,
        out_files: &'a mut Vec<String>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::Move { mv } => {
                    out_files.push(mv.to.clone());
                }
                _ => {}
            }
        })
    }

    fn run<'a>(
        &'a self,
        _cwd: &'a str,
        target_id: &'a str,
        step_id: &'a str,
        _emakefile_cwd: &'a str,
        _silent: bool,
        _action: &'a PluginAction,
        in_files: &'a Vec<String>,
        out_files: &'a Vec<String>,
        _working_dir: &'a String,
        _maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            let mut has_error = false;
            let src = in_files.clone();
            let destination = out_files[0].clone();
            let options = CopyOptions {
                overwrite: true,
                skip_exist: false,
                copy_inside: true,
                ..Default::default()
            };
            let action_id = String::from(ID) + &src.join(";") + &destination;

            Logger::set_action(
                target_id.to_string(),
                step_id.to_string(),
                LogAction {
                    id: action_id.clone(),
                    status: ProgressStatus::Progress,
                    description: String::from("Moving files"),
                    progress: ActionProgressType::Bar,
                    percent: Some(0),
                },
            );

            // We use copy because move is not working correctly
            let copy_result =
                copy_items_with_progress(&src, &destination, &options, |process_info| {
                    Logger::set_action(
                        target_id.to_string(),
                        step_id.to_string(),
                        LogAction {
                            id: action_id.clone(),
                            status: ProgressStatus::Progress,
                            description: format!("Copying file {}", process_info.dir_name),
                            progress: ActionProgressType::Bar,
                            percent: Some(if process_info.total_bytes > 0 {
                                ((process_info.copied_bytes * 100) / process_info.total_bytes)
                                    as usize
                            } else {
                                0
                            }),
                        },
                    );

                    TransitProcessResult::ContinueOrAbort
                });

            if copy_result.is_err() {
                log::error!("{}", copy_result.err().unwrap());
                has_error = true;
            }

            Logger::set_action(
                target_id.to_string(),
                step_id.to_string(),
                LogAction {
                    id: action_id.clone(),
                    status: ProgressStatus::Progress,
                    description: String::from("Removing source files"),
                    progress: ActionProgressType::Spinner,
                    percent: None,
                },
            );
            let remove_result = fs_extra::remove_items(&src);
            if remove_result.is_err() {
                log::error!("{}", remove_result.err().unwrap());
                has_error = true;
            }

            Logger::set_action(
                target_id.to_string(),
                step_id.to_string(),
                LogAction {
                    id: action_id.clone(),
                    status: ProgressStatus::Done,
                    description: String::from("Move files is done"),
                    progress: ActionProgressType::Spinner,
                    percent: None,
                },
            );

            has_error
        })
    }
    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
}
