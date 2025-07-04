use serde_yml::Value;
use std::{
    collections::{HashMap, HashSet}, future::Future, path::{Path, PathBuf}, pin::Pin
};

use crate::{commands::build, console::log, emake, graph::{self, generator::get_absolute_target_path, Node}};

use super::Action;
pub static ID: &str = "target";

pub struct Target;

impl Action for Target {
    fn generate<'a>(
        &'a self,
        _cwd: &'a Path,
        _args: &'a Value,
        _emakefile: &mut emake::Emakefile,
        _graph: &'a mut graph::Graph,
        _visited: &'a mut HashSet<String>,
    ) -> Option<Node> {
        None
    }

    fn run<'a>(
        &'a self,
        cwd: &'a str,
        emakefile_cwd: &'a str,
        silent: bool,
        args: &'a Value,
        _in_files: &'a Vec<String>,
        _out_files: &'a Vec<String>,
        _working_dir: &'a String,
        _maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            let target = args.as_str().unwrap_or("").to_string();
            let real_target = get_absolute_target_path(&target,&emakefile_cwd.to_string(), Path::new(cwd));
            log::info!("\n\n==========> Running subtarget {real_target} {target} {emakefile_cwd}\n\n");
            build::run(&real_target, &silent, std::path::PathBuf::from(cwd).as_path(), false).await;
            log::info!("\n\n==========> End subtarget\n\n");
            false
        })
    }

    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
}
