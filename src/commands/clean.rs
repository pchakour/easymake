use std::{
    collections::HashMap, fs, io::{self, BufRead, BufReader}, path::{Path, PathBuf}, process::{Command, Stdio}, sync::{Arc, Mutex}
};

use crate::{cache, console::log, emake, get_cwd, graph};

const CACHE_DIR: &str = ".emake";

fn contains_subdirectory<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn run(dry_run: &bool) {
    // let clean_commands = graph::analysor::get_clean_commands(cwd);
    let path = get_cwd().join(CACHE_DIR);

    // Getting out_files from cache
    let cache_folder = path.join("cache");

    if *dry_run {
        log::info!("List of files to delete:");
    }

    for out_file_result in glob::glob(&format!("{}/**/tag_out_file", cache_folder.to_str().unwrap())).unwrap()
    {
        if let Ok(out_file) = out_file_result {
            let dirname = out_file.parent().unwrap();
            // Exclude outfile that are also in_file for the same target
            let has_in_file = dirname.join("tag_in_file").exists();
            if has_in_file {
                // log::debug!("Ignoring file because it's also an in_file {:?}", out_file);
                continue;
            }
            
            // Exclude directories that contain also result of other target
            let has_other_target = contains_subdirectory(&dirname).unwrap();
            // if has_other_target {
            //     // log::debug!("Ignoring file because it contains at least an other target {:?}", out_file);
            //     continue;
            // }

            let file_to_delete = dirname.to_string_lossy().replacen(cache_folder.to_str().unwrap(), "", 1);
            
            if !dry_run {
                log::debug!("Cache file to remove {}", file_to_delete);
                let _ = fs_extra::remove_items(&[&file_to_delete]);
            } else {
                log::info!("    {}", file_to_delete);
            }
        }
    }
    
    // Delete emake directory
    if !dry_run {
        log::debug!("Cache file to remove {}", path.to_string_lossy().to_string());
        let _ = std::fs::remove_dir_all(path);
    } else {
        log::info!("    {}", path.to_string_lossy().to_string());
    }

    // let working_dir = cache::get_working_dir_path();
    // let out_dir = cache::get_out_dir_path();
    // let default_replacements = HashMap::from([
    //     (String::from("EMAKE_WORKING_DIR"), working_dir.to_owned()),
    //     (String::from("EMAKE_OUT_DIR"), out_dir.to_owned()),
    //     (
    //         String::from("EMAKE_CWD_DIR"),
    //         String::from(cwd.to_str().unwrap()),
    //     ),
    // ]);

    // for (_, (emakefile_path, command)) in &clean_commands {
    //     let compiled_command = emake::compiler::compile(
    //         cwd.to_str().unwrap(),
    //         command,
    //         emakefile_path,
    //         Some(&default_replacements),
    //     );

    //     let mut shell = "sh";
    //     let mut arg = "-c";

    //     if cfg!(target_os = "windows") {
    //         shell = "cmd";
    //         arg = "/C";
    //     }

    //     let mut output = Command::new(shell)
    //         .current_dir(cwd)
    //         .arg(arg) // Pass the command string to the shell
    //         .arg(compiled_command)
    //         .stdout(Stdio::piped())
    //         .stderr(Stdio::piped())
    //         .spawn()
    //         .expect("Failed to execute command");

    //     let stdout = output.stdout.take().unwrap();
    //     let stderr = output.stderr.take().unwrap();

    //     let stdout_reader = BufReader::new(stdout);
    //     let stderr_reader = BufReader::new(stderr);

    //     // Spawn threads to read both stdout and stderr
    //     let stdout_thread: std::thread::JoinHandle<()> = std::thread::spawn(move || {
    //         for line in stdout_reader.lines() {
    //             if let Ok(text) = line {
    //                 log::text!("{}{}", log::INDENT, text);
    //             }
    //         }
    //     });

    //     stdout_thread.join().unwrap();

    //     let stderr_buffer: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new())); // Mutex allows safe mutation
    //                                                                                  // let stderr_buffer_clone = Arc::clone(&stderr_buffer);

    //     let stderr_thread = std::thread::spawn(move || {
    //         for line in stderr_reader.lines() {
    //             if let Ok(text) = line {
    //                 log::warning!("{}{}", log::INDENT, text);
    //             }
    //         }
    //     });

    //     stderr_thread.join().unwrap();

    //     let status = output.wait().expect("Failed to wait on child");

    //     if !status.success() {
    //         log::warning!("{}", stderr_buffer.lock().unwrap());
    //     }
    // }
}
