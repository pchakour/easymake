use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    future::Future,
    io::BufReader,
    path::{Path, PathBuf},
    pin::Pin,
};
use zip::ZipArchive;

use crate::{
    console::log,
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
}

pub struct Extract;

fn extract(archive_path: &str, output_dir: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let path = Path::new(archive_path);
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let file = std::fs::File::open(path)?;
    let file = BufReader::new(file);

    let mut extracted_files = Vec::new();

    if extension == "zip" {
        let mut zip = ZipArchive::new(file)?;
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
                extracted_files.push(outpath);
            }
        }
    } else if archive_path.ends_with(".tar.gz") {
        let tar = GzDecoder::new(file);
        let mut archive = TarArchive::new(tar);
        archive.entries()?.filter_map(Result::ok).for_each(|mut entry| {
            if let Ok(path) = entry.path() {
                let full_path = Path::new(output_dir).join(&path);
                let _ = entry.unpack(&full_path);
                extracted_files.push(full_path);
            }
        });
    } else if archive_path.ends_with(".tar.xz") {
        let tar = XzDecoder::new(file);
        let mut archive = TarArchive::new(tar);
        archive.entries()?.filter_map(Result::ok).for_each(|mut entry| {
            if let Ok(path) = entry.path() {
                let full_path = Path::new(output_dir).join(&path);
                let _ = entry.unpack(&full_path);
                extracted_files.push(full_path);
            }
        });
    } else {
        return Err(format!("Unsupported archive format: {}", archive_path).into());
    }

    Ok(extracted_files)
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
            let some_files = compile(
                cwd,
                emakefile_cwd,
                action,
                in_files,
                out_files,
                maybe_replacements,
            );

            if let Some((from, to)) = some_files {
                match extract(&from, &to) {
                    Ok(extracted_files) => {
                        println!("✅ Extraction complete!");
                        println!("{:?}", extracted_files);
                        has_error = false;
                    }
                    Err(e) => {
                        log::error!("❌ Extraction failed: {}", e);
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
