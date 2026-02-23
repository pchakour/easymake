use config_macros::ActionDoc;
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
    console::log,
    emake::{InFile, PluginAction},
};
use flate2::{write::GzEncoder, Compression};
use globset::{Glob, GlobSet, GlobSetBuilder};
use xz2::write::XzEncoder;
use zstd::stream::Encoder as ZstdEncoder;

use super::Action;
pub static ID: &str = "archive";

#[derive(ActionDoc, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[action_doc(
    id = "archive",
    short_desc = "Compress your files as an archive",
    example = "
targets:
  pre_archive:
    steps:
      - description: Creating file to archive
        shell:
          out_files: [\"{{ EMAKE_WORKING_DIR }}/file_to_archive.txt\"]
          cmd: echo 'Hello World !' > {{ out_files }}
  archive:
    deps:
        - pre_archive
    steps:
        - description: 'Example files compression'
          archive:
            from:
                - \"{{ EMAKE_WORKING_DIR }}/file_to_archive.txt\"
            to: \"{{ EMAKE_OUT_DIR }}/archive.zip\"
"
)]
pub struct ArchiveAction {
    #[action_prop(description = "Files to compress", required = true)]
    pub from: Vec<InFile>,
    #[action_prop(description = "Destination", required = true)]
    pub to: String,
    #[action_prop(description = "Exclude a list of file", required = false)]
    pub exclude: Option<Vec<String>>,
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
    _target_id: &str,
    step_id: &str,
    from_paths: &Vec<String>,
    archive_path: &str,
    exclude_paths: &Vec<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new(archive_path);
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    log::debug!("[{}] {}", step_id, "Starting files compression");
    // Build globset once (works with empty exclude_paths)
    let globset = build_exclude_globset(exclude_paths).unwrap();

