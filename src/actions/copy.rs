use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap},
    future::Future,
    path::{PathBuf},
    pin::Pin,
};

use crate::{
    console::log,
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

            has_error
        })
    }
    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
}
