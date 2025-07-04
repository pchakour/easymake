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
struct PathInfo {
    emakefile_path: PathBuf,
    target_type: TargetType,
    target_name: String,
}

pub enum Target {
    TargetEntry(Vec<TargetEntry>),
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

fn extract_info_from_path(path: &String, cwd: &Path, emakefile_current_path: &String) -> PathInfo {
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

        let mut emakefile_path = cwd.join("Emakefile");
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

fn extract_info_from_path2(path: &String, cwd: &Path) -> PathInfo {
    println!("extract_info_from_path2 {} {:?}", path, cwd);
    let tmp = path.replace("//", (String::from(cwd.to_str().unwrap()) + "/").as_str());
    let mut parts: Vec<&str> = tmp.split('/').collect();
    let target_parts: Vec<&str> = parts.pop().unwrap().split(':').collect();

    let target_type: TargetType;
    let target_name: String;
    
    if target_parts.len() == 2 {
        target_type = target_type_str_to_enum(target_parts[0]);
        target_name = String::from(target_parts[1]);
    } else {
        target_type = TargetType::Targets;
        target_name = String::from(target_parts[0]);
    }

    let emakefile_path = PathBuf::from(parts.join("/")).join("Emakefile");

    return PathInfo {
        emakefile_path,
        target_type,
        target_name
    }
}

pub fn get_target_name(target_path: &String) -> String {
    let mut parts: Vec<&str> = target_path.split(":").collect();
    parts.pop().unwrap().to_string()
}

pub fn get_target_on_path(
    cwd: &Path,
    credentials_path: &String,
    emakefile_current_path: &String,
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

pub fn get_target(cwd: &Path, target: &String, emakefile: &mut emake::Emakefile) -> Vec<TargetEntry> {
    // Check if the target exists in the current Emakefile
    let emakefile_current_path = emakefile.path.to_owned().unwrap();
    let target_path_info = extract_info_from_path(target, cwd, &emakefile_current_path);
    println!("TAEREE {:?} {} {:?}", cwd, target, emakefile.path);

    *emakefile = emake::loader::load_file(&target_path_info.emakefile_path.to_str().unwrap());

    if emakefile.targets.contains_key(&target_path_info.target_name) {
        return emakefile.targets.get(&target_path_info.target_name).unwrap().to_owned();
    } else {
        log::error!("No target {} found", target);
        std::process::exit(1);
    }
}

pub fn load_file(root: &str) -> emake::Emakefile {
    let build_file_content = read_file_content(root);
    let mut emakefile: emake::Emakefile = serde_yml::from_str(&build_file_content).unwrap();
    emakefile.path = Some(String::from(root));
    // println!("{:?}", emakefile);
    return emakefile;
}