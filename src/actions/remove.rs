use config_macros::ActionDoc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, pin::Pin};

use crate::{
    emake::{InFile, PluginAction},
};

use super::Action;
pub static ID: &str = "remove";

#[derive(ActionDoc, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    #[action_prop(
        description = "List of path to remove. Could be folders or files",
        required = true
    )]
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
        _target_id: &'a str,
        _step_id: &'a str,
        _emakefile_cwd: &'a str,
        _silent: bool,
        _action: &'a PluginAction,
        in_files: &'a Vec<String>,
        _out_files: &'a Vec<String>,
        _working_dir: &'a String,
        _maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>> {
        Box::pin(async move {
            let paths = in_files;

            fs_extra::remove_items(&paths).map_err(|error| {
                format!("Error when deleting files {}", error).into()
            })
        })
    }
    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
    fn get_checksum(&self) -> Option<String> {
        None
    }
}
