use crate::actions::{
    compute_action_footprint, get_registered_action_footprint, register_action_footprint,
};
use crate::console::log;
use crate::emake::loader::{extract_info_from_path, Target, TargetType};
use crate::emake::Step;
use crate::graph::generator::{get_absolute_target_path, to_emakefile_path};
use crate::utils::get_absolute_file_path;
use crate::{
    cache, credentials, emake, get_mutex_for_id, graph, utils, ACTIONS_STORE, CACHE_TO_UPDATE,
    CREDENTIALS_STORE, GLOBAL_SEMAPHORE, MULTI_PROGRESS,
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

    // Set up progress bar
    let mp = MULTI_PROGRESS.clone();
    let pb = mp.add(
        ProgressBar::new(total_size)
            .with_style(
                ProgressStyle::with_template("Downloading file {prefix:.bold}\n {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})")?
                    .progress_chars("=> "),
            )
            .with_prefix(String::from(url)),
    );

    let mut dest = BufWriter::new(File::create(output_path)?);
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        dest.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

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

async fn run_step<'a>(
    step_id: &'a str,
    step: &'a Step,
    cwd: &'a str,
    emakefile_current_path: &'a str,
) {
    let mp = MULTI_PROGRESS.clone();
    let spinner = mp.add(ProgressBar::hidden());
    let step_description = step.description.clone().unwrap_or(step_id.to_string());

    spinner.set_message(format!("Running step {}", step_description));
    spinner.set_style(
        ProgressStyle::with_template(&format!("{}{{spinner}} {{msg}}", log::INDENT)).unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(SPINNER_TIME));

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

    let mut out_files;
    if step.out_files.is_none() {
        out_files = Vec::new()
    } else {
        out_files = step.out_files.clone().unwrap()
    };

    plugin.insert_in_files(&step.plugin, &mut in_files).await;
    plugin.insert_out_files(&step.plugin, &mut out_files).await;

    let mut need_to_run_action = in_files.len() == 0 && out_files.len() == 0;
    let mut real_in_files = Vec::new();
    let mut real_out_files = Vec::new();
    let working_dir = cache::get_working_dir_path(cwd);
    let out_dir = cache::get_out_dir_path(cwd);
    let default_replacements = HashMap::from([
        (String::from("EMAKE_WORKING_DIR"), working_dir.to_owned()),
        (String::from("EMAKE_CWD_DIR"), cwd.to_owned()),
        (String::from("EMAKE_OUT_DIR"), out_dir.to_owned()),
    ]);

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

        let mut download_futures = Vec::new();
        let mut downloadable_files_indices = Vec::new();

        for (index, file) in files.iter().enumerate() {
            if graph::common::is_downloadable_file(file) {
                let filename = get_filename_from_url(file).unwrap();
                let mut output = PathBuf::from(cache::get_working_dir_path(cwd));
                output.push(&filename);
                let output_string = output.to_str().unwrap().to_string();
                downloadable_files_indices.push((output_string.clone(), index));

                if cache::has_file_changed(cwd, &output_string, step_id).await {
                    let file = file.clone(); // required if file is &String
                    let cwd = cwd.to_string();
                    let emakefile_current_path = emakefile_current_path.to_string();
                    let file_credentials = file_credentials.clone();
                    let _s = GLOBAL_SEMAPHORE.acquire().await;
                    download_futures.push(tokio::spawn(async move {
                        download_file(
                            &file,
                            &output_string,
                            &cwd,
                            &emakefile_current_path,
                            &file_credentials,
                        )
                        .await
                    }));
                }
            }
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
        for (replace, index) in downloadable_files_indices {
            if let Some(file) = files.get_mut(index) {
                *file = replace;
            }
        }

        real_in_files.extend(files);
    }

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

    let mut real_files = Vec::new();
    real_files.extend(&real_in_files);
    real_files.extend(&real_out_files);

    for file in &real_files {
        let file_changed = cache::has_file_changed(cwd, *file, step_id).await;
        if file_changed {
            log::info!("File change {}", *file);
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
                emakefile_current_path,
                false,
                &step.plugin,
                &real_in_files,
                &real_out_files,
                &working_dir,
                Some(&default_replacements),
            )
            .await;

        if !has_error {
            // Register footprint
            register_action_footprint(&step_id, &action_footprint, cwd).await;

            // Register files cache
            for file in &real_files {
                let file_absolute_path =
                    String::from(get_absolute_file_path(cwd, *file).to_str().unwrap());
                CACHE_TO_UPDATE.insert((file_absolute_path, String::from(step_id)));
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

            cache::write_cache(cwd).await; // Only usefull if we call several time a target
            log::info!("{}Action {} done successfully !", log::INDENT, step_id);
        }
    } else {
        log::info!(
            "{}No need to run action {} because no input/output files changed",
            log::INDENT,
            step_id
        );
    }
}

pub fn run_target3<'a>(
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

                let _s = GLOBAL_SEMAPHORE.acquire().await;
                let handle = tokio::spawn(async move {
                    run_target3(dependency_target_path_clone, cwd_clone).await;
                });

                dependencies_tasks.push(handle);
            }

            // Await all dependency tasks
            futures::future::join_all(dependencies_tasks).await;
        }

        // Run steps now !
        // println!("Run step {:?}", target);
        let mp = MULTI_PROGRESS.clone();
        let spinner = mp.add(ProgressBar::new_spinner());
        spinner.set_message(
            style(format!("Running target {}...", target_absolute_path))
                .blue()
                .bold()
                .to_string(),
        );
        spinner.enable_steady_tick(Duration::from_millis(SPINNER_TIME));
        if let Some(steps) = &target.steps {
            let mut steps_tasks: Vec<JoinHandle<()>> = Vec::new();

            for (step_index, step) in steps.iter().enumerate() {
                let step_index_string = format!("{}", step_index);
                let step_id = target_absolute_path.clone() + "/" + step_index_string.as_str();
                let cwd_clone = cwd.clone();
                let step_clone = step.clone();
                let emakefile_path = emakefile_path.to_string_lossy().to_string();
                let fut = async move {
                    let m = get_mutex_for_id(&step_id).await;
                    let _guard = m.lock().await;
                    run_step(&step_id, &step_clone, &cwd_clone, &emakefile_path).await;
                };

                if target.parallel.unwrap_or(false) {
                    let handle: JoinHandle<()> = tokio::spawn(fut);
                    steps_tasks.push(handle);
                } else {
                    fut.await;
                    log::info!("NIQUER TAMER {:?}", target_absolute_path.clone() + "/" + step_index_string.as_str());
                }
            }

            futures::future::join_all(steps_tasks).await;
            spinner.finish_with_message(format!(
                "{} target {} done",
                style("✔").green().bold(),
                style(target_absolute_path).bold()
            ));
        }
    })
}
