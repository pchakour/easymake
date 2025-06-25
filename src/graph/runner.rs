use crate::console::log;
use crate::graph::InFile;
use crate::{emake, graph};
use crate::actions::ActionsStore;
use std::fs::File;
use std::io::{copy, BufWriter};
use std::process;
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    sync::Arc,
};
use tokio::{
    fs,
    sync::{RwLock, Semaphore},
    task::{self, JoinHandle},
};
use url::Url;
use std::path::PathBuf;


const CACHE_DIR: &str = ".emake/cache";
const WORKING_DIR: &str = ".emake/workspace";

async fn get_cache_dir_path(cwd: &String, is_absolute: bool) -> String {
    get_dir_path(cwd, CACHE_DIR, is_absolute).await
}

async fn get_working_dir_path(cwd: &String, is_absolute: bool) -> String {
    get_dir_path(cwd, WORKING_DIR, is_absolute).await
}

async fn get_dir_path(cwd: &String, dir: &str, is_absolute: bool) -> String {
    let relative_path = PathBuf::from(cwd.clone() + "/" + dir);

    if is_absolute {
        return String::from(std::fs::canonicalize(&relative_path).unwrap().to_str().unwrap());
    }

    String::from(relative_path.to_str().unwrap())
}

async fn create_dir(cwd: &String, dir: &str) {
    let cache_dir = get_dir_path(cwd, dir, false).await;
    let path = std::path::Path::new(&cache_dir);
    if let Ok(cache_file_dir_exists) = fs::try_exists(&path).await {
        if !cache_file_dir_exists {
            fs::create_dir_all(&path).await.unwrap();
        }
    }
}

fn get_absolute_file_path(cwd: &String, file: &String) -> std::path::PathBuf {
    let absolute_path = cwd.clone();
    let mut path = std::path::PathBuf::from(&absolute_path);
    path.push(file);
    path
}

async fn get_file_cache(cwd: &String, file_absolute_path: &String) -> std::path::PathBuf {
    let cache_path = format!("{}/{}/time", get_cache_dir_path(cwd, false).await, file_absolute_path);
    let path = std::path::Path::new(&cache_path);
    path.to_path_buf()
}

async fn get_file_modification_time(file_absolute_path: &String) -> Option<String> {
    match fs::try_exists(&file_absolute_path).await {
        Ok(filepath_exists) => {
            if filepath_exists {
                match fs::metadata(&file_absolute_path).await {
                    Ok(metadata) => {
                        let current_time = format!("{:?}", metadata.modified().unwrap());
                        return Some(current_time);
                    }
                    Err(error) => {
                        println!("ERROR {}", error);
                    }
                }
            }
        },
        Err(error) => {
            panic!("{}", error);
        }
    }

    None
}

async fn write_cache(cwd: &String, cache_to_update: Arc<RwLock<Vec<String>>>,) {
    let read_cache_to_update = cache_to_update.read().await;
    for file_absolute_path in read_cache_to_update.iter() {
        if let Ok(file_exists) = fs::try_exists(&file_absolute_path).await {
            if file_exists {
                if is_file_changed(cwd, file_absolute_path).await {
                    println!("Absolute path {}", file_absolute_path);
                    let maybe_current_time = get_file_modification_time(&file_absolute_path).await;
    
                    if let Some(current_time) = maybe_current_time {
                        write_file_in_cache(cwd, &file_absolute_path, &current_time).await;
                    }
                }
            } else {
                log::error!("You try to cache a file that doesn't exist. Check your input/output, the file is {}", file_absolute_path);
                process::exit(1);
            }
        }
    }
}

async fn write_file_in_cache(cwd: &String, file_absolute_path: &String, modification_date: &String) {
    let cache_file_path = get_file_cache(cwd, &file_absolute_path).await;
    let cache_file_dir = cache_file_path.parent().unwrap();
    if let Ok(cache_file_dir_exists) = fs::try_exists(&cache_file_dir).await {
        if !cache_file_dir_exists {
            // println!("Exists dir cache {:?}", cache_file_dir);
            fs::create_dir_all(&cache_file_dir).await.unwrap();
        }

        // println!("Write file cache {:?}", cache_file_path);
        fs::write(&cache_file_path, &modification_date).await.unwrap();
    }
}

