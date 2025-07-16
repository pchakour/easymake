use crate::actions::{
    compute_action_footprint, get_registered_action_footprint, register_action_footprint,
};
use crate::cache::get_out_dir_path;
use crate::console::log;
use crate::console::logger::{
    ActionProgressType, LogAction, LogStep, LogTarget, Logger, ProgressStatus,
};
use crate::emake::loader::{extract_info_from_path, Target, TargetType};
use crate::emake::Step;
use crate::graph::generator::{get_absolute_target_path, to_emakefile_path};
use crate::utils::get_absolute_file_path;
use crate::{
    cache, credentials, emake, get_mutex_for_id, graph, utils, ACTIONS_STORE,
    CACHE_IN_FILE_TO_UPDATE, CACHE_OUT_FILE_TO_UPDATE, CREDENTIALS_STORE, GLOBAL_SEMAPHORE,
    MULTI_PROGRESS,
};
use console::style;
use futures::future::join_all;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressFinish, ProgressStyle};
use reqwest::Client;
use std::borrow::Cow;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process;
use std::process::ExitStatus;
use std::time::Duration;
use std::{collections::HashMap, future::Future};
use tokio::task::JoinHandle;
use url::Url;

static SPINNER_TIME: u64 = 200;

async fn download_file(
    target_id: &str,
    step_id: &str,
    url: &str,
    output_path: &str,
    cwd: &str,
    emakefile_cwd: &str,
    maybe_credentials_key: &Option<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut maybe_credentials: Option<credentials::PlainCredentials> = None;
    if let Some(credentials_key) = maybe_credentials_key {
        let result_credentials_config = emake::loader::get_target_on_path(
            cwd,
            credentials_key,
            emakefile_cwd,
            Some(TargetType::Credentials),
        );

        match result_credentials_config {
            Ok(credentials_config) => match credentials_config {
                Target::CredentialEntry(credentials_config) => {
                    if !credentials_config.contains_key("type") {
                        log::error!("The credential {} must contains a type", credentials_key);
                        process::exit(1);
                    }
                    let credentials_type =
                        String::from(credentials_config.get("type").unwrap().as_str().unwrap());
                    let maybe_credentials_plugin = CREDENTIALS_STORE.get(&credentials_type);
                    if let Some(credentials_plugin) = maybe_credentials_plugin {
                        maybe_credentials =
                            Some(credentials_plugin.extract(cwd, &credentials_config));
                    } else {
                        log::error!("The credential type {} does not exist", credentials_type);
                        process::exit(1);
                    }
                }
                _ => {
                    log::error!("The specified path {} is not a credential", credentials_key);
                    std::process::exit(1);
                }
            },
            Err(error) => {
                log::error!("{}", error);
                std::process::exit(1);
            }
        }
    }

    // Validate URL
    let parsed_url: Url = Url::parse(url)?;
    if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
        return Err("URL must be HTTP or HTTPS".into());
    }

    // Send GET request
    let client = Client::new();
    let mut request = client.get(url);

    if let Some(credentials) = maybe_credentials {
        request = request.basic_auth(credentials.username, credentials.password);
    }

    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(format!("Failed to download: HTTP {}", response.status()).into());
    }

    let total_size = response
        .content_length()
        .ok_or("Failed to get content length")?;

    let description = format!("Downloading file {}", url);
    let action_id = String::from("DOWNLOADING_FILE_") + url;

    Logger::set_action(
        target_id.to_string(),
        step_id.to_string(),
        LogAction {
            id: action_id.clone(),
            description: description.clone(),
            status: ProgressStatus::Progress,
            progress: ActionProgressType::Bar,
            percent: Some(0),
        },
    );

    let mut dest = BufWriter::new(File::create(output_path)?);
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        dest.write_all(&chunk)?;
        downloaded += chunk.len() as u64;

        Logger::set_action(
            target_id.to_string(),
            step_id.to_string(),
            LogAction {
                id: action_id.clone(),
                description: description.clone(),
                status: ProgressStatus::Progress,
                progress: ActionProgressType::Bar,
                percent: Some(if total_size > 0 {
                    ((downloaded * 100) / total_size) as usize
                } else {
                    0
                }),
            },
        );
    }

    Logger::set_action(
        target_id.to_string(),
        step_id.to_string(),
        LogAction {
            id: action_id.clone(),
            description: format!("File {} downloaded", url),
            status: ProgressStatus::Done,
            progress: ActionProgressType::Bar,
            percent: None,
        },
    );

    Ok(())
}

