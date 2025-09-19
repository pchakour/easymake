use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};
use crate::{console::log, graph::runner::is_url, utils::get_absolute_file_path, CACHE_IN_FILE_TO_UPDATE, CACHE_OUT_FILE_TO_UPDATE};

const CACHE_DIR: &str = ".emake/cache";
const WORKING_DIR: &str = ".emake/workspace";
const OUT_DIR: &str = ".emake/out";
const FOOTPRINTS_DIR: &str = ".emake/footprints";

async fn create_dir(cwd: &str, dir: &str) {
    let cache_dir = get_dir_path(cwd, dir);
    let path = std::path::Path::new(&cache_dir);
    if let Ok(cache_file_dir_exists) = tokio::fs::try_exists(&path).await {
        if !cache_file_dir_exists {
            tokio::fs::create_dir_all(&path).await.unwrap();
        }
    }
}

pub async fn create_cache_dir(cwd: &str) {
    create_dir(cwd, CACHE_DIR).await;
    create_dir(cwd, WORKING_DIR).await;
    create_dir(cwd, OUT_DIR).await;
    create_dir(cwd, FOOTPRINTS_DIR).await;
}

pub async fn write_cache(cwd: &str) {
    // Now remove and collect owned JoinHandles
    let cache_in_file_to_update: Vec<(String, String)> = CACHE_IN_FILE_TO_UPDATE
        .iter()
        .map(|entry| entry.key().clone())
        .collect::<Vec<_>>();

    write_file_cache(&cache_in_file_to_update, &true, cwd).await;

    let cache_out_file_to_update: Vec<(String, String)> = CACHE_OUT_FILE_TO_UPDATE
        .iter()
        .map(|entry| entry.key().clone())
        .collect::<Vec<_>>();

    write_file_cache(&cache_out_file_to_update, &false, cwd).await;
}

async fn write_file_cache(files: &Vec<(String, String)>, ignore_not_exists: &bool, cwd: &str) {
    for (file_absolute_path, action_id) in files {
        if let Ok(file_exists) = tokio::fs::try_exists(&file_absolute_path).await {
            if file_exists {
                if has_file_changed(cwd, file_absolute_path, action_id, ignore_not_exists).await {
                    let maybe_current_time = get_file_modification_time(&file_absolute_path).await;

                    if let Some(current_time) = maybe_current_time {
                        write_file_in_cache(cwd, file_absolute_path, action_id, &current_time)
                            .await;
                    }
                }
            } else {
                let current_time = format!("{:?}", SystemTime::now()).replace(" ", "");
                write_file_in_cache(cwd, file_absolute_path, action_id, &current_time).await;

                if !ignore_not_exists {
                    log::error!("You try to cache a file that doesn't exist. Check your input/output, the file is {}", file_absolute_path);
                    std::process::exit(1);
                }
            }
        }
    }
}

async fn get_file_modification_time(file_absolute_path: &str) -> Option<String> {
    match tokio::fs::try_exists(&file_absolute_path).await {
        Ok(true) => match tokio::fs::metadata(&file_absolute_path).await {
            Ok(metadata) => {
                let mut current_time = format!("{:?}", metadata.modified().unwrap());
                current_time = current_time.replace(" ", "");
                return Some(current_time);
            }
            Err(error) => {
                println!("ERROR {}", error);
            }
        },
        Ok(false) => (),
        Err(error) => {
            panic!(
                "try_exists failed on file {}: {}",
                file_absolute_path, error
            );
        }
    }

    None
}

async fn write_file_in_cache(
    cwd: &str,
    file_absolute_path: &str,
    action_id: &str,
    modification_date: &str,
) {
    let cache_file_path = get_file_cache(cwd, &file_absolute_path);
    let cache_file_dir = cache_file_path.parent().unwrap();
    if let Ok(cache_file_dir_exists) = tokio::fs::try_exists(&cache_file_dir).await {
        if !cache_file_dir_exists {
            // println!("Exists dir cache {:?}", cache_file_dir);
            tokio::fs::create_dir_all(&cache_file_dir).await.unwrap();
        }

        // println!("Write file cache {:?}", cache_file_path);

        // Check if the line already exist
        let mut action_line = String::from(action_id);
        action_line.push_str(" ");
        action_line.push_str(&modification_date);

        if let Ok(cache_file_exists) = tokio::fs::try_exists(&cache_file_path).await {
            if cache_file_exists {
                let file_content = tokio::fs::read_to_string(&cache_file_path).await.unwrap();
                let mut lines: Vec<&str> = file_content.split("\n").collect();
                let mut maybe_line_action_id_index = None;

                for (line_index, line) in lines.iter().enumerate() {
                    let details: Vec<&str> = line.split(" ").collect();
                    let line_action_id = details[0];

                    if line_action_id == action_id {
                        maybe_line_action_id_index = Some(line_index);
                        break;
                    }
                }

                if let Some(line_action_id_index) = maybe_line_action_id_index {
                    lines[line_action_id_index] = action_line.as_str();
                } else {
                    lines.push(&action_line.as_str());
                }

                tokio::fs::write(&cache_file_path, &lines.join("\n"))
                    .await
                    .unwrap();
                return ();
            }
        }

        tokio::fs::write(&cache_file_path, &action_line)
            .await
            .unwrap();
    }
}

