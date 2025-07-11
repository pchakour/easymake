use fs_extra::{
    copy_items_with_progress,
    dir::{CopyOptions, TransitProcessResult},
    move_items_with_progress, TransitProcess,
};
use serde::{Deserialize, Serialize};
use serde_yml::Value;
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    pin::Pin,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use crate::{
    console::log,
    emake::{self, InFile, PluginAction},
    graph::{self, Node},
    GLOBAL_SEMAPHORE,
};

use super::Action;
pub static ID: &str = "copy";

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
        cwd: &'a str,
        emakefile_cwd: &'a str,
        _silent: bool,
        action: &'a PluginAction,
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

            // We use copy because move is not working correctly
            let copy_result = copy_items_with_progress(&src, &destination, &options, |process_info| {
                let percent =
                    process_info.copied_bytes as f64 / process_info.total_bytes as f64 * 100.0;
                println!("Progress: {}%", percent);

                TransitProcessResult::ContinueOrAbort
            });

            if copy_result.is_err() {
                log::error!("{}", copy_result.err().unwrap());
                has_error = true;
            }

            let remove_result = fs_extra::remove_items(&src);
            if remove_result.is_err() {
                log::error!("{}", remove_result.err().unwrap());
                has_error = true;
            }

            has_error
        })
    }
    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
}
