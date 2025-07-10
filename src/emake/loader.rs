use serde_yml;
use crate::emake::{self, CredentialEntry, TargetEntry, VariableEntry};
use crate::console::log;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum TargetType {
    Credentials,
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
    } else if target_type == "credentials" {
        return TargetType::Credentials;
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
    credentials_path: &str,
    emakefile_current_path: &str,
    maybe_force_type: Option<TargetType>
) -> Result<Target, String> {
    // Check if the target exists in the current Emakefile
    let mut credentials_path_info = extract_info_from_path(credentials_path, cwd, emakefile_current_path);

    if let Ok(emakefile_exists) = std::fs::exists(&credentials_path_info.emakefile_path) {
        if !emakefile_exists {
            return Err(format!("Emakefile {} doesn't exist", credentials_path_info.emakefile_path.to_str().unwrap()));
        }
    }

    let emakefile = emake::loader::load_file(&credentials_path_info.emakefile_path.to_str().unwrap());

    if let Some(force_type) = maybe_force_type {
        credentials_path_info.target_type = force_type;
    }

    match credentials_path_info.target_type {
        TargetType::Targets => {
            if emakefile.targets.contains_key(&credentials_path_info.target_name) {
                return Ok(Target::TargetEntry(emakefile.targets.get(&credentials_path_info.target_name).unwrap().to_owned()));
            } else {
                return Err(format!("No target named {} found in Emakefile {}", credentials_path_info.target_name, credentials_path_info.emakefile_path.to_str().unwrap()));
            }
        },
        TargetType::Credentials => {
            if let Some(credentials) = emakefile.credentials {
                if credentials.contains_key(&credentials_path_info.target_name) {
                    return Ok(Target::CredentialEntry(credentials.get(&credentials_path_info.target_name).unwrap().to_owned()));
                } else {
                    return Err(format!("No credentials named {} found in Emakefile {}", credentials_path_info.target_name, credentials_path_info.emakefile_path.to_str().unwrap()));
                }
            } else {
                return Err(format!("No credentials defined in Emakefile {}. Expected a credential named {}", credentials_path_info.emakefile_path.to_str().unwrap(), credentials_path_info.target_name));
            }
        },
        TargetType::Variables => {
            if let Some(variables) = emakefile.variables {
                if variables.contains_key(&credentials_path_info.target_name) {
                    return Ok(Target::VariableEntry(variables.get(&credentials_path_info.target_name).unwrap().to_owned()));
                } else {
                    return Err(format!("No variable named {} found in Emakefile {}", credentials_path_info.target_name, credentials_path_info.emakefile_path.to_str().unwrap()));
                }
            } else {
                return Err(format!("No variables defined in Emakefile {}. Expected a variable named {}", credentials_path_info.emakefile_path.to_str().unwrap(), credentials_path_info.target_name));
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