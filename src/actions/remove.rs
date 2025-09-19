use config_macros::ActionDoc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, pin::Pin};

use crate::{
    console::{
        logger::{ActionProgressType, LogAction, Logger, ProgressStatus},
    },
    emake::{InFile, PluginAction},
};

use super::Action;
pub static ID: &str = "remove";

#[derive(ActionDoc, Debug, Clone, Serialize, Deserialize)]
#[action_doc(
    id = "remove",
    short_desc = "Remove a list of paths",
    example = "
{% raw %}
targets:
    pre_remove:
        steps:
            - description: Creating a file to remove
              shell:
                out_files:
                    - \"{{ EMAKE_OUT_DIR }}/hello.txt\"
                cmd: echo 'hello' > {{ out_files }}
    remove_example:
        steps:
            - description: Remove file
              remove:
                paths:
                    - \"{{ EMAKE_OUT_DIR }}/hello.txt\"
{% endraw %}
"
)]
pub struct RemoveAction {
    #[action_prop(description = "List of path to remove. Could be folders or files", required = true)]
    pub paths: Vec<String>,
}

pub struct Remove;

impl Action for Remove {
    fn insert_in_files<'a>(
        &'a self,
        action: &'a PluginAction,
        in_files: &'a mut Vec<InFile>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::Remove { remove } => {
                    for path in &remove.paths {
                        in_files.push(InFile::Simple(path.to_string()));
                    }
                }
                _ => {}
            }
        })
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
        _cwd: &'a str,
        target_id: &'a str,
        step_id: &'a str,
        _emakefile_cwd: &'a str,
        _silent: bool,
        _action: &'a PluginAction,
        in_files: &'a Vec<String>,
        _out_files: &'a Vec<String>,
        _working_dir: &'a String,
        _maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            let mut has_error = false;
            let paths = in_files;
            let action_id = String::from(target_id) + step_id + ID + paths.join(",").as_str();

            // Check if path exists otherwise log a warning
            // for path in paths {
            //     if !std::fs::exists(&path).unwrap() {
            //         // TODO log a warning
            //         continue;
            //     }
            // }

            Logger::set_action(
                target_id.to_string(),
                step_id.to_string(),
                LogAction {
                    id: action_id.clone(),
                    status: ProgressStatus::Progress,
                    description: String::from("Deleting files"),
                    progress: ActionProgressType::Spinner,
                    percent: Some(0),
                },
            );

            let remove_result = fs_extra::remove_items(&paths);

            if remove_result.is_err() {
                Logger::set_action(
                    target_id.to_string(),
                    step_id.to_string(),
                    LogAction {
                        id: action_id.clone(),
                        status: ProgressStatus::Failed,
                        description: format!("Error when deleting files {}", remove_result.err().unwrap()),
                        progress: ActionProgressType::Spinner,
                        percent: Some(0),
                    },
                );
                has_error = true;
            } else {
                Logger::set_action(
                    target_id.to_string(),
                    step_id.to_string(),
                    LogAction {
                        id: action_id.clone(),
                        status: ProgressStatus::Done,
                        description: String::from("Deleting files"),
                        progress: ActionProgressType::Spinner,
                        percent: Some(0),
                    },
                );
            }

            has_error
        })
    }
    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
}
