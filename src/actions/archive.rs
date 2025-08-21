use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipWriter};

use crate::{
    actions,
    console::{
        log,
        logger::{ActionProgressType, LogAction, Logger, ProgressStatus},
    },
    emake::{InFile, PluginAction},
};
use flate2::{write::GzEncoder, Compression};
use globset::{Glob, GlobSet, GlobSetBuilder};
use xz2::write::XzEncoder;
use zstd::stream::Encoder as ZstdEncoder;

use super::Action;
pub static ID: &str = "archive";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveAction {
    from: String,
    to: String,
    exclude: Option<Vec<String>>,
}

pub struct Archive;

fn build_exclude_globset(patterns: &Vec<String>) -> Result<GlobSet, Box<dyn std::error::Error>> {
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        builder.add(Glob::new(pat)?);
    }
    Ok(builder.build()?)
}

fn walk_with_excludes<'a>(
    dir_path: &'a Path,
    exclude_patterns: &Vec<String>,
) -> Result<Vec<walkdir::DirEntry>, Box<dyn std::error::Error>> {
    let globset = build_exclude_globset(exclude_patterns)?;

    let entries = WalkDir::new(dir_path)
        .into_iter()
        .filter_entry(|entry| {
            // Convert the path to a relative path (optional but more flexible)
            let rel = entry.path().strip_prefix(dir_path).unwrap_or(entry.path());

            // Exclude if matches any pattern
            !globset.is_match(rel)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(entries)
}

fn archive(
    target_id: &str,
    step_id: &str,
    dir_to_archive: &str,
    archive_path: &str,
    exclude_paths: &Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(archive_path);
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let action_id = String::from("ARCHIVE") + archive_path;

    Logger::set_action(
        target_id.to_string(),
        step_id.to_string(),
        LogAction {
            id: action_id.clone(),
            status: ProgressStatus::Progress,
            description: String::from("Starting files compression"),
            progress: ActionProgressType::Spinner,
            percent: None,
        },
    );

    let dir_path = Path::new(dir_to_archive);

    if extension == "zip" {
        let file = std::fs::File::create(path)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for entry in walkdir::WalkDir::new(dir_path) {
            let entry = entry?;
            let entry_path = entry.path();
            let relative_path = entry_path.strip_prefix(dir_path)?;

            let name = relative_path
                .to_str()
                .ok_or("Failed to convert path to string")?
                .replace('\\', "/"); // ensure consistent path separator

            Logger::set_action(
                target_id.to_string(),
                step_id.to_string(),
                LogAction {
                    id: action_id.clone(),
                    status: ProgressStatus::Progress,
                    description: format!("Compressing: {}", name),
                    progress: ActionProgressType::Spinner,
                    percent: None,
                },
            );

            if entry.file_type().is_file() {
                zip.start_file(&name, options)?;
                let mut f = std::fs::File::open(entry_path)?;
                std::io::copy(&mut f, &mut zip)?;
            } else if entry.file_type().is_dir() {
                // For directories, add a trailing slash to mark it as a folder in the zip
                let dir_name = format!("{}/", name);
                zip.add_directory(dir_name, options)?;
            }
        }

        zip.finish()?;
    } else if archive_path.ends_with(".tar.gz") {
        let dir_path = Path::new(dir_to_archive);
        let tar_gz = std::fs::File::create(archive_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);
        let entries: Vec<_> = walk_with_excludes(dir_path, exclude_paths)?;
        let total = entries.len();
        let mut current = 0;
        for entry in entries {
            let entry_path = entry.path();
            let mut relative_path = entry_path.strip_prefix(dir_path)?;
            if relative_path.as_os_str().is_empty() {
                relative_path = Path::new(".");
            }
            let name = relative_path
                .to_str()
                .ok_or("Failed to convert relative path to str")?
                .replace('\\', "/");
            current += 1;
            let percent = current * 100 / total;
            Logger::set_action(
                target_id.to_string(),
                step_id.to_string(),
                LogAction {
                    id: action_id.clone(),
                    status: ProgressStatus::Progress,
                    description: format!("Compressing: {}", name),
                    progress: ActionProgressType::Bar,
                    percent: Some(percent),
                },
            );
            if entry.file_type().is_dir() {
                tar.append_dir(relative_path, entry_path)?;
            } else if entry.file_type().is_file() {
                let mut file = std::fs::File::open(entry_path)?;
                tar.append_file(relative_path, &mut file)?;
            }
        }
        tar.finish()?;
    } else if archive_path.ends_with(".tar.zst") {
        let dir_path = Path::new(dir_to_archive);

        // Create destination file
        let tar_file = std::fs::File::create(archive_path)?;

        // Create zstd encoder with compression level 3
        let mut enc = ZstdEncoder::new(tar_file, 3)?;
        // Enable multithreading (use all cores)
        enc.multithread(num_cpus::get() as u32)?;

        let mut tar = tar::Builder::new(enc);

        let entries: Vec<_> = walk_with_excludes(dir_path, exclude_paths)?;
        let total = entries.len();
        let mut current = 0;

        for entry in entries {
            let entry_path = entry.path();
            let mut relative_path = entry_path.strip_prefix(dir_path)?;
            if relative_path.as_os_str().is_empty() {
                relative_path = Path::new(".");
            }

            let name = relative_path
                .to_str()
                .ok_or("Failed to convert relative path to str")?
                .replace('\\', "/");

            current += 1;
            let percent = current * 100 / total;

            // Progress log
            Logger::set_action(
                target_id.to_string(),
                step_id.to_string(),
                LogAction {
                    id: action_id.to_string(),
                    status: ProgressStatus::Progress,
                    description: format!("Compressing: {}", name),
                    progress: ActionProgressType::Bar,
                    percent: Some(percent),
                },
            );

            if entry.file_type().is_dir() {
                tar.append_dir(relative_path, entry_path)?;
            } else if entry.file_type().is_file() {
                let mut file = std::fs::File::open(entry_path)?;
                tar.append_file(relative_path, &mut file)?;
            }
        }

        // Finish tar, then encoder
        tar.finish()?;
        let enc = tar.into_inner()?; // get back the zstd encoder
        enc.finish()?; // finalize zstd
    } else if archive_path.ends_with(".tar.xz") {
        let dir_path = Path::new(dir_to_archive);
        let tar_xz = std::fs::File::create(archive_path)?;
        let enc = XzEncoder::new(tar_xz, 6);
        let mut tar = tar::Builder::new(enc);

        let entries: Vec<_> = WalkDir::new(dir_path)
            .into_iter()
            .collect::<Result<_, _>>()?;
        let total = entries.len();
        let mut current = 0;

        for entry in entries {
            let entry_path = entry.path();
            let mut relative_path = entry_path.strip_prefix(dir_path)?;
            if relative_path.as_os_str().is_empty() {
                relative_path = Path::new(".");
            }

            let name = relative_path
                .to_str()
                .ok_or("Failed to convert relative path to str")?
                .replace('\\', "/");

            current += 1;
            let percent = current * 100 / total;

            // Real-time feedback (you can use Logger instead of println! if preferred)
            Logger::set_action(
                target_id.to_string(),
                step_id.to_string(),
                LogAction {
                    id: action_id.clone(),
                    status: ProgressStatus::Progress,
                    description: format!("Compressing: {}", name),
                    progress: ActionProgressType::Bar,
                    percent: Some(percent),
                },
            );

            if entry.file_type().is_dir() {
                tar.append_dir(relative_path, entry_path)?;
            } else if entry.file_type().is_file() {
                let mut file = std::fs::File::open(entry_path)?;
                tar.append_file(relative_path, &mut file)?;
            }
        }

        tar.finish()?;
    } else {
        return Err(format!("Unsupported archive format: {}", archive_path).into());
    }

    Logger::set_action(
        target_id.to_string(),
        step_id.to_string(),
        LogAction {
            id: action_id,
            status: ProgressStatus::Done,
            description: String::from("Archive completed"),
            progress: ActionProgressType::None,
            percent: Some(100),
        },
    );

    Ok(())
}

impl Action for Archive {
    fn insert_in_files<'a>(
        &'a self,
        action: &'a PluginAction,
        in_files: &'a mut Vec<InFile>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::Archive { archive } => {
                    in_files.push(InFile::Simple(archive.from.clone()));
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
                PluginAction::Archive { archive } => {
                    out_files.push(archive.to.clone());
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
        action: &'a PluginAction,
        in_files: &'a Vec<String>,
        out_files: &'a Vec<String>,
        _working_dir: &'a String,
        _maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            let mut has_error = false;
            let from = &in_files[0];
            let to = &out_files[0];

            let to_path = PathBuf::from(to);
            if !std::fs::exists(&to_path.parent().unwrap()).unwrap() {
                fs::create_dir_all(&to_path.parent().unwrap()).unwrap();
            }

            let action_id = String::from("ARCHIVE") + from + to;
            let from_path = PathBuf::from(from);
            if !from_path.exists() {
                Logger::set_action(
                    target_id.to_string(),
                    step_id.to_string(),
                    LogAction {
                        id: action_id.clone(),
                        status: ProgressStatus::Failed,
                        description: format!("archive failed: Path doesn't exist {}", from),
                        progress: ActionProgressType::Spinner,
                        percent: None,
                    },
                );

                return true;
            }
            if !from_path.is_dir() {
                Logger::set_action(
                    target_id.to_string(),
                    step_id.to_string(),
                    LogAction {
                        id: action_id.clone(),
                        status: ProgressStatus::Failed,
                        description: format!("archive failed: Path is not a directory {}", from),
                        progress: ActionProgressType::Spinner,
                        percent: None,
                    },
                );

                return true;
            }

            let mut exclude_paths = Vec::new();
            match action {
                PluginAction::Archive { archive } => {
                    exclude_paths = archive.exclude.clone().unwrap_or(Vec::new());
                }
                _ => {}
            }

            match archive(target_id, step_id, from, to, &exclude_paths) {
                Ok(()) => {
                    has_error = false;
                }
                Err(_) => {
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
