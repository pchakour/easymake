use config_macros::ActionDoc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, path::PathBuf, pin::Pin};

use crate::{
    console::log,
    emake::{InFile, PluginAction},
};

use super::Action;
pub static ID: &str = "copy";

#[derive(ActionDoc, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[action_doc(
    id = "copy",
    short_desc = "Copy files or folders to a specific destination",
    example = "
{% raw %}
targets:
    pre_copy_files:
        steps:
            - description: Generate hello world file
              shell:
                out_files: [\"{{ EMAKE_WORKING_DIR }}/hello_world.txt\"]
                cmd: touch {{ out_files }}
    copy_files:
        deps:
            - pre_copy_files
        steps:
            - description: Copy hello world file
              copy:
                from: 
                    - \"{{ EMAKE_WORKING_DIR }}/hello_world.txt\"
                to:
                    - \"{{ EMAKE_OUT_DIR }}/hello_world.txt\"
{% endraw %}
"
)]
pub struct CopyAction {
    #[action_prop(description = "A list of source files to copy", required = true)]
    pub from: Vec<String>,
    #[action_prop(
        description = "A list of destination files. The number of destinations must be one to copy all sources in the destination or must match the number of destination",
        required = true
    )]
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
            let mut handles = Vec::new();

            let from = in_files;
            let to = out_files;

            for (index, from) in from.iter().enumerate() {
                let mut destination = &to[0];

                if to.len() > index {
                    destination = &to[index];
                }

                let action_description = format!("Copying file {} to {}", from, destination);
                log::action_info!(step_id, ID, "{}", action_description);

                let src_owned = from.clone();
                let dest_owned = destination.clone();

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

                    match copy_result {
                        Ok(_) => Ok(()),
                        Err(exception) => Err(format!("{}: {}", error, exception)),
                    }
                }));
            }

            let results = futures::future::join_all(handles).await;
            for result in results {
                if result.is_err() {
                    return Err(format!("{:?}", result.err()).into())
                }
            }

            Ok(())
        })
    }
    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
    fn get_checksum(&self) -> Option<String> {
        None
    }
}
