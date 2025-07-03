use std::{collections::HashMap, path::{Path}};

use glob::glob;
use regex::Regex;

use crate::{console::log, emake::{self, loader::{Target, TargetType}}, CREDENTIALS_STORE};

fn call_glob(cwd: &Path, pattern: &String) -> Option<String> {
    let absolute_pattern = cwd.join(pattern);
    let mut paths: Vec<String> = Vec::new();
    for entry in glob(&absolute_pattern.to_string_lossy()).expect(&format!("Failed to read glob pattern {}", pattern)) {
        match entry {
            Ok(path) => {
                // let current_dir = cwd.to_str().unwrap().replace("./", "") + "/";
                // let test = String::from(path.to_string_lossy()).replace(&current_dir, "");
                paths.push(String::from(path.to_string_lossy()));
            },
            Err(e) => println!("{:?}", e),
        }
    }

    let result = format!("[{}]", paths.join(", "));
    // println!("Result {}", result);
    Some(result)
}

fn call_credential_password(cwd: &Path, emakefile_current_path: &String, credential_name: &String) -> Option<String> {
    let result_credentials_config = emake::loader::get_target_on_path(
        cwd, 
        credential_name, 
        emakefile_current_path,
        Some(TargetType::Credentials),
    );

    match result_credentials_config {
        Ok(credential_config) => {
            match credential_config {
                Target::CredentialEntry(raw_credential) => {
                    let credential_type = String::from(raw_credential.get("type").unwrap().as_str().unwrap());
                    let maybe_credential_plugin = CREDENTIALS_STORE.get(&credential_type);
                    match maybe_credential_plugin {
                        Some(credential_plugin) => {
                            return credential_plugin.extract(cwd.to_str().unwrap(), &raw_credential).password;
                        },
                        None => {
                            log::error!("Type {} doesn't exist for credential {}", credential_type, credential_name);
                            std::process::exit(1);
                        }
                    }
                }
                Target::TargetEntry(_) => None,
                Target::VariableEntry(_) => None,
            }
        },
        Err(error) => {
            log::error!("{}", error);
            std::process::exit(1);
        }
    }
}

fn call_credential_username(cwd: &Path, emakefile_current_path: &String, credential_name: &String) -> Option<String> {
    let result_credentials_config = emake::loader::get_target_on_path(
        cwd, 
        credential_name, 
        emakefile_current_path,
        Some(TargetType::Credentials),
    );

    match result_credentials_config {
        Ok(credential_config) => {
            match credential_config {
                Target::CredentialEntry(raw_credential) => {
                    let credential_type = String::from(raw_credential.get("type").unwrap().as_str().unwrap());
                    let maybe_credential_plugin = CREDENTIALS_STORE.get(&credential_type);
                    match maybe_credential_plugin {
                        Some(credential_plugin) => {
                            return Some(credential_plugin.extract(cwd.to_str().unwrap(), &raw_credential).username);
                        },
                        None => {
                            log::error!("Type {} doesn't exist for credential {}", credential_type, credential_name);
                            std::process::exit(1);
                        }
                    }
                }
                Target::TargetEntry(_) => None,
                Target::VariableEntry(_) => None,
            }
        },
        Err(error) => {
            log::error!("{}", error);
            std::process::exit(1);
        }
    }
}

fn extract_function_args(element: &str, function: &str) -> Vec<String> {
    let mut args = String::from(element);
    args.pop();

    let mut fn_called = String::from(function);
    fn_called.push_str("(");
    args = args.replace(&fn_called, "");

    args.split(',').map(|e| { e.replace('"', "").replace("'", "")}).collect()
}

fn call_function(cwd: &Path, emakefile_current_path: &String, element: &str) -> Option<String> {
    let glob_re: Regex = Regex::new(r####"["|']{0,1}\s*glob(.[^)])\s*["|']{0,1}"####).unwrap();
    if glob_re.is_match(&element) {
        let args = extract_function_args(&element, "glob");
        return call_glob(cwd, &args[0]);
    }

    let credential_password_re: Regex = Regex::new(r####"["|']{0,1}\s*credential_password(.[^)])\s*["|']{0,1}"####).unwrap();
    if credential_password_re.is_match(&element) {
        let args = extract_function_args(&element, "credential_password");
        return call_credential_password(cwd, emakefile_current_path, &args[0]);
    }

    let credential_username_re: Regex = Regex::new(r####"["|']{0,1}\s*credential_username(.[^)])\s*["|']{0,1}"####).unwrap();
    if credential_username_re.is_match(&element) {
        let args = extract_function_args(&element, "credential_username");
        return call_credential_username(cwd, emakefile_current_path, &args[0]);
    }

    return None;
}

fn get_user_variable(user_variable: &String, cwd: &Path, emakefile_current_path: &String) -> Result<String, String> {
    let result_target = emake::loader::get_target_on_path(
        cwd, 
        user_variable, 
        emakefile_current_path, 
        Some(TargetType::Variables)
    );

    if let Ok(target) = result_target {
        match target {
            Target::VariableEntry(variable) => {
                return Ok(variable);
            }
            Target::TargetEntry(_) => {},
            Target::CredentialEntry(_) => {},
        }
    }
    
    Err(format!("Variable {} not found", user_variable))
}

pub fn compile(
    cwd: &str,
    content: &str,
    emakefile_current_path: &String,
    maybe_replacements: Option<&HashMap<String, String>>
) -> String {
    let current_dir = std::path::Path::new(cwd);
    let re = Regex::new(r"\{\{(.*?)\}\}").unwrap();
    let result = re.replace_all(content, |caps: &regex::Captures| {
        let mut element = String::from(caps[1].trim());

        // Replace user variables inside ${}
        let var_re = Regex::new(r"\$\{([^}]+)\}").unwrap();
        element = var_re.replace_all(&element, |var_caps: &regex::Captures| {
            let result_variable = get_user_variable(
                &var_caps[1].trim().to_string(), 
                current_dir,
                emakefile_current_path
            );

            match result_variable {
                Ok(variable) => {
                    return variable;
                },
                Err(error) => {
                    let mut throw_error = true;
                    if let Some(replacements) = maybe_replacements {
                        if replacements.contains_key(var_caps[1].trim()) {
                            throw_error = false;
                        }
                    }

                    if throw_error {
                        log::error!("{}", error);
                        std::process::exit(1);
                    } else {
                        return var_caps[0].to_string();
                    }
                }
            }
        }).to_string();

        let result_variable = get_user_variable(
            &element.trim().to_string(), 
            current_dir,
            emakefile_current_path
        );

        if let Ok(variable) = result_variable {
            element = variable;
        }

        // Replace non user variables
        if let Some(replacements) = maybe_replacements {
            let var_re = Regex::new(r"\$\{([^}]+)\}").unwrap();
            element = var_re.replace_all(&element, |var_caps: &regex::Captures| {
                return replacements.get(&var_caps[1].trim().to_string()).unwrap_or(&&var_caps[0].to_string()).to_string();
            }).to_string();

            if replacements.contains_key(element.as_str()) {
                element = replacements.get(element.as_str()).unwrap().to_string();
            }
        }

        // Call functions
        if let Some(result_function) =  call_function(current_dir, emakefile_current_path, &element) {
            element = result_function;
        }

        
        element
    });

    // println!("RESULT {:?}", result);

    result.to_string()
}