async fn is_file_changed(cwd: &String, file: &String) -> bool {
    let mut file_changed = true;
    let file_absolute_path = String::from(get_absolute_file_path(cwd, file).to_str().unwrap_or(""));

    if let Some(modification_date) = get_file_modification_time(&file_absolute_path).await {
        let cache_file = get_file_cache(cwd, &file_absolute_path).await;
        if let Ok(cache_file_exists) = fs::try_exists(&cache_file).await {
            if cache_file_exists {
                let previous_time =
                    fs::read_to_string(&cache_file).await.unwrap();
                if previous_time == modification_date {
                    file_changed = false;
                }
            }
        }
    }

    file_changed
}

async fn extract_credentials(credentials_key: &String) {

}

async fn download_file(url: &str, output_path: &str, maybe_credentials: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    log::text!("{}⏳ Downloading file {}...", log::INDENT, url);

    let mut credentials = None;
    if let Some(credentials_key) = maybe_credentials {
        credentials = Some(extract_credentials(&credentials_key).await);
    }

    // Validate URL
    let parsed_url: Url = Url::parse(url)?;
    if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
        return Err("URL must be HTTP or HTTPS".into());
    }

    // Send GET request
    let response = reqwest::get(url).await.expect("request failed");
    if !response.status().is_success() {
        return Err(format!("Failed to download: HTTP {}", response.status()).into());
    }

    // Open file for writing
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    // Stream response into file
    let content = response.bytes().await?;
    copy(&mut content.as_ref(), &mut writer)?;

    log::text!("{}✅ File downloaded to: {}", log::INDENT, output_path);
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

// async fn compute_out_files_cache(cwd: &String, cache_to_update: Arc<RwLock<Vec<String>>>) {
//     let working_dir = get_working_dir_path(cwd, true).await;
//     let default_replacements = HashMap::from([
//         ("EMAKE_WORKING_DIR", working_dir.as_str())
//     ]);

//     for out_file in cache_to_update.read().await.into_iter() {
//         let compiled_out_file_string = emake::compiler::compile(cwd, &out_file, Some(&default_replacements));
//         let mut files = Vec::from([compiled_out_file_string.clone()]);
//         let parsed_compiled_files_result: Result<Vec<String>, _> = serde_yml::from_str(&compiled_out_file_string);
//         match parsed_compiled_files_result {
//             Ok(parsed_compiled_files) => files = parsed_compiled_files,
//             Err(_) => ()
//         }

//         for file in files {
//             cache_to_update.write().await.insert(file);
//         }
//     }
// }