fn get_filename_from_url(url: &str) -> Option<String> {
    // Parse the URL
    if let Ok(parsed_url) = Url::parse(url) {
        // Extract the last segment of the path (filename)
        if let Some(segments) = parsed_url.path_segments() {
            return segments.last().map(|s| s.to_string());
        }
    }
    None
}

async fn get_real_in_files<'a>(
    target_id: &'a str,
    step_id: &'a str,
    step: &'a Step,
    cwd: &'a str,
    emakefile_current_path: &'a str,
) -> Vec<String> {
    let plugin = ACTIONS_STORE.get(&step.plugin).expect(&format!(
        "Can't execute step \"{}\", we are not able to find the plugin used in this step",
        step.description.clone().unwrap_or(step_id.to_string())
    ));

    let mut in_files;
    if step.in_files.is_none() {
        in_files = Vec::new()
    } else {
        in_files = step.in_files.clone().unwrap()
    };

    plugin.insert_in_files(&step.plugin, &mut in_files).await;

    let mut real_in_files = Vec::new();
    let working_dir = cache::get_working_dir_path(cwd);
    let out_dir = cache::get_out_dir_path(cwd);
    let default_replacements = HashMap::from([
        (String::from("EMAKE_WORKING_DIR"), working_dir.to_owned()),
        (String::from("EMAKE_CWD_DIR"), cwd.to_owned()),
        (String::from("EMAKE_OUT_DIR"), out_dir.to_owned()),
    ]);

    let mut download_futures = Vec::new();
    let mut downloadable_files_indices = HashMap::new();

    // Get in files modification date
    for in_file in &in_files {
        let file_path;
        let mut file_credentials = None;

        match &in_file {
            emake::InFile::Simple(src) => file_path = src,
            emake::InFile::Detailed(in_file_entry) => {
                file_path = &in_file_entry.file;
                file_credentials = in_file_entry.clone().credentials;
            }
        }

        let compiled_in_file_string = emake::compiler::compile(
            cwd,
            &file_path,
            emakefile_current_path,
            Some(&default_replacements),
        );
        let mut files = Vec::from([compiled_in_file_string.clone()]);

        let parsed_compiled_files_result: Result<Vec<String>, _> =
            serde_yml::from_str(&compiled_in_file_string);
        match parsed_compiled_files_result {
            Ok(parsed_compiled_files) => files = parsed_compiled_files,
            Err(_) => (),
        }

        for file in &files {
            if graph::common::is_downloadable_file(&file) {
                let filename = get_filename_from_url(&file).unwrap();
                let mut output = PathBuf::from(cache::get_working_dir_path(cwd));
                output.push(&filename);
                let output_string = output.to_str().unwrap().to_string();
                downloadable_files_indices.insert(file.clone(), output_string.clone());
                if cache::has_file_changed(cwd, &output_string, step_id, &false).await {
                    let file_clone = file.clone(); // required if file is &String
                    let target_id_clone = String::from(target_id);
                    let step_id_clone = String::from(step_id);
                    let cwd_clone = cwd.to_string();
                    let emakefile_current_path = emakefile_current_path.to_string();
                    let file_credentials = file_credentials.clone();
                    download_futures.push(tokio::spawn(async move {
                        let _s = GLOBAL_SEMAPHORE.acquire().await;
                        download_file(
                            &target_id_clone,
                            &step_id_clone,
                            &file_clone,
                            &output_string,
                            &cwd_clone,
                            &emakefile_current_path,
                            &file_credentials,
                        )
                        .await
                    }));
                }
            }
        }
        real_in_files.extend(files);
    }

    // Run all downloads in parallel
    let download_results = join_all(download_futures).await;

    // Check for errors
    for result in download_results {
        if let Err(err) = result {
            log::error!("Download task panicked: {:?}", err);
        } else if let Err(err) = result.unwrap() {
            log::error!("Download failed: {:?}", err);
        }
    }

    // Replace URLs with local file paths
    for (initial_name, replaced_name) in downloadable_files_indices {
        real_in_files = real_in_files
            .iter()
            .map(|file| {
                if *file == initial_name {
                    return replaced_name.clone();
                }

                file.clone()
            })
            .collect();
    }

    real_in_files
}

