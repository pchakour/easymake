use config_macros::ActionDoc;
use fs_extra::{
    copy_items_with_progress,
    dir::{CopyOptions, TransitProcessResult},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, pin::Pin};

use crate::{
    console::log,
    emake::{InFile, PluginAction},
};

use super::Action;
pub static ID: &str = "move";

#[derive(ActionDoc, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[action_doc(
    id = "move",
    short_desc = "Move files",
    example = "
{% raw %}
targets:
    extraction_example:
        steps:
            - description: Retrieve and move url folder
              move:
                from: 
                    - https://github.com/pchakour/easymake/archive/refs/heads/main.zip
                to: \"{{ EMAKE_OUT_DIR }}/easymake_moved\"
{% endraw %}
"
)]
pub struct MoveAction {
    #[action_prop(description = "A list of source files to move", required = true)]
    from: Vec<InFile>,
    #[action_prop(
        description = "The destination to move source files. \
    Can be a folder or a filename if the from property contains only one file. The folder will be automatically created if doesn't exist",
        required = true
    )]
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
                        in_files.push(file.clone());
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
        _target_id: &'a str,
        step_id: &'a str,
        _emakefile_cwd: &'a str,
        _silent: bool,
        _action: &'a PluginAction,
        in_files: &'a Vec<String>,
        out_files: &'a Vec<String>,
        _working_dir: &'a String,
        _maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>> {
        Box::pin(async move {
            let src = in_files.clone();
            let destination = out_files[0].clone();
            let options = CopyOptions {
                overwrite: true,
                skip_exist: false,
                copy_inside: true,
                ..Default::default()
            };

            // We use copy because move is not working correctly
            let copy_result =
                copy_items_with_progress(&src, &destination, &options, |process_info| {
                    let mut percent = 0;
                    
                    if process_info.total_bytes > 0 {
                        percent = ((process_info.copied_bytes * 100) / process_info.total_bytes) as usize
                    }

                    log::action_debug!(
                        step_id,
                        ID,
                        "Percent {}% | Copying file {}",
                        percent,
                        process_info.dir_name
                    );

                    TransitProcessResult::ContinueOrAbort
                });

            if copy_result.is_err() {
                return Err(format!("{}", copy_result.err().unwrap()).into());
            }

            log::action_info!(step_id, ID, "Removing source files");
            fs_extra::remove_items(&src).map_err(|error| {
                format!("{}", error).into()
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