async fn run_action(
    action_id: &String,
    silent: bool,
    cwd: &String,
    emakefile_cwd: &String,
    in_files: &Vec<InFile>,
    out_files: &Vec<String>,
    action: &graph::Action,
    plugins_store: Arc<RwLock<ActionsStore>>,
    cache_to_update: Arc<RwLock<Vec<String>>>,
) {
    log::info!("{}Running action {:?}", log::INDENT, action);
    let read_plugin_store = plugins_store.read().await;
    let maybe_plugin = read_plugin_store.get(&action.plugin_id);
    // let mut files_to_update_cache = HashSet::new();
    
    if let Some(plugin) = maybe_plugin {
        let mut need_to_run_action = in_files.len() == 0 && out_files.len() == 0;
        let mut real_in_files = Vec::new();
        let mut real_out_files = Vec::new();
        let working_dir = get_working_dir_path(cwd, true).await;
        let default_replacements = HashMap::from([
            (String::from("EMAKE_WORKING_DIR"), working_dir.to_owned()),
            (String::from("EMAKE_CWD_DIR"), cwd.to_owned()),
        ]);

        // Get in files modification date
        for in_file in in_files {
            let compiled_in_file_string = emake::compiler::compile(cwd, &in_file.file, Some(&default_replacements));
            let mut files = Vec::from([compiled_in_file_string.clone()]);

            let parsed_compiled_files_result: Result<Vec<String>, _> = serde_yml::from_str(&compiled_in_file_string);
            match parsed_compiled_files_result {
                Ok(parsed_compiled_files) => files = parsed_compiled_files,
                Err(_) => ()
            }

            let mut files_to_replace = HashMap::new();
            for (index, file) in files.iter().enumerate() {
                if graph::common::is_downloadable_file(file) {
                    let filename = get_filename_from_url(file).unwrap();
                    let mut output = PathBuf::from(get_working_dir_path(cwd, true).await);
                    output.push(&filename);
                    let output_string = String::from(output.to_str().unwrap());
                    if is_file_changed(cwd, &output_string).await {
                        download_file(file, &output_string).await.unwrap();
                    }
                    files_to_replace.insert(output_string, index);
                }
            }

            for (replace, index) in files_to_replace {
                let _ = std::mem::replace(&mut files[index], replace);
            }

            real_in_files.extend(files);
        }

        for out_file in out_files {
            let compiled_out_file_string = emake::compiler::compile(cwd, out_file, Some(&default_replacements));
            let mut files = Vec::from([compiled_out_file_string.clone()]);

            let parsed_compiled_files_result: Result<Vec<String>, _> = serde_yml::from_str(&compiled_out_file_string);
            match parsed_compiled_files_result {
                Ok(parsed_compiled_files) => files = parsed_compiled_files,
                Err(_) => ()
            }

            real_out_files.extend(files);
        }

        let mut real_files = Vec::new();
        real_files.extend(&real_in_files);       
        real_files.extend(&real_out_files);

        for file in real_files {
            let file_absolute_path = String::from(get_absolute_file_path(cwd, &file).to_str().unwrap());
            cache_to_update.write().await.push(file_absolute_path);
            let file_changed = is_file_changed(cwd, &file).await;
            if file_changed {
                need_to_run_action = true;
            }
        }

        if need_to_run_action {
            plugin.action(
                cwd,
                emakefile_cwd,
                silent,
                &action.args,
                &real_in_files,
                &real_out_files,
                &working_dir,
                Some(&default_replacements)
            ).await;
            log::info!("{}Action {:?} done !", log::INDENT, action);
        } else {
            log::info!(
                "{}No need to run action {:?} because no input/output files changed", log::INDENT, action
            );
        }
    }
}