async fn get_real_out_files<'a>(
    step_id: &'a str,
    step: &'a Step,
    cwd: &'a str,
    emakefile_current_path: &'a str,
) -> Vec<String> {
    let plugin = ACTIONS_STORE.get(&step.plugin).expect(&format!(
        "Can't execute step \"{}\", we are not able to find the plugin used in this step",
        step.description.clone().unwrap_or(step_id.to_string())
    ));

    let mut out_files;
    let mut real_out_files = Vec::new();
    let working_dir = cache::get_working_dir_path(cwd);
    let out_dir = cache::get_out_dir_path(cwd);
    let default_replacements = HashMap::from([
        (String::from("EMAKE_WORKING_DIR"), working_dir.to_owned()),
        (String::from("EMAKE_CWD_DIR"), cwd.to_owned()),
        (String::from("EMAKE_OUT_DIR"), out_dir.to_owned()),
    ]);

    if step.out_files.is_none() {
        out_files = Vec::new()
    } else {
        out_files = step.out_files.clone().unwrap()
    };

    plugin.insert_out_files(&step.plugin, &mut out_files).await;

    for out_file in &out_files {
        let compiled_out_file_string = emake::compiler::compile(
            cwd,
            out_file,
            emakefile_current_path,
            Some(&default_replacements),
        );
        let mut files = Vec::from([compiled_out_file_string.clone()]);

        let parsed_compiled_files_result: Result<Vec<String>, _> =
            serde_yml::from_str(&compiled_out_file_string);
        match parsed_compiled_files_result {
            Ok(parsed_compiled_files) => files = parsed_compiled_files,
            Err(_) => (),
        }

        real_out_files.extend(files);
    }

    real_out_files
}

