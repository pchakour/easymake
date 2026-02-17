use config_macros::ActionDoc;
use fs_extra::dir::CopyOptions;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, future::Future, path::PathBuf, pin::Pin};

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
targets:
    pre_copy:
        steps:
            - description: Generate hello world file
              shell:
                out_files: [\"{{ EMAKE_WORKING_DIR }}/hello_world.txt\"]
                cmd: touch {{ out_files }}
    copy:
        deps:
            - pre_copy
        steps:
            - description: Copy hello world file
              copy:
                from: 
                    - \"{{ EMAKE_WORKING_DIR }}/hello_world.txt\"
                to: \"{{ EMAKE_OUT_DIR }}/hello_world.txt\"
"
)]
pub struct CopyAction {
    #[action_prop(description = "A list of source files to copy", required = true)]
    pub from: Vec<String>,
    #[action_prop(
        description = "A list of destination files. The number of destinations must be one to copy all sources in the destination or must match the number of destination",
        required = true
    )]
    pub to: String,
    #[action_prop(
        description = "Overwrite if dest files already exists",
        required = false,
        default = true,
    )]
    pub overwrite: Option<bool>,

    #[action_prop(
        description = "Ignore dest file if already exists",
        required = false,
        default = false,
    )]
    pub skip_exist: Option<bool>,
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
                    let dest_path = PathBuf::from(&copy.to);
                    if dest_path.is_dir() || dest_path.ends_with("/") {
                        for src in &copy.from {
                            let src_path = PathBuf::from(src);
                            let dirname = src_path.file_name().unwrap();
                            out_files.push(PathBuf::from(copy.to.to_string()).join(&dirname).to_string_lossy().to_string());
                        }
                    } else {
                        out_files.push(copy.to.to_string());
                    }
                }
                _ => {}
            }
        })
    }

    fn run<'a>(
        &'a self,
        _target_id: &'a str,
        step_id: &'a str,
        _emakefile_cwd: &'a str,
        _silent: bool,
        action: &'a PluginAction,
        in_files: &'a Vec<String>,
        out_files: &'a Vec<String>,
        _working_dir: &'a String,
        _maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>> {
        Box::pin(async move {
            let copy_action = match action {
                PluginAction::Copy { copy } => {
                    copy
                },
                _ => {
                    log::panic!("Error when using copy");
                }
            };
            let mut handles = Vec::new();

            let from: &Vec<String> = in_files;
            let destination = &out_files[0];

            for (_, from) in from.iter().enumerate() {
                let action_description = format!("Copying file {} to {}", from, destination);
                log::action_info!(step_id, ID, "{}", action_description);

                let src_owned = from.clone();
                let dest_owned = destination.clone();
                let overwrite = copy_action.overwrite.unwrap_or(true);
                let skip_exist = copy_action.skip_exist.unwrap_or(false);

                handles.push(tokio::spawn(async move {
                    let src_path = PathBuf::from(&src_owned);
                    let dest_path = PathBuf::from(&dest_owned);

                    
                    // Decide if destination is a directory
                    let is_dest_dir = dest_path.is_dir() || dest_owned.ends_with('/');
                    
                    // Ensure directory exists
                    let dest_dir = if is_dest_dir {
                        dest_path.clone()
                    } else {
                        dest_path.parent().unwrap().to_path_buf()
                    };
                    
                    fs::create_dir_all(&dest_dir).unwrap();

                    if dest_dir.is_dir() {
                        let options = CopyOptions {
                            overwrite,
                            skip_exist,
                            ..Default::default()
                        };

                        let result = fs_extra::copy_items(&[&src_owned], &dest_dir, &options);

                        if let Err(e) = result {
                            return Err(format!(
                                "Can't copy using fs_extra::copy_items function {} → {}: {:?}",
                                src_owned, dest_owned, e
                            ));
                        }
                    } else {
                        if dest_path.exists() && !skip_exist && !overwrite {
                            return Err(format!("Dest path {} already exists", dest_path.to_str().unwrap()));
                        }

                        if let Err(e) = fs::copy(&src_path, &dest_path) {
                            return Err(format!(
                                "Can't copy using fs::copy function {} → {}: {:?}",
                                src_owned, dest_owned, e
                            ));
                        }
                    }

                    Ok(())
                }));
            }

            let results = futures::future::join_all(handles).await;
            for result in results {
                if result.is_err() {
                    return Err(format!("{:?}", result.err().unwrap()).into());
                } else {
                    let thread_result = result.unwrap();
                    if thread_result.is_err() {
                        return Err(format!("{:?}", thread_result.err().unwrap()).into());
                    }
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
