use config_macros::ActionDoc;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    future::Future,
    io::BufReader,
    path::Path,
    pin::Pin,
    sync::{Arc, Mutex},
};
use zip::ZipArchive;

use crate::{
    console::{
        logger::{ActionProgressType, LogAction, Logger, ProgressStatus},
    },
    emake::{InFile, PluginAction},
};
use flate2::read::GzDecoder;
use rayon::prelude::*;
use std::io;
use tar::Archive as TarArchive;
use xz2::read::XzDecoder;

use super::Action;
pub static ID: &str = "extract";

#[derive(ActionDoc, Debug, Clone, Serialize, Deserialize)]
#[action_doc(
    id = "extract",
    short_desc = "Extract archive",
    description = "Support archive are: zip, tar.gz and tar.xz",
    example = "
{% raw %}
targets:
    extraction_example:
        steps:
            - description: Retrieve and extract archive from url
              extract: 
                from: 
                    - https://github.com/pchakour/easymake/archive/refs/heads/main.zip
                to:
                    - \"{{ EMAKE_OUT_DIR }}\"
                out_files:
                    - \"{{ glob('${EMAKE_OUT_DIR}/main/**/*') }}\"
{% endraw %}
"
)]
pub struct ExtractAction {
    #[action_prop(description = "Archive to extract, can be an url", required = true)]
    pub from: InFile,
    #[action_prop(description = "Folder in which extract the archive", required = true)]
    pub to: String,
    #[action_prop(description = "To register extracted file in the cache. Allow to execute again the extraction if a file from out_files change", required = false, default = "None")]
    pub out_files: Option<Vec<String>>,
}

pub struct Extract;

#[cfg(unix)]
fn set_unix_permissions(file: &zip::read::ZipFile, outpath: &Path) {
    use std::os::unix::fs::PermissionsExt;

    if let Some(mode) = file.unix_mode() {
        std::fs::set_permissions(outpath, std::fs::Permissions::from_mode(mode)).unwrap();
    }
}

fn extract_zip_multithreaded(file: &std::fs::File, output_dir: &str) -> io::Result<()> {
    let file_buffer = BufReader::new(file);
    let zip = ZipArchive::new(file_buffer)?;
    let zip = Arc::new(Mutex::new(zip));

    // Collect all indices first
    let indices: Vec<usize> = (0..zip.lock().unwrap().len()).collect();

    // Parallel extraction
    indices.par_iter().try_for_each(|&i| {
        let mut zip = zip.lock().unwrap();
        let mut file_in_zip = zip
            .by_index(i)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let outpath = Path::new(output_dir).join(file_in_zip.name());

        if file_in_zip.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file_in_zip, &mut outfile)?;
        }

        #[cfg(unix)]
        set_unix_permissions(&file_in_zip, &outpath);

        Ok(())
    })
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
        let _ = extract_zip_multithreaded(&file, output_dir);
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
                    description: format!(
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
                description: format!("Extraction of file {} is done", archive_path),
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

impl Action for Extract {
    fn insert_in_files<'a>(
        &'a self,
        action: &'a PluginAction,
        in_files: &'a mut Vec<InFile>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::Extract { extract } => {
                    in_files.push(extract.from.clone());
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
                    out_files.push(extract.to.clone());
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
            let from = &in_files[0];
            let to = &out_files[0];

            if !fs::exists(to).unwrap() {
                fs::create_dir_all(to).unwrap();
            }

            return match extract(target_id, step_id, from, to) {
                Ok(()) => false,
                Err(_) => true
            }
        })
    }
    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
}
