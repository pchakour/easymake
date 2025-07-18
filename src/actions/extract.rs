use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    future::Future,
    io::BufReader,
    path::{Path},
    pin::Pin,
};
use zip::ZipArchive;

use crate::{
    console::{
        log,
        logger::{ActionProgressType, LogAction, Logger, ProgressStatus},
    },
    emake::{self, InFile, PluginAction},
};
use flate2::read::GzDecoder;
use tar::Archive as TarArchive;
use xz2::read::XzDecoder;

use super::Action;
pub static ID: &str = "extract";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractAction {
    from: String,
    to: String,
    out_files: Option<Vec<String>>,
}

pub struct Extract;

#[cfg(unix)]
fn set_unix_permissions(file: &zip::read::ZipFile, outpath: &Path) {
    use std::os::unix::fs::PermissionsExt;

    if let Some(mode) = file.unix_mode() {
        std::fs::set_permissions(outpath, std::fs::Permissions::from_mode(mode)).unwrap();
    }
}

fn extract(
    target_id: &str,
    step_id: &str,
    archive_path: &str,
    output_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(archive_path);
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let file = std::fs::File::open(path)?;
    let file_buffer = BufReader::new(&file);
    let action_id = String::from("EXTRACT") + archive_path;

    Logger::set_action(
        target_id.to_string(),
        step_id.to_string(),
        LogAction {
            id: action_id.clone(), 
            status: ProgressStatus::Progress,
            description: String::from("Starting files extraction"),
            progress: ActionProgressType::Spinner,
            percent: None,
        },
    );

    if extension == "zip" {
        let mut zip = ZipArchive::new(file_buffer)?;
        for i in 0..zip.len() {
            let mut file_in_zip = zip.by_index(i)?;
            let outpath = Path::new(output_dir).join(file_in_zip.name());

            if file_in_zip.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    std::fs::create_dir_all(p)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file_in_zip, &mut outfile)?;
            }

            #[cfg(unix)]
            set_unix_permissions(&file_in_zip, &outpath);
        }
        Ok(())
    } else if archive_path.ends_with(".tar.gz") {
        // Extract
        let tar = GzDecoder::new(file_buffer);
        let mut archive = TarArchive::new(tar);

        for entry in archive.entries()? {
            let mut entry = entry?;
            Logger::set_action(
                target_id.to_string(),
                step_id.to_string(),
                LogAction {
                    id: action_id.clone(),
                    status: ProgressStatus::Progress,
                    description:format!(
                        "Extracting file {}",
                        entry.header().path().unwrap().to_string_lossy().to_string()
                    ),
                    progress: ActionProgressType::Spinner,
                    percent: None,
                },
            );
            entry.unpack_in(output_dir)?;
        }

        Logger::set_action(
            target_id.to_string(),
            step_id.to_string(),
            LogAction {
                id: action_id.clone(),
                status: ProgressStatus::Done,
                description:format!(
                    "Extraction of file {} is done",
                    archive_path
                ),
                progress: ActionProgressType::Spinner,
                percent: None,
            },
        );

        Ok(())
    } else if archive_path.ends_with(".tar.xz") {
        let tar = XzDecoder::new(file);
        let mut archive = TarArchive::new(tar);
        for entry in archive.entries()? {
            let mut entry = entry?;
            entry.unpack_in(output_dir)?;
        }

        Ok(())
    } else {
        Err(format!("Unsupported archive format: {}", archive_path).into())
    }
}

fn compile<'a>(
    cwd: &'a str,
    emakefile_cwd: &'a str,
    action: &'a PluginAction,
    in_files: &'a Vec<String>,
    out_files: &'a Vec<String>,
    maybe_replacements: Option<&'a HashMap<String, String>>,
) -> Option<(String, String)> {
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
        PluginAction::Extract { extract } => {
            let from_compiled = emake::compiler::compile(
                cwd,
                &extract.from,
                &emakefile_cwd.to_string(),
                Some(&replacements),
            );
            let to_compiled = emake::compiler::compile(
                cwd,
                &extract.to,
                &emakefile_cwd.to_string(),
                Some(&replacements),
            );

            return Some((from_compiled, to_compiled));
        }
        _ => {}
    }

    None
}

impl Action for Extract {
    fn insert_in_files<'a>(
        &'a self,
        action: &'a PluginAction,
        in_files: &'a mut Vec<InFile>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::Extract { extract } => {
                    in_files.push(InFile::Simple(extract.from.clone()));
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
                PluginAction::Extract { extract } => {
                    if let Some(plugin_out_files) = &extract.out_files {
                        for out_file in plugin_out_files {
                            out_files.push(out_file.clone());
                        }
                    }
                }
                _ => {}
            }
        })
    }

    fn run<'a>(
        &'a self,
        cwd: &'a str,
        target_id: &'a str,
        step_id: &'a str,
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
            let some_files = compile(
                cwd,
                emakefile_cwd,
                action,
                in_files,
                out_files,
                maybe_replacements,
            );

            if let Some((_from, to)) = some_files {
                match extract(target_id, step_id, &in_files[0], &to) {
                    Ok(()) => {
                        has_error = false;
                    }
                    Err(e) => {
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