async fn run_step<'a>(
    target_id: &'a str,
    step_id: &'a str,
    step: &'a Step,
    cwd: &'a str,
    emakefile_current_path: &'a str,
    force_out_files: Option<Vec<String>>,
) {
    let plugin = ACTIONS_STORE.get(&step.plugin).expect(&format!(
        "Can't execute step \"{}\", we are not able to find the plugin used in this step",
        step.description.clone().unwrap_or(step_id.to_string())
    ));
    let step_description = step.description.clone().unwrap_or(step_id.to_string());
    Logger::set_step(
        target_id.to_string(),
        LogStep {
            id: step_id.to_string(),
            description: Some(step_description),
            actions: Vec::new(),
            status: ProgressStatus::Progress,
        },
    );

    let working_dir = cache::get_working_dir_path(cwd);
    let out_dir = cache::get_out_dir_path(cwd);
    let default_replacements = HashMap::from([
        (String::from("EMAKE_WORKING_DIR"), working_dir.to_owned()),
        (String::from("EMAKE_CWD_DIR"), cwd.to_owned()),
        (String::from("EMAKE_OUT_DIR"), out_dir.to_owned()),
    ]);
    let real_in_files = get_real_in_files(target_id, step_id, step, cwd, emakefile_current_path).await;
    let plugin_out_files = get_real_out_files(step_id, step, cwd, emakefile_current_path).await;
    let mut real_out_files = plugin_out_files.clone();
    if force_out_files.is_some() {
        real_out_files = force_out_files.unwrap();
    }

    let mut need_to_run_action = real_in_files.len() == 0 && real_out_files.len() == 0;

    for file in &real_in_files {
        let file_changed = cache::has_file_changed(cwd, file, step_id, &true).await;
        if file_changed {
            need_to_run_action = true;
        }
    }

    for file in &real_out_files {
        let file_changed = cache::has_file_changed(cwd, file, step_id, &false).await;
        if file_changed {
            need_to_run_action = true;
        }
    }

    if let Some(checksum_command) = &step.checksum {
        let mut maybe_checksum: Option<String> = None;
        let (status, stdout, stderr) = utils::run_command(
            checksum_command,
            Path::new(cwd),
            Path::new(emakefile_current_path),
            Some(&default_replacements),
        );

        if ExitStatus::success(&status) {
            maybe_checksum = Some(stdout);
        } else {
            log::warning!("Error when computing checksum of action {step_id}: {stderr}");
        }

        if let Some(checksum) = maybe_checksum {
            if let Some(current_action_checksum) =
                cache::get_cache_action_checksum(step_id, cwd).await
            {
                if checksum.trim().to_string() != current_action_checksum {
                    log::info!(
                        "Checksum change from {} to {}",
                        current_action_checksum,
                        checksum.trim().to_string()
                    );
                    need_to_run_action = true;
                }
            }
        }
    }

    // Compute action footprint
    let action_footprint = compute_action_footprint(&step.plugin);
    let register_footprint = get_registered_action_footprint(&step_id, cwd).await;
    if register_footprint.is_none() || action_footprint != register_footprint.unwrap() {
        need_to_run_action = true;
    }

    if need_to_run_action {
        let has_error = plugin
            .run(
                cwd,
                target_id,
                step_id,
                emakefile_current_path,
                false,
                &step.plugin,
                &real_in_files,
                &plugin_out_files,
                &working_dir,
                Some(&default_replacements),
            )
            .await;

        if !has_error {
            // Register footprint
            register_action_footprint(&step_id, &action_footprint, cwd).await;

            // Register files cache
            for file in &real_in_files {
                let file_absolute_path =
                    String::from(get_absolute_file_path(cwd, file).to_str().unwrap());
                CACHE_IN_FILE_TO_UPDATE.insert((file_absolute_path, String::from(step_id)));
            }

            for file in &real_out_files {
                let file_absolute_path =
                    String::from(get_absolute_file_path(cwd, file).to_str().unwrap());
                CACHE_OUT_FILE_TO_UPDATE.insert((file_absolute_path, String::from(step_id)));
            }

            // Compute checksum
            if let Some(checksum_command) = &step.checksum {
                let mut maybe_checksum: Option<String> = None;
                let (status, stdout, stderr) = utils::run_command(
                    checksum_command,
                    Path::new(cwd),
                    Path::new(emakefile_current_path),
                    Some(&default_replacements),
                );

                if ExitStatus::success(&status) {
                    maybe_checksum = Some(stdout);
                } else {
                    log::warning!("Error when computing checksum of action {step_id}: {stderr}");
                }

                if let Some(checksum) = maybe_checksum {
                    cache::write_cache_action_checksum(step_id, &checksum.trim().to_string(), cwd)
                        .await
                }
            }

            cache::write_in_cache(cwd).await; // Only usefull if we call several time a target
        }
    } else {
        log::info!(
            "{}No need to run action {} because no input/output files changed",
            log::INDENT,
            step_id
        );
    }
}

