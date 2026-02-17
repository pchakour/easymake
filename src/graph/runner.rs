use crate::actions::{
    compute_action_footprint, get_registered_action_footprint, register_action_footprint,
};
use crate::commands::build::update_progress;
use crate::console::log::{self, StepStatus};
use crate::emake::loader::{extract_info_from_path, Target, TargetType};
use crate::emake::{Credentials, Step};
use crate::graph::generator::{get_absolute_target_path, to_emakefile_path};
use crate::utils::get_absolute_file_path;
use crate::{
    ACTIONS_STORE, CACHE_IN_FILE_TO_UPDATE, CACHE_OUT_FILE_TO_UPDATE, CREDENTIALS_STORE, cache, emake, get_cwd, get_mutex_for_id, graph, secrets, utils
};
use dashmap::DashMap;
use futures::future::join_all;
use futures::StreamExt;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, future::Future};
use tokio::task::JoinHandle;
use tokio::time::interval;
use url::Url;

static RUNNED_TARGETS: Lazy<DashMap<String, Arc<tokio::sync::Mutex<()>>>> = Lazy::new(DashMap::new);

pub fn is_url(s: &str) -> bool {
    Url::parse(s).is_ok()
}

async fn download_file(
    _target_id: &str,
    step_id: &str,
    url: &str,
    output_path: &str,
    emakefile_cwd: &str,
    maybe_credentials: &Option<Credentials>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut maybe_username_secret: Option<secrets::PlainSecret> = None;
    let mut maybe_password_secret: Option<secrets::PlainSecret> = None;
    if let Some(credentials) = maybe_credentials {
        let result_username_secret = emake::loader::get_target_on_path(
            &credentials.username,
            emakefile_cwd,
            Some(TargetType::Secrets),
        );

        let mut maybe_result_password_secret: Option<
            Result<emake::loader::Target, std::string::String>,
        > = None;
        if let Some(credential_password) = &credentials.password {
            maybe_result_password_secret = Some(emake::loader::get_target_on_path(
                &credential_password,
                emakefile_cwd,
                Some(TargetType::Secrets),
            ));
        }

        match result_username_secret {
            Ok(username_secret) => match username_secret {
                Target::SecretEntry(secret_config) => {
                    if !secret_config.contains_key("type") {
                        log::panic!("The secret {} must contains a type", credentials.username);
                    }
                    let secret_type =
                        String::from(secret_config.get("type").unwrap().as_str().unwrap());
                    let maybe_secret_plugin = CREDENTIALS_STORE.get(&secret_type);
                    if let Some(secret_plugin) = maybe_secret_plugin {
                        maybe_username_secret = Some(secret_plugin.extract(&secret_config));
                    } else {
                        log::panic!("The secret type {} does not exist", secret_type);
                    }
                }
                _ => {
                    log::panic!(
                        "The specified path {} is not a secret",
                        credentials.username
                    );
                }
            },
            Err(error) => {
                log::panic!("{}", error);
            }
        }

        if let Some(result_password_secret) = maybe_result_password_secret {
            match result_password_secret {
                Ok(password_secret) => match password_secret {
                    Target::SecretEntry(secret_config) => {
                        if !secret_config.contains_key("type") {
                            log::panic!(
                                "The secret {:?} must contains a type",
                                credentials.password
                            );
                        }
                        let secret_type =
                            String::from(secret_config.get("type").unwrap().as_str().unwrap());
                        let maybe_secret_plugin = CREDENTIALS_STORE.get(&secret_type);
                        if let Some(secret_plugin) = maybe_secret_plugin {
                            maybe_password_secret =
                                Some(secret_plugin.extract(&secret_config));
                        } else {
                            log::panic!("The credential type {} does not exist", secret_type);
                        }
                    }
                    _ => {
                        log::panic!(
                            "The specified path {:?} is not a credential",
                            credentials.password
                        );
                    }
                },
                Err(error) => {
                    log::panic!("{}", error);
                }
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

    if let Some(username_secret) = maybe_username_secret {
        request = request.basic_auth(username_secret, maybe_password_secret);
    }

    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(format!("Failed to download: HTTP {}", response.status()).into());
    }

    let total_size = response
        .content_length()
        .ok_or("Failed to get content length")?;

    let description = format!("Downloading file {}", url);
    let action_id = "download file";
    log::action_info!(step_id, action_id, "{}", description);

    let mut dest = BufWriter::new(File::create(output_path)?);
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        dest.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        let mut percent = 0;
        if total_size > 0 {
            percent = ((downloaded * 100) / total_size) as usize
        }

        log::action_debug!(step_id, action_id, "Percent {}% | {}", percent, description);
    }

    log::action_info!(step_id, action_id, "File {} downloaded", url);
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
    emakefile_current_path: &'a str,
) -> Vec<String> {
    let plugin = ACTIONS_STORE.get(&step.action).expect(&format!(
        "Can't execute step \"{}\", we are not able to find the action used in this step",
        step.description.clone()
    ));

    let mut in_files = Vec::new();

    plugin.insert_in_files(&step.action, &mut in_files).await;

    let mut real_in_files = Vec::new();
    let working_dir = cache::get_working_dir_path();
    let out_dir = cache::get_out_dir_path();
    let cwd = get_cwd().to_string_lossy().to_string();
    let default_replacements = HashMap::from([
        (String::from("EMAKE_WORKING_DIR"), working_dir.to_owned()),
        (String::from("EMAKE_CWD_DIR"), cwd.to_owned()),
        (String::from("EMAKE_OUT_DIR"), out_dir.to_owned()),
    ]);

    let mut download_futures = Vec::new();
    let mut downloadable_files_indices = HashMap::new();
    let mut downloaded_files = Vec::new();
    // Get in files modification date
    for in_file in &in_files {
        let file_path;
        let mut file_credentials = None;

        match &in_file {
            emake::InFile::Simple(src) => file_path = src,
            emake::InFile::Detailed {
                file: detailed_file,
                credentials: detailed_credentials,
            } => {
                file_path = &detailed_file;
                file_credentials = detailed_credentials.clone();
            }
        }

        let compiled_in_file_string = emake::compiler::compile(
            &file_path,
            emakefile_current_path,
            Some(&default_replacements),
            None
        );
        let mut files = Vec::from([compiled_in_file_string.clone()]);

        let parsed_compiled_files_result: Result<Vec<String>, _> =
            serde_json::from_str(&compiled_in_file_string);

        match parsed_compiled_files_result {
            Ok(parsed_compiled_files) => files = parsed_compiled_files,
            Err(_) => (),
        }

        for file in &files {
            if graph::common::is_downloadable_file(&file) {
                let filename = get_filename_from_url(&file).unwrap();
                let mut output = PathBuf::from(cache::get_working_dir_path());
                output.push(&filename);
                let output_string = output.to_str().unwrap().to_string();
                downloadable_files_indices.insert(file.clone(), output_string.clone());
                if cache::has_file_changed(&output_string, step_id, &false) {
                    downloaded_files.push(file.clone());
                    let file_clone = file.clone(); // required if file is &String
                    let target_id_clone = String::from(target_id);
                    let step_id_clone = String::from(step_id);
                    let emakefile_current_path = emakefile_current_path.to_string();
                    let file_credentials = file_credentials.clone();

                    download_futures.push(tokio::spawn(async move {
                        download_file(
                            &target_id_clone,
                            &step_id_clone,
                            &file_clone,
                            &output_string,
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
    for (index, result) in download_results.into_iter().enumerate() {
        let mut maybe_error = None;
        if let Err(err) = result {
            maybe_error = Some(err.to_string());
        } else if let Err(err) = result.unwrap() {
            maybe_error = Some(err.to_string());
        }

        if let Some(error) = maybe_error {
            log::panic!(
                "Error when downloading file {} from step {}: {}",
                downloaded_files[index],
                step_id,
                error
            );
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
    _step_id: &'a str,
    step: &'a Step,
    emakefile_current_path: &'a str,
) -> Vec<String> {
    let plugin = ACTIONS_STORE.get(&step.action).expect(&format!(
        "Can't execute step \"{}\", we are not able to find the plugin used in this step",
        step.description.clone()
    ));

    let mut out_files = Vec::new();
    let mut real_out_files = Vec::new();
    let working_dir = cache::get_working_dir_path();
    let out_dir = cache::get_out_dir_path();
    let default_replacements = HashMap::from([
        (String::from("EMAKE_WORKING_DIR"), working_dir.to_owned()),
        (String::from("EMAKE_CWD_DIR"), get_cwd().to_string_lossy().to_string()),
        (String::from("EMAKE_OUT_DIR"), out_dir.to_owned()),
    ]);

    plugin.insert_out_files(&step.action, &mut out_files).await;

    for out_file in &out_files {
        let compiled_out_file_string = emake::compiler::compile(
            out_file,
            emakefile_current_path,
            Some(&default_replacements),
            None
        );
        let mut files = Vec::from([compiled_out_file_string.clone()]);

        let parsed_compiled_files_result: Result<Vec<String>, _> =
            serde_json::from_str(&compiled_out_file_string);
        match parsed_compiled_files_result {
            Ok(parsed_compiled_files) => files = parsed_compiled_files,
            Err(_) => (),
        }

        real_out_files.extend(files);
    }

    real_out_files
}

async fn run_with_progress<F, T>(
    task: F,
    log_every: Duration,
    step_id: &str,
    step_description: &String,
) -> T
where
    F: Future<Output = T>,
{
    let mut task = Box::pin(task);
    let start = tokio::time::Instant::now();
    let mut ticker = interval(log_every);

    loop {
        tokio::select! {
            result = &mut task => {
                let elapsed = start.elapsed();
                log::step_info!(step_id, StepStatus::Finished, format!("{} after {:?}", step_description, elapsed));
                return result;
            }
            _ = ticker.tick() => {
                let elapsed = start.elapsed();
                log::step_info!(step_id, StepStatus::Running, format!("{} still running  ({:?} elapsed)", step_description, elapsed));
            }
        }
    }
}

async fn run_step<'a>(
    target_id: &'a str,
    step_id: &'a str,
    step: &'a Step,
    emakefile_current_path: &'a str,
    force_out_files: Option<Vec<String>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let plugin = ACTIONS_STORE.get(&step.action).expect(&format!(
        "Can't execute step \"{}\", we are not able to find the plugin used in this step",
        step.description.clone()
    ));
    let step_description = step.description.clone();
    log::step_info!(step_id, StepStatus::Running, step_description);

    let working_dir = cache::get_working_dir_path();
    let out_dir = cache::get_out_dir_path();
    let cwd = get_cwd();
    let default_replacements = HashMap::from([
        (String::from("EMAKE_WORKING_DIR"), working_dir.to_owned()),
        (String::from("EMAKE_CWD_DIR"), cwd.to_string_lossy().to_string()),
        (String::from("EMAKE_OUT_DIR"), out_dir.to_owned()),
    ]);
    let real_in_files =
        get_real_in_files(target_id, step_id, step, emakefile_current_path).await;
    let plugin_out_files = get_real_out_files(step_id, step, emakefile_current_path).await;
    let mut real_out_files = plugin_out_files.clone();
    if force_out_files.is_some() {
        real_out_files = force_out_files.unwrap();
    }
    let checksum_command = plugin.get_checksum();

    let mut need_to_run_action =
        (real_in_files.len() == 0 && real_out_files.len() == 0) || checksum_command.is_some();

    for file in &real_in_files {
        let file_changed = cache::has_file_changed(file, step_id, &true);
        if file_changed {
            need_to_run_action = true;
            break;
        }
    }

    if !need_to_run_action {
        for file in &real_out_files {
            let file_changed = cache::has_file_changed(file, step_id, &false);
            if file_changed {
                need_to_run_action = true;
                break;
            }
        }
    }

    if let Some(checksum_command) = &checksum_command {
        let mut maybe_checksum: Option<String> = None;
        let (status, stdout, stderr) = utils::run_command(
            checksum_command,
            Path::new(emakefile_current_path),
            Some(&default_replacements),
        );

        if ExitStatus::success(&status) {
            maybe_checksum = Some(stdout);
        } else {
            log::warning!("Error when computing checksum of action {step_id}: {stderr}");
        }

        if let Some(checksum) = maybe_checksum {
            if let Some(current_action_checksum) = cache::get_cache_action_checksum(step_id).await {
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
    let action_footprint = compute_action_footprint(&step.action);
    let register_footprint = get_registered_action_footprint(&step_id).await;
    if register_footprint.is_none() || action_footprint != register_footprint.unwrap() {
        need_to_run_action = true;
    }

    if need_to_run_action {
        let run_result = run_with_progress(
            plugin.run(
                target_id,
                step_id,
                emakefile_current_path,
                false,
                &step.action,
                &real_in_files,
                &plugin_out_files,
                &working_dir,
                Some(&default_replacements),
            )
            ,
            Duration::from_secs(5),
            step_id,
            &step_description,
        )
        .await
        .map_err(|e| {
                let msg = e.to_string();
                Box::<dyn Error + Send + Sync>::from(msg)
            });

        if !run_result.is_err() {
            // Register footprint
            register_action_footprint(&step_id, &action_footprint).await;

            // Register files cache
            for file in real_in_files {
                let mut filename = file;

                if is_url(&filename) {
                    let encoded_filename = urlencoding::encode(&filename).to_string();
                    filename = encoded_filename;
                }

                let file_absolute_path = String::from(
                    get_absolute_file_path(&filename)
                        .to_str()
                        .unwrap(),
                );
                CACHE_IN_FILE_TO_UPDATE.insert((file_absolute_path, String::from(step_id)));
            }

            for file in &real_out_files {
                let file_absolute_path = String::from(
                    get_absolute_file_path( file)
                        .to_str()
                        .unwrap(),
                );
                CACHE_OUT_FILE_TO_UPDATE.insert((file_absolute_path, String::from(step_id)));
            }

            // Compute checksum
            if let Some(checksum_command) = &checksum_command {
                let mut maybe_checksum: Option<String> = None;
                let (status, stdout, stderr) = utils::run_command(
                    checksum_command,
                    Path::new(emakefile_current_path),
                    Some(&default_replacements),
                );

                if ExitStatus::success(&status) {
                    maybe_checksum = Some(stdout);
                } else {
                    log::warning!("Error when computing checksum of action {step_id}: {stderr}");
                }

                if let Some(checksum) = maybe_checksum {
                    cache::write_cache_action_checksum(step_id, &checksum.trim().to_string()).await
                }
            }

            // log::step_info!(step_id, StepStatus::Finished, step_description);
        }

        return run_result;
    } else {
        log::step_info!(step_id, StepStatus::Skipped, step_description);
    }

    Ok(())
}

pub fn run_target<'a>(
    target_absolute_path: String,
) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        if RUNNED_TARGETS.contains_key(&target_absolute_path) {
            let mutex = RUNNED_TARGETS.get(&target_absolute_path).unwrap();
            let _target_lock = mutex.lock().await;
            return;
        }

        let mutex = Arc::new(tokio::sync::Mutex::new(()));
        RUNNED_TARGETS.insert(target_absolute_path.clone(), mutex.clone());
        let _target_lock = mutex.lock().await;

        let emakefile_path = to_emakefile_path(&target_absolute_path);
        let emakefile = emake::loader::load_file(&emakefile_path.to_string_lossy().to_string());
        let target_info = extract_info_from_path(
            &target_absolute_path,
            &emakefile_path.to_string_lossy().to_string(),
        );

        let target_absolute_path_clone = target_absolute_path.clone();
        let maybe_target = emakefile.targets.get(&target_info.unwrap().target_name);

        if maybe_target.is_none() {
            log::panic!("Target not found: {}", target_absolute_path_clone);
        }
        let target = maybe_target.unwrap();

        if let Some(deps) = &target.deps {
            let mut dependencies_tasks: Vec<JoinHandle<()>> = Vec::new();

            for dependency in deps {
                let dependency_target_path =
                    get_absolute_target_path(dependency, &emakefile.path.clone().unwrap());

                let dependency_target_path_clone = dependency_target_path.clone();
                let handle = tokio::spawn(async move {
                    run_target(dependency_target_path_clone).await;
                });

                dependencies_tasks.push(handle);
            }

            // Await all dependency tasks
            futures::future::join_all(dependencies_tasks).await;
        }

        if let Some(steps) = &target.steps {
            let mut steps_tasks: Vec<JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>> =
                Vec::new();

            for (step_index, step) in steps.iter().enumerate() {
                let step_index_string = format!("{}", step_index);
                let step_id = target_absolute_path.clone() + "/" + step_index_string.as_str();
                let step_id_clone: String = step_id.clone();
                let target_id_clone = target_absolute_path.clone();
                let step_clone = step.clone();
                let emakefile_path_str = emakefile_path.to_string_lossy().to_string();
                let emakefile_path_str_clone = emakefile_path_str.clone();

                if target.parallel.unwrap_or(false) {
                    let fut = async move {
                        let m = get_mutex_for_id(&step_id_clone).await;
                        let _guard = m.lock().await;
                        update_progress(true, false);
                        let run_step_result = run_step(
                            &target_id_clone,
                            &step_id_clone,
                            &step_clone,
                            &emakefile_path_str,
                            None,
                        )
                        .await;
                        update_progress(false, true);
                        run_step_result
                    };
                    let handle: JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> =
                        tokio::spawn(fut);
                    steps_tasks.push(handle);
                } else {
                    let mut step_out_files =
                        get_real_out_files(&step_id, &step, &emakefile_path_str_clone).await;
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
                            &emakefile_path_str_clone,
                        )
                        .await;
                        for step_out_file in &step_out_files {
                            if current_step_in_files.contains(step_out_file) {
                                step_out_files = get_real_out_files(
                                    &current_step_id,
                                    current_step,
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
                    update_progress(true, false);
                    let run_step_result = run_step(
                        &target_id_clone,
                        &step_id_clone,
                        &step_clone,
                        &emakefile_path_str,
                        Some(step_out_files),
                    )
                    .await;
                    update_progress(false, true);
                    if run_step_result.is_err() {
                        log::panic!(
                            "An error occured when running the step [{}] {}, the status code is not 0. Error: {}",
                            step_id_clone,
                            step.description,
                            run_step_result.err().unwrap()
                        );
                    }
                }
            }

            let join_results = futures::future::join_all(steps_tasks).await;
            for result in join_results {
                let run_step_result = result.unwrap();
                if run_step_result.is_err() {
                    log::panic!(
                        "An error occured when running a step, the status code is not 0. Error: {}",
                        run_step_result.err().unwrap()
                    );
                }
            }
        }
    })
}