fn bfs_parallel(
    graph: Arc<RwLock<graph::Graph>>,
    node_id: String,
    semaphore: Arc<Semaphore>,
    running_tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
    plugins_store: Arc<RwLock<ActionsStore>>,
    visited: Arc<RwLock<HashSet<String>>>,
    silent: bool,
    cwd: String,
    root_dir: String,
    cache_to_update: Arc<RwLock<Vec<String>>>,
    // outfiles: Arc<RwLock<Vec<String>>>,
    p_current_step: usize,
    total_steps: usize,
) -> impl Future + Send {
    async move {
        let mut current_step = p_current_step;
        if !visited.read().await.contains(&node_id) {
            visited.write().await.insert(node_id.clone());

            // Get the current node
            let maybe_node = {
                let safe_graph = graph.read().await;
                safe_graph.nodes.get(&node_id).cloned()
            };

            if let Some(node) = maybe_node {
                // Wait for all incomming actions
                for in_neighbor in &node.in_neighbors {
                    if let Some(t) = running_tasks.write().await.get_mut(in_neighbor) {
                        t.await.unwrap();
                    }
                    running_tasks.write().await.remove(in_neighbor);
                }

                // Get all output neighbors
                let mut action_number = 0;
                let neighbors = node.out_neighbors;
                for neighbor_id in &neighbors {
                    // If action run it
                    let g = graph.read().await;
                    let neighbor = g.nodes.get(neighbor_id).unwrap();
                    let in_files = neighbor.in_files.clone();
                    let out_files = neighbor.out_files.clone();
                    let action_id = neighbor.id.clone();
                    let emakefile_cwd = neighbor.cwd.clone();
                    if let Some(action) = &neighbor.action {
                        // Run it using the plugin store
                        let _ = semaphore.acquire().await.unwrap();
                        let plugins_store_clone = Arc::clone(&plugins_store);
                        let cache_to_update_clone = Arc::clone(&cache_to_update);
                        let cloned_action = action.clone();
                        let cloned_cwd = cwd.clone();
                        current_step += 1;
                        action_number += 1;
                        log::step!(current_step, total_steps, "Running action {} of target {}", action_number, node_id);
                        let r = task::spawn(async move {
                            run_action(
                                &action_id,
                                silent,
                                &cloned_cwd,
                                &emakefile_cwd,
                                &in_files,
                                &out_files,
                                &cloned_action,
                                plugins_store_clone,
                                cache_to_update_clone,
                            )
                            .await;
                        });

                        running_tasks.write().await.insert(neighbor_id.clone(), r);
                    }
                }

                for neighbor_id in &neighbors {
                    let graph_clone = Arc::clone(&graph);
                    let neighbor_clone = neighbor_id.clone();
                    let semaphore_clone = Arc::clone(&semaphore);
                    let running_tasks_clone = Arc::clone(&running_tasks);
                    let plugins_store_clone = Arc::clone(&plugins_store);
                    let visited_clone = Arc::clone(&visited);
                    let cache_to_update_clone = Arc::clone(&cache_to_update);
                    // let outfiles_clone = Arc::clone(&outfiles);
                    let silent_clone = silent.clone();

                    Box::pin(bfs_parallel(
                        graph_clone,
                        neighbor_clone,
                        semaphore_clone,
                        running_tasks_clone,
                        plugins_store_clone,
                        visited_clone,
                        silent_clone,
                        cwd.clone(),
                        root_dir.clone(),
                        cache_to_update_clone,
                        // outfiles_clone,
                        current_step,
                        total_steps
                    ))
                    .await;
                }
            }
        }
    }
}

pub async fn run_target(
    target_id: &String,
    graph: graph::Graph,
    plugins_store: ActionsStore,
    silent: &bool,
    cwd: &String,
) {
    let total_steps = graph::analysor::steps_len(&graph);
    log::info!("Total steps to run {total_steps} for target {target_id}\n");

    let mgraph = Arc::new(RwLock::new(graph));
    let semaphore = Arc::new(Semaphore::new(15));
    let running_tasks = Arc::new(RwLock::new(HashMap::new()));
    let running_tasks_clone = Arc::clone(&running_tasks);
    let mplugins_store = Arc::new(RwLock::new(plugins_store));
    let visited = Arc::new(RwLock::new(HashSet::new()));
    let cache_to_update = Arc::new(RwLock::new(Vec::new()));
    // let cache_to_update = Arc::new(RwLock::new(HashMap::new()));
    let read_cache_to_update = cache_to_update.clone();
    // let write_cache_to_update = cache_to_update.clone();
    let silent_clone = silent.clone();
    // let outfiles = Arc::new(RwLock::new(Vec::new()));
    // let read_outfiles: Arc<RwLock<Vec<String>>> = outfiles.clone();

    create_dir(cwd, WORKING_DIR).await;
    create_dir(cwd, CACHE_DIR).await;

    bfs_parallel(
        mgraph,
        target_id.clone(),
        semaphore,
        running_tasks_clone,
        mplugins_store,
        visited,
        silent_clone,
        cwd.clone(),
        cwd.clone(),
        cache_to_update,
        // outfiles,
        0,
        total_steps
    )
    .await;

    for task in running_tasks.write().await.iter_mut() {
        task.1.await.unwrap();
    }

    write_cache(cwd, read_cache_to_update).await;
}
