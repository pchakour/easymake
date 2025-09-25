use crate::{console::log, emake::{self, SecretEntry, VariableEntry}};
use serde_yml;
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
    SecretEntry(SecretEntry),
}

fn read_file_content(path: &str) -> String {
    // log::info!("Loading file {:?}", path);
    let content = std::fs::read_to_string(path).unwrap();
    return content;
}

fn target_type_str_to_enum(target_type: &str) -> Result<TargetType, String> {
    if target_type == "targets" {
        return Ok(TargetType::Targets);
    } else if target_type == "secrets" {
        return Ok(TargetType::Secrets);
    } else if target_type == "variables" {
        return Ok(TargetType::Variables);
    }

    Err(format!("Unknown target type {}", target_type))
}

pub fn extract_info_from_path(
    path: &str,
    cwd: &str,
    emakefile_current_path: &str,
) -> Result<PathInfo, String> {
    let mut target_split: Vec<&str> = path.split('/').collect();
    target_split.retain(|s| !s.is_empty());
    let from_root = path.starts_with("//");
    let maybe_real_target: Option<&str> = target_split.pop();

    if let Some(real_target) = maybe_real_target {
        let real_target_split: Vec<&str> = real_target.split(':').collect();
        let mut target_type = TargetType::Targets;
        let mut target_name = real_target.to_string();

        if real_target_split.len() == 2 {
            let target_type_result = target_type_str_to_enum(real_target_split[0]);
            if target_type_result.is_ok() {
                target_type = target_type_result?;
                target_name = String::from(real_target_split[1]);
            } else {
                return Err(target_type_result.err().unwrap());
            }
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

        return Ok(PathInfo {
            emakefile_path,
            target_type,
            target_name,
        });
    }

    Err(format!("Malformed target path {}", path))
}

pub fn get_target_on_path(
    cwd: &str,
    secrets_path: &str,
    emakefile_current_path: &str,
    maybe_force_type: Option<TargetType>,
) -> Result<Target, String> {
    // Check if the target exists in the current Emakefile
    let secrets_path_info_result = extract_info_from_path(secrets_path, cwd, emakefile_current_path);
    let mut secrets_path_info = secrets_path_info_result?;

    if let Ok(emakefile_exists) = std::fs::exists(&secrets_path_info.emakefile_path) {
        if !emakefile_exists {
            return Err(format!(
                "Emakefile {} doesn't exist",
                secrets_path_info.emakefile_path.to_str().unwrap()
            ));
        }
    }

    let emakefile = emake::loader::load_file(&secrets_path_info.emakefile_path.to_str().unwrap());

    if let Some(force_type) = maybe_force_type {
        secrets_path_info.target_type = force_type;
    }

    match secrets_path_info.target_type {
        TargetType::Targets => {
            if emakefile
                .targets
                .contains_key(&secrets_path_info.target_name)
            {
                return Ok(Target::TargetEntry(
                    emakefile
                        .targets
                        .get(&secrets_path_info.target_name)
                        .unwrap()
                        .to_owned(),
                ));
            } else {
                return Err(format!(
                    "No target named {} found in Emakefile {}",
                    secrets_path_info.target_name,
                    secrets_path_info.emakefile_path.to_str().unwrap()
                ));
            }
        }
        TargetType::Secrets => {
            if let Some(secrets) = emakefile.secrets {
                if secrets.contains_key(&secrets_path_info.target_name) {
                    return Ok(Target::SecretEntry(
                        secrets
                            .get(&secrets_path_info.target_name)
                            .unwrap()
                            .to_owned(),
                    ));
                } else {
                    return Err(format!(
                        "No secrets named {} found in Emakefile {}",
                        secrets_path_info.target_name,
                        secrets_path_info.emakefile_path.to_str().unwrap()
                    ));
                }
            } else {
                return Err(format!(
                    "No secrets defined in Emakefile {}. Expected a credential named {}",
                    secrets_path_info.emakefile_path.to_str().unwrap(),
                    secrets_path_info.target_name
                ));
            }
        }
        TargetType::Variables => {
            if let Some(variables) = emakefile.variables {
                if variables.contains_key(&secrets_path_info.target_name) {
                    return Ok(Target::VariableEntry(
                        variables
                            .get(&secrets_path_info.target_name)
                            .unwrap()
                            .to_owned(),
                    ));
                } else {
                    return Err(format!(
                        "No variable named {} found in Emakefile {}",
                        secrets_path_info.target_name,
                        secrets_path_info.emakefile_path.to_str().unwrap()
                    ));
                }
            } else {
                return Err(format!(
                    "No variables defined in Emakefile {}. Expected a variable named {}",
                    secrets_path_info.emakefile_path.to_str().unwrap(),
                    secrets_path_info.target_name
                ));
            }
        }
    }
}

pub fn load_file(root: &str) -> emake::Emakefile {
    let build_file_content = read_file_content(root);
    let emakefile_result = serde_yml::from_str(&build_file_content);

    if emakefile_result.is_err() {
        log::panic!("An error occured when loading Emakefile {}: \n\n{:?}", root, emakefile_result.as_ref().err());
    }

    let mut emakefile: emake::Emakefile = emakefile_result.unwrap();
    emakefile.path = Some(String::from(root));
    // println!("{:?}", emakefile);
    return emakefile;
}
