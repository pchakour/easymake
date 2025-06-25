use serde_yml::Value;
use std::{
    collections::HashMap, future::Future, path::PathBuf, pin::Pin
};

use crate::{commands::build, console::log};

use super::Plugin;
pub static ID: &str = "target";

pub struct Target;

impl Plugin for Target {
    fn action<'a>(
        &'a self,
        cwd: &'a str,
        emakefile_cwd: &'a str,
        silent: bool,
        args: &'a Value,
        _in_files: &'a Vec<String>,
        _out_files: &'a Vec<String>,
        _working_dir: &'a String,
        _maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let target = args.as_str().unwrap_or("").to_string();
            
            let mut real_target = target.clone();
            if !target.starts_with('/') {
                let emakefile_absolute_path = PathBuf::from(emakefile_cwd);
                let emakefile_relative_path = emakefile_absolute_path.parent().unwrap().strip_prefix(cwd).unwrap();
                real_target = format!("/{}", emakefile_relative_path.join(&target).to_string_lossy());
            }
            log::info!("\n\n==========> Running subtarget {real_target} {target} {emakefile_cwd}\n\n");
            build::run(&real_target, &silent, std::path::PathBuf::from(cwd).as_path()).await;
            log::info!("\n\n==========> End subtarget\n\n");
        })
    }

    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync> {
        Box::new(Self)
    }
}