pub fn run_target<'a>(
    target_absolute_path: String,
    cwd: String,
) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        let emakefile_path = to_emakefile_path(&target_absolute_path, &cwd);
        let emakefile = emake::loader::load_file(&emakefile_path.to_string_lossy().to_string());
        let target_info = extract_info_from_path(
            &target_absolute_path,
            &cwd,
            &emakefile_path.to_string_lossy().to_string(),
        );

        let target = emakefile.targets.get(&target_info.target_name).unwrap();
        if let Some(deps) = &target.deps {
            let mut dependencies_tasks: Vec<JoinHandle<()>> = Vec::new();

            for dependency in deps {
                let dependency_target_path =
                    get_absolute_target_path(dependency, &emakefile.path.clone().unwrap(), &cwd);

                let dependency_target_path_clone = dependency_target_path.clone();
                let cwd_clone = cwd.clone();

                let handle = tokio::spawn(async move {
                    let _s = GLOBAL_SEMAPHORE.acquire().await;
                    run_target(dependency_target_path_clone, cwd_clone).await;
                });

                dependencies_tasks.push(handle);
            }

            // Await all dependency tasks
            futures::future::join_all(dependencies_tasks).await;
        }
    
        Logger::set_target(LogTarget {
            id: target_absolute_path.clone(),
            description: None,
            steps: Vec::new(),
            status: ProgressStatus::Progress,
        });

        if let Some(steps) = &target.steps {
            let mut steps_tasks: Vec<JoinHandle<()>> = Vec::new();

            for (step_index, step) in steps.iter().enumerate() {
                let step_index_string = format!("{}", step_index);
                let step_id = target_absolute_path.clone() + "/" + step_index_string.as_str();
                let cwd_clone = cwd.clone();
                let step_id_clone: String = step_id.clone();
                let target_id_clone = target_absolute_path.clone();
                let step_clone = step.clone();
                let emakefile_path_str = emakefile_path.to_string_lossy().to_string();
                let emakefile_path_str_clone = emakefile_path_str.clone();

                if target.parallel.unwrap_or(false) {
                    let fut = async move {
                        let _s = GLOBAL_SEMAPHORE.acquire().await;
                        let m = get_mutex_for_id(&step_id_clone).await;
                        let _guard = m.lock().await;
                        run_step(
                            &target_id_clone,
                            &step_id_clone,
                            &step_clone,
                            &cwd_clone,
                            &emakefile_path_str,
                            None,
                        )
                        .await;
                    };
                    let handle: JoinHandle<()> = tokio::spawn(fut);
                    steps_tasks.push(handle);
                } else {
                    let mut step_out_files =
                        get_real_out_files(&step_id, &step, &cwd, &emakefile_path_str_clone).await;
                    // Find last out_files
                    let mut current_step_index = step_index.clone() + 1;
                    while current_step_index < steps.len() {
                        let current_step = &steps[current_step_index];
                        let current_step_index_string = format!("{}", current_step_index);
                        let current_step_id =
                            target_absolute_path.clone() + "/" + current_step_index_string.as_str();
                        let current_step_in_files = get_real_in_files(
                            &target_id_clone,
                            &current_step_id,
                            current_step,
                            &cwd,
                            &emakefile_path_str_clone,
                        )
                        .await;
                        for step_out_file in &step_out_files {
                            if current_step_in_files.contains(step_out_file) {
                                step_out_files = get_real_out_files(
                                    &current_step_id,
                                    current_step,
                                    &cwd,
                                    &emakefile_path_str_clone,
                                )
                                .await;
                                break;
                            }
                        }
                        current_step_index += 1;
                    }

                    let m = get_mutex_for_id(&step_id_clone).await;
                    let _guard = m.lock().await;
                    run_step(
                        &target_id_clone,
                        &step_id_clone,
                        &step_clone,
                        &cwd_clone,
                        &emakefile_path_str,
                        Some(step_out_files),
                    )
                    .await;
                }
            }

            futures::future::join_all(steps_tasks).await;
        }
    })
}
