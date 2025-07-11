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
pub struct CopyAction {
    pub from: Vec<String>,
    pub to: Vec<String>,
}

pub struct Copy;

fn compile<'a>(
    cwd: &'a str,
    emakefile_cwd: &'a str,
    action: &'a PluginAction,
    in_files: &'a Vec<String>,
    out_files: &'a Vec<String>,
    maybe_replacements: Option<&'a HashMap<String, String>>,
) -> Option<(Vec<String>, Vec<String>)> {
    let mut replacements: HashMap<String, String> = HashMap::new();

    if in_files.len() > 0 {
        replacements.insert(String::from("in_files"), in_files[0].clone());
    }

    if out_files.len() > 0 {
        replacements.insert(String::from("out_files"), out_files[0].clone());
    }

    for (index, in_file) in in_files.iter().enumerate() {
        let key = format!("in_files[{}]", index);
        replacements.insert(key, in_file.clone());
    }

    for (index, in_file) in out_files.iter().enumerate() {
        let key = format!("out_files[{}]", index);
        replacements.insert(key, in_file.clone());
    }

    if let Some(default_replacements) = maybe_replacements {
        replacements.extend(default_replacements.to_owned());
    }

    match &action {
        PluginAction::Copy { copy } => {
            let mut from_compiled = Vec::new();
            for from in &copy.from {
                let compile_result = emake::compiler::compile(
                    cwd,
                    from,
                    &emakefile_cwd.to_string(),
                    Some(&replacements),
                );

                from_compiled.push(compile_result);
            }

            let mut to_compiled = Vec::new();
            for to in &copy.to {
                let compile_result = emake::compiler::compile(
                    cwd,
                    to,
                    &emakefile_cwd.to_string(),
                    Some(&replacements),
                );

                to_compiled.push(compile_result);
            }

            return Some((from_compiled, to_compiled));
        }
        _ => {}
    }

    None
}

impl Action for Copy {
    fn insert_in_files<'a>(
        &'a self,
        _action: &'a PluginAction,
        _in_files: &'a mut Vec<InFile>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {})
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
        cwd: &'a str,
        emakefile_cwd: &'a str,
        _silent: bool,
        action: &'a PluginAction,
        in_files: &'a Vec<String>,
        out_files: &'a Vec<String>,
        _working_dir: &'a String,
        maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            let mut has_error = false;
            let mut handles = Vec::new();
            let files = compile(
                cwd,
                emakefile_cwd,
                action,
                in_files,
                out_files,
                maybe_replacements,
            );

            if let Some((from, to)) = files {
                for (index, from) in from.iter().enumerate() {
                    let mut destination = &to[0];

                    if to.len() > index {
                        destination = &to[index];
                    }

                    let src_owned = from.clone();
                    let dest_owned = destination.clone();
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
                        tokio::fs::copy(src_path, dest_path).await.expect(&error);
                    }));
                }

                let results = futures::future::join_all(handles).await;
                for result in results {
                    if let Err(e) = result {
                        log::error!("{:?}", e);
                        has_error = true;
                    }
                }
            } else {
                log::error!("Error when trying to compile from and to parameters");
                has_error = true;
            }

            has_error
        })
    }
    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
}