pub async fn get_cache_action_checksum(action_id: &str, cwd: &str) -> Option<String> {
    let cache_dir = get_cache_dir_path(cwd);
    let checksum_cache_path = Path::new(&cache_dir).join("checksum");

    if let Ok(checksum_cache_exists) = tokio::fs::try_exists(&checksum_cache_path).await {
        if checksum_cache_exists {
            if let Ok(cache_content) = tokio::fs::read_to_string(&checksum_cache_path).await {
                let lines: Vec<&str> = cache_content.split("\n").collect();
                for line in lines {
                    let details: Vec<&str> = line.split(" ").collect();
                    let line_action_id = details[0];
                    if action_id == line_action_id {
                        let line_checksum = details[1];
                        return Some(String::from(line_checksum));
                    }
                }
            }
        }
    }

    None
}

pub async fn write_cache_action_checksum(action_id: &str, checksum: &str, cwd: &str) {
    let cache_dir = get_cache_dir_path(cwd);
    let checksum_cache_path = Path::new(&cache_dir).join("checksum");

    if let Ok(checksum_cache_exists) = tokio::fs::try_exists(&checksum_cache_path).await {
        if !checksum_cache_exists {
            tokio::fs::File::create(&checksum_cache_path).await.unwrap();
        }
    }

    if let Ok(cache_content) = tokio::fs::read_to_string(&checksum_cache_path).await {
        let mut maybe_action_line_index = None;
        let mut lines: Vec<&str> = cache_content.split("\n").collect();
        for (line_index, line) in lines.iter().enumerate() {
            let details: Vec<&str> = line.split(" ").collect();
            let line_action_id = details[0];
            if action_id == line_action_id {
                maybe_action_line_index = Some(line_index);
                break;
            }
        }

        let formated_line = format!("{} {}", action_id, checksum);
        if let Some(action_line_index) = maybe_action_line_index {
            lines[action_line_index] = formated_line.as_str();
        } else {
            lines.push(formated_line.as_str());
        }

        let content = lines.join("\n");
        tokio::fs::write(&checksum_cache_path, &content)
            .await
            .unwrap();
    }
}

pub async fn has_file_changed(cwd: &str, file: &str, action_id: &str, ignore_not_exists: &bool) -> bool {
    let mut filename = String::from(file);

    if is_url(file) {
        filename = urlencoding::encode(file).to_string();
    }

    let mut file_changed = !ignore_not_exists;
    let file_absolute_path = String::from(get_absolute_file_path(cwd, &filename).to_str().unwrap_or(""));

    if let Some(modification_date) = get_file_modification_time(&file_absolute_path).await {
        file_changed = true;
        let cache_file = get_file_cache(cwd, &file_absolute_path);
        if let Ok(cache_file_exists) = tokio::fs::try_exists(&cache_file).await {
            if cache_file_exists {
                let file_content = tokio::fs::read_to_string(&cache_file).await.unwrap();
                let lines: Vec<&str> = file_content.split("\n").collect();

                for line in lines {
                    let details: Vec<&str> = line.split(" ").collect();
                    let line_action_id = details[0];

                    if line_action_id == action_id {
                        let previous_time = String::from(details[1]);
                        if previous_time == modification_date {
                            file_changed = false;
                        }
                        break;
                    }
                }
            }
        }
    }

    file_changed
}

fn get_file_cache(cwd: &str, file_absolute_path: &str) -> std::path::PathBuf {
    let cache_path = format!("{}{}/time", get_cache_dir_path(cwd), file_absolute_path);
    let path = std::path::Path::new(&cache_path);
    path.to_path_buf()
}

pub fn get_cache_dir_path(cwd: &str) -> String {
    get_dir_path(cwd, CACHE_DIR)
}

pub fn get_working_dir_path(cwd: &str) -> String {
    get_dir_path(cwd, WORKING_DIR)
}

pub fn get_out_dir_path(cwd: &str) -> String {
    get_dir_path(cwd, OUT_DIR)
}

pub fn get_footprints_dir_path(cwd: &str) -> String {
    get_dir_path(cwd, FOOTPRINTS_DIR)
}

fn get_dir_path(cwd: &str, dir: &str) -> String {
    let relative_path = PathBuf::from(cwd).join(dir);

    if !relative_path.is_absolute() {
        return String::from(
            std::fs::canonicalize(&relative_path)
                .unwrap()
                .to_str()
                .unwrap(),
        );
    }

    String::from(relative_path.to_str().unwrap())
}