    if extension == "zip" {
        let file = std::fs::File::create(path)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for from in from_paths {
            let from_path = Path::new(from);
            if !from_path.exists() {
                return Err(format!("Archive: path doesn't exist {}", from).into());
            }

            if from_path.is_file() {
                // Skip if excluded
                if globset.is_match(from_path) {
                    continue;
                }

                let name = from_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or("Invalid filename")?
                    .replace('\\', "/");

                log::action_debug!(step_id, ID, "Compressing file: {}", name);
                zip.start_file(&name, options)?;
                let mut f = std::fs::File::open(from_path)?;
                std::io::copy(&mut f, &mut zip)?;
            } else if from_path.is_dir() {
                // Use the helper to get entries already filtered by excludes
                let entries: Vec<_> = walk_with_excludes(from_path, exclude_paths).unwrap();
                let total = entries.len();
                let mut current = 0;

                for entry in entries {
                    let entry_path = entry.path();
                    // compute name inside archive relative to the base 'from' directory
                    let relative = entry_path.strip_prefix(from_path)?;
                    let name = relative
                        .to_str()
                        .ok_or("Failed to convert path to string")?
                        .replace('\\', "/");

                    current += 1;
                    // optional progress percent
                    let percent = if total > 0 {
                        Some(current * 100 / total)
                    } else {
                        None
                    };

                    log::action_debug!(step_id, ID, "Percent: {}% / Compressing: {}", percent.unwrap_or(0), name);

                    if entry.file_type().is_file() {
                        zip.start_file(&name, options)?;
                        let mut f = std::fs::File::open(entry_path)?;
                        std::io::copy(&mut f, &mut zip)?;
                    } else if entry.file_type().is_dir() {
                        let dir_name = format!("{}/", name);
                        zip.add_directory(dir_name, options)?;
                    }
                }
            }
        }

        zip.finish()?;
    } else if archive_path.ends_with(".tar.gz") {
        let tar_gz = std::fs::File::create(archive_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);

        for from in from_paths {
            let from_path = Path::new(from);
            if !from_path.exists() {
                continue;
            }

            if from_path.is_file() {
                if globset.is_match(from_path) {
                    continue;
                }
                let mut file = std::fs::File::open(from_path)?;
                let name = from_path.file_name().ok_or("Invalid filename")?;
                tar.append_file(name, &mut file)?;
                continue;
            }

            let entries: Vec<_> = walk_with_excludes(from_path, exclude_paths).unwrap();
            let total = entries.len();
            let mut current = 0;
            for entry in entries {
                let entry_path = entry.path();
                let mut relative_path = entry_path.strip_prefix(from_path)?;
                if relative_path.as_os_str().is_empty() {
                    relative_path = Path::new(".");
                }
                let name = relative_path
                    .to_str()
                    .ok_or("Failed to convert relative path to str")?
                    .replace('\\', "/");

                current += 1;
                let percent = if total > 0 { Some(current * 100 / total) } else { None };
                log::action_debug!(step_id, ID, "Percent: {}% / Compressing: {}", percent.unwrap_or(0), name);

                if entry.file_type().is_dir() {
                    tar.append_dir(relative_path, entry_path)?;
                } else if entry.file_type().is_file() {
                    let mut file = std::fs::File::open(entry_path)?;
                    tar.append_file(relative_path, &mut file)?;
                }
            }
        }

        tar.finish()?;
    } else if archive_path.ends_with(".tar.zst") {
        let tar_file = std::fs::File::create(archive_path)?;
        let mut enc = ZstdEncoder::new(tar_file, 3)?;
        enc.multithread(num_cpus::get() as u32)?;
        let mut tar = tar::Builder::new(enc);

        for from in from_paths {
            let from_path = Path::new(from);
            if !from_path.exists() {
                continue;
            }
            if from_path.is_file() {
                if globset.is_match(from_path) {
                    continue;
                }
                let mut file = std::fs::File::open(from_path)?;
                let name = from_path.file_name().ok_or("Invalid filename")?;
                tar.append_file(name, &mut file)?;
                continue;
            }

            let entries: Vec<_> = walk_with_excludes(from_path, exclude_paths).unwrap();
            let total = entries.len();
            let mut current = 0;

            for entry in entries {
                let entry_path = entry.path();
                let mut relative_path = entry_path.strip_prefix(from_path)?;
                if relative_path.as_os_str().is_empty() {
                    relative_path = Path::new(".");
                }

                let name = relative_path
                    .to_str()
                    .ok_or("Failed to convert relative path to str")?
                    .replace('\\', "/");

                current += 1;
                let percent = if total > 0 { Some(current * 100 / total) } else { None };

                log::action_debug!(step_id, ID, "Percent: {}% / Compressing: {}", percent.unwrap_or(0), name);

                if entry.file_type().is_dir() {
                    tar.append_dir(relative_path, entry_path)?;
                } else if entry.file_type().is_file() {
                    let mut file = std::fs::File::open(entry_path)?;
                    tar.append_file(relative_path, &mut file)?;
                }
            }
        }

        tar.finish()?;
        let enc = tar.into_inner()?;
        enc.finish()?;
    } else if archive_path.ends_with(".tar.xz") {
        let tar_xz = std::fs::File::create(archive_path)?;
        let enc = XzEncoder::new(tar_xz, 6);
        let mut tar = tar::Builder::new(enc);

        for from in from_paths {
            let from_path = Path::new(from);
            if !from_path.exists() {
                continue;
            }
            if from_path.is_file() {
                if globset.is_match(from_path) {
                    continue;
                }
                let mut file = std::fs::File::open(from_path)?;
                let name = from_path.file_name().ok_or("Invalid filename")?;
                tar.append_file(name, &mut file)?;
                continue;
            }

            let entries: Vec<_> = walk_with_excludes(from_path, exclude_paths).unwrap();
            let total = entries.len();
            let mut current = 0;

            for entry in entries {
                let entry_path = entry.path();
                let mut relative_path = entry_path.strip_prefix(from_path)?;
                if relative_path.as_os_str().is_empty() {
                    relative_path = Path::new(".");
                }

                let name = relative_path
                    .to_str()
                    .ok_or("Failed to convert relative path to str")?
                    .replace('\\', "/");

                current += 1;
                let percent = if total > 0 {
                    Some(current * 100 / total)
                } else {
                    None
                };

                log::action_debug!(step_id, ID, "[{}] Percent: {}% / Compressing: {}", step_id, percent.unwrap_or(0), name);

                if entry.file_type().is_dir() {
                    tar.append_dir(relative_path, entry_path)?;
                } else if entry.file_type().is_file() {
                    let mut file = std::fs::File::open(entry_path)?;
                    tar.append_file(relative_path, &mut file)?;
                }
            }
        }

        tar.finish()?;
    } else {
        return Err(format!("Unsupported archive format: {}", archive_path).into());
    }

    log::action_info!(step_id, ID, "Archive {} completed", archive_path);

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
                    for path in &archive.from {
                        in_files.push(path.clone());
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
                PluginAction::Archive { archive } => {
                    out_files.push(archive.to.clone());
                }
                _ => {}
            }
        })
    }

    fn run<'a>(
        &'a self,
        target_id: &'a str,
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
            let to = &out_files[0];
            let to_path = PathBuf::from(to);

            if !std::fs::exists(&to_path.parent().unwrap()).unwrap() {
                fs::create_dir_all(&to_path.parent().unwrap()).unwrap();
            }

            let mut exclude_paths = Vec::new();
            if let PluginAction::Archive { archive } = action {
                exclude_paths = archive.exclude.clone().unwrap_or_default();
            }

            let target_id_clone = target_id.to_string();
            let step_id_clone = step_id.to_string();
            let in_files_clone = in_files.clone();
            let to_clone = to.clone();
            let exclude_paths_clone = exclude_paths.clone();

            let spawn_result = tokio::task::spawn_blocking(move || {
                archive(&target_id_clone, &step_id_clone, &in_files_clone, &to_clone, &exclude_paths_clone)
            }).await.unwrap();

            if spawn_result.is_err() {
                let error_message = spawn_result.err().unwrap().to_string();
                let error: Result<(), Box<dyn std::error::Error>> = Err(error_message.into());
                return error;
            }

            Ok(())
        })
    }

    fn get_checksum(&self) -> Option<String> {
        None
    }

    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
}
