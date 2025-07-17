use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, path::PathBuf, pin::Pin};

use crate::{
    console::{
        log,
        logger::{ActionProgressType, LogAction, Logger, ProgressStatus},
    },
    emake::{InFile, PluginAction},
    GLOBAL_SEMAPHORE,
};

use super::Action;
pub static ID: &str = "copy";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyAction {
    pub from: Vec<String>,
    pub to: Vec<String>,
}

pub struct Copy;

impl Action for Copy {
    fn insert_in_files<'a>(
        &'a self,
        action: &'a PluginAction,
        in_files: &'a mut Vec<InFile>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::Copy { copy } => {
                    for from in &copy.from {
                        in_files.push(InFile::Simple(from.to_string()));
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
                PluginAction::Copy { copy } => {
                    for to in &copy.to {
                        out_files.push(to.to_string());
                    }
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
            let mut handles = Vec::new();

            let from = in_files;
            let to = out_files;

            for (index, from) in from.iter().enumerate() {
                let mut destination = &to[0];

                if to.len() > index {
                    destination = &to[index];
                }
                let action_id = String::from(target_id) + step_id + ID + from + destination;

                let action_description = format!("Copying file {} to {}", from, destination);

                Logger::set_action(
                    target_id.to_string(),
                    step_id.to_string(),
                    LogAction {
                        id: action_id.clone(),
                        status: ProgressStatus::Progress,
                        description: action_description.clone(),
                        progress: ActionProgressType::Spinner,
                        percent: None,
                    },
                );

                let src_owned = from.clone();
                let dest_owned = destination.clone();
                let action_id_clone = action_id.clone();
                let target_id_clone = target_id.to_string();
                let step_id_clone = step_id.to_string();
                let action_description_clone = action_description.clone();
                let _s = GLOBAL_SEMAPHORE.acquire().await;

                handles.push(tokio::spawn(async move {
                    let mut dest_path = PathBuf::from(&dest_owned);
                    let src_path = PathBuf::from(&src_owned);
                    if dest_path.is_dir() {
                        let filename = src_path.file_name().unwrap().to_str().unwrap();
                        dest_path = dest_path.join(filename);
                    }

                    let dest_dir = dest_path.parent().unwrap();
                    if !tokio::fs::try_exists(dest_dir).await.unwrap() {
                        tokio::fs::create_dir_all(dest_dir).await.unwrap();
                    }

                    let error = format!("Can't copy from {} to {}", src_owned, dest_owned);
                    let copy_result = tokio::fs::copy(src_path, dest_path).await;

                    if copy_result.is_err() {
                        Logger::set_action(
                            target_id_clone,
                            step_id_clone,
                            LogAction {
                                id: action_id_clone.clone(),
                                status: ProgressStatus::Failed,
                                description: error,
                                progress: ActionProgressType::Spinner,
                                percent: None,
                            },
                        );
                    } else {
                        Logger::set_action(
                            target_id_clone,
                            step_id_clone,
                            LogAction {
                                id: action_id_clone.clone(),
                                status: ProgressStatus::Done,
                                description: action_description_clone,
                                progress: ActionProgressType::Spinner,
                                percent: None,
                            },
                        );  
                    }

                    copy_result
                }));
            }

            let results = futures::future::join_all(handles).await;
            for result in results {
                if result.is_err() {
                    has_error = true;
                    break;
                }
            }

            has_error
        })
    }
    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
}
