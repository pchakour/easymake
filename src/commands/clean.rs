use crate::{console::log::{self, StepStatus}, get_cwd};

const CACHE_DIR: &str = ".emake";

pub async fn run(dry_run: &bool) {
    // let clean_commands = graph::analysor::get_clean_commands(cwd);
    let path = get_cwd().join(CACHE_DIR);

    // Getting out_files from cache
    let cache_folder = path.join("cache");

    if *dry_run {
        log::info!("List of files to delete:");
    }

    let mut files_to_delete = Vec::new();

    if path.exists() {
        files_to_delete.push(path.to_string_lossy().to_string());
    }

    for out_file_result in glob::glob(&format!("{}/**/tag_out_file", cache_folder.to_str().unwrap())).unwrap()
    {
        if let Ok(out_file) = out_file_result {
            let dirname = out_file.parent().unwrap();
            // Exclude outfile that are also in_file for the same target
            let has_in_file = dirname.join("tag_in_file").exists();
            if has_in_file {
                log::debug!("Ignoring file because it's also an in_file {:?}", out_file);
                continue;
            }

            let file_to_delete = dirname.to_string_lossy().replacen(cache_folder.to_str().unwrap(), "", 1);
            files_to_delete.push(file_to_delete);
        }
    }

    let mut remove_duplicate = files_to_delete.clone();
    remove_duplicate.retain(|file| files_to_delete.iter().find(|f| file != *f && file.contains(*f)).iter().len() == 0);

    for f in &remove_duplicate {
        log::info!("    {}", f);
    }

    if !dry_run {
        log::step_info!("CACHE", StepStatus::Running, "Removing files");
        fs_extra::remove_items(&remove_duplicate).unwrap();
        log::step_info!("CACHE", StepStatus::Finished, "Cache cleaned successfully !");
    } else {
        log::warning!("Dry run mode nothing was deleted");
    }

}
