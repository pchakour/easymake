use serde_yml;
use crate::emake::{self, CredentialEntry, VariableEntry};
use crate::console::log;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum TargetType {
    Secrets,
    Variables,
    Targets,
}

#[derive(Debug)]
pub struct PathInfo {
    pub emakefile_path: PathBuf,
    pub target_type: TargetType,
    pub target_name: String,
}

pub enum Target {
    TargetEntry(emake::Target),
    VariableEntry(VariableEntry),
    CredentialEntry(CredentialEntry),
}

fn read_file_content(path: &str) -> String {
    // log::info!("Loading file {:?}", path);
    let content = std::fs::read_to_string(path).unwrap();
    return content;
}

fn target_type_str_to_enum(target_type: &str) -> TargetType {
    if target_type == "targets" {
        return TargetType::Targets;
    } else if target_type == "secrets" {
        return TargetType::Secrets;
    } else if target_type == "variables" {
        return TargetType::Variables;
    } else {
        log::error!("Unknown target type {}", target_type);
        std::process::exit(1);
    }
}

pub fn extract_info_from_path(path: &str, cwd: &str, emakefile_current_path: &str) -> PathInfo {
    let mut target_split: Vec<&str> = path.split('/').collect();
    target_split.retain(|s| !s.is_empty());
    let from_root = path.starts_with("//");
    let maybe_real_target: Option<&str> = target_split.pop();

    if let Some(real_target) = maybe_real_target {
        let real_target_split: Vec<&str> = real_target.split(':').collect();
        let mut target_type = TargetType::Targets;
        let mut target_name = real_target.to_string();

        if real_target_split.len() == 2 {
            target_type = target_type_str_to_enum(real_target_split[0]);
            target_name = String::from(real_target_split[1]);
        }

        let mut emakefile_path = Path::new(cwd).join("Emakefile");
        if !from_root {
            emakefile_path = PathBuf::from(emakefile_current_path);
        }
        emakefile_path.pop();
        if target_split.len() > 0 {
            emakefile_path = emakefile_path.join(target_split.iter().collect::<PathBuf>());
        }
        emakefile_path = emakefile_path.join("Emakefile");

        return PathInfo {
            emakefile_path,
            target_type,
            target_name
        }
    }

    log::error!("Malformed target path {}", path);
    std::process::exit(1);
}

pub fn get_target_on_path(
    cwd: &str,
    secrets_path: &str,
    emakefile_current_path: &str,
    maybe_force_type: Option<TargetType>
) -> Result<Target, String> {
    // Check if the target exists in the current Emakefile
    let mut secrets_path_info = extract_info_from_path(secrets_path, cwd, emakefile_current_path);

    if let Ok(emakefile_exists) = std::fs::exists(&secrets_path_info.emakefile_path) {
        if !emakefile_exists {
            return Err(format!("Emakefile {} doesn't exist", secrets_path_info.emakefile_path.to_str().unwrap()));
        }
    }

    let emakefile = emake::loader::load_file(&secrets_path_info.emakefile_path.to_str().unwrap());

    if let Some(force_type) = maybe_force_type {
        secrets_path_info.target_type = force_type;
    }

    match secrets_path_info.target_type {
        TargetType::Targets => {
            if emakefile.targets.contains_key(&secrets_path_info.target_name) {
                return Ok(Target::TargetEntry(emakefile.targets.get(&secrets_path_info.target_name).unwrap().to_owned()));
            } else {
                return Err(format!("No target named {} found in Emakefile {}", secrets_path_info.target_name, secrets_path_info.emakefile_path.to_str().unwrap()));
            }
        },
        TargetType::Secrets => {
            if let Some(secrets) = emakefile.secrets {
                if secrets.contains_key(&secrets_path_info.target_name) {
                    return Ok(Target::CredentialEntry(secrets.get(&secrets_path_info.target_name).unwrap().to_owned()));
                } else {
                    return Err(format!("No secrets named {} found in Emakefile {}", secrets_path_info.target_name, secrets_path_info.emakefile_path.to_str().unwrap()));
                }
            } else {
                return Err(format!("No secrets defined in Emakefile {}. Expected a credential named {}", secrets_path_info.emakefile_path.to_str().unwrap(), secrets_path_info.target_name));
            }
        },
        TargetType::Variables => {
            if let Some(variables) = emakefile.variables {
                if variables.contains_key(&secrets_path_info.target_name) {
                    return Ok(Target::VariableEntry(variables.get(&secrets_path_info.target_name).unwrap().to_owned()));
                } else {
                    return Err(format!("No variable named {} found in Emakefile {}", secrets_path_info.target_name, secrets_path_info.emakefile_path.to_str().unwrap()));
                }
            } else {
                return Err(format!("No variables defined in Emakefile {}. Expected a variable named {}", secrets_path_info.emakefile_path.to_str().unwrap(), secrets_path_info.target_name));
            }
        }
    }
}

pub fn load_file(root: &str) -> emake::Emakefile {
    let build_file_content = read_file_content(root);
    let mut emakefile: emake::Emakefile = serde_yml::from_str(&build_file_content).unwrap();
    emakefile.path = Some(String::from(root));
    // println!("{:?}", emakefile);
    return emakefile;
}