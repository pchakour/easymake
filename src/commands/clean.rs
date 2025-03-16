use std::path::{Path, PathBuf};

use crate::console::log;

const CACHE_DIR: &str = ".emake";

pub fn run(cwd: &Path) {
    log::step!(1, 1, "Cleaning cache");
    let mut path = PathBuf::from(cwd);
    path.push(CACHE_DIR);
    let _ = std::fs::remove_dir_all(path);
}