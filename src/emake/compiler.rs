use std::{collections::HashMap, path::Path, thread::current};

use glob::glob;
use regex::Regex;
use serde_json::Map;
use sha2::digest::typenum::Integer;

use crate::{
    console::log,
    emake::{
        self,
        loader::{Target, TargetType},
    },
    CREDENTIALS_STORE,
};

#[derive(PartialEq)]
enum TOKEN_STATE {
    PATH_OR_HELPER_NAME,
    PIPE,
    ARG_STRING_OPEN,
    ARG_STRING_CLOSE,
    ARG_PATH,
    OPEN_VAR,
    CLOSE_VAR,
    OPEN_STRING,
    CLOSE_STRING,
    UNKNOWN,
}

#[derive(PartialEq, Debug, Clone)]
enum TOKEN_TAG {
    HELPER,
    PATH,
    ARG_STRING,
    ARG_PATH,
    VAR,
    PIPE,
    STRING,
    UNKNOWN,
}

const ESCAPE_CHAR: char = '\\';

const FUNCTIONS: [&str; 5] = ["glob", "get_secret", "values_of", "keys_of", "array_to_shell"];

fn call_glob(
    cwd: &str,
    pattern: &String,
    maybe_replacements: Option<&HashMap<String, String>>,
) -> String {
    let absolute_pattern = Path::new(cwd).join(pattern);
    let mut paths: Vec<String> = Vec::new();
    for entry in glob(&absolute_pattern.to_string_lossy())
        .expect(&format!("Failed to read glob pattern {}", pattern))
    {
        match entry {
            Ok(path) => {
                // let current_dir = cwd.to_str().unwrap().replace("./", "") + "/";
                // let test = String::from(path.to_string_lossy()).replace(&current_dir, "");
                paths.push(String::from(path.to_string_lossy()));
            }
            Err(e) => {
                log::panic!(
                    "Error when executing glob function with pattern {}: {:?}",
                    pattern,
                    e
                );
            }
        }
    }

    let result = stringify_variable_value(&paths);
    log::debug!("Glob call from pattern {} to {}", pattern, result);
    result
}

fn call_get_secret(
    cwd: &str,
    emakefile_current_path: &str,
    credential_name: &String,
) -> Option<String> {
    let result_secrets_config = emake::loader::get_target_on_path(
        cwd,
        credential_name,
        emakefile_current_path,
        Some(TargetType::Secrets),
    );

    match result_secrets_config {
        Ok(credential_config) => match credential_config {
            Target::SecretEntry(raw_credential) => {
                let credential_type =
                    String::from(raw_credential.get("type").unwrap().as_str().unwrap());
                let maybe_credential_plugin = CREDENTIALS_STORE.get(&credential_type);
                match maybe_credential_plugin {
                    Some(credential_plugin) => {
                        return Some(credential_plugin.extract(cwd, &raw_credential));
                    }
                    None => {
                        log::panic!(
                            "Type {} doesn't exist for credential {}",
                            credential_type,
                            credential_name
                        );
                    }
                }
            }
            Target::TargetEntry(_) => None,
            Target::VariableEntry(_) => None,
        },
        Err(error) => {
            log::panic!("{}", error);
        }
    }
}

fn call_values_of(
    cwd: &str,
    emakefile_current_path: &str,
    map_str_or_map_var: &String,
    maybe_replacements: Option<&HashMap<String, String>>,
) -> Option<String> {
    let resolved = get_user_variable(map_str_or_map_var, cwd, emakefile_current_path)
        .unwrap_or_else(|_| map_str_or_map_var.to_string());

    match serde_json::from_str(&resolved) {
        Ok(parsed) => match parsed {
            serde_json::Value::Object(object) => {
                let values = object
                    .values()
                    .map(|v| {
                        compile(
                            cwd,
                            &stringify_variable_value(v),
                            emakefile_current_path,
                            maybe_replacements,
                        )
                    })
                    .collect::<Vec<String>>();
                Some(stringify_variable_value(&values))
            }
            _ => {
                log::panic!(
                    "Specified string is not resolved as json object {}",
                    map_str_or_map_var
                );
            }
        },
        Err(error) => {
            log::panic!("{}", error);
        }
    }
}

fn variables_resolve(
    value: &String,
    cwd: &str,
    emakefile_current_path: &str,
    counter: i16,
) -> String {
    if counter > 10 {
        log::panic!("Recursive limit for variable resolution is reached: [value={value}]");
    }

    match get_user_variable(value, cwd, emakefile_current_path) {
        Ok(v) => {
            if v == *value {
                return v;
            }
            return variables_resolve(&v, cwd, emakefile_current_path, counter);
        }
        Err(err) => {
            log::panic!("AAAA {:?}", err);
        }
    }
}

fn call_keys_of(
    cwd: &str,
    emakefile_current_path: &str,
    map_str_or_map_var: &String,
    maybe_replacements: Option<&HashMap<String, String>>,
) -> Option<String> {
    let resolved = get_user_variable(map_str_or_map_var, cwd, emakefile_current_path)
        .unwrap_or_else(|_| map_str_or_map_var.to_string());
    get_user_variable(map_str_or_map_var, cwd, emakefile_current_path).unwrap();

    match serde_json::from_str(&resolved) {
        Ok(parsed) => match parsed {
            serde_json::Value::Object(object) => {
                let keys = object
                    .keys()
                    .map(|v| {
                        compile(
                            cwd,
                            &stringify_variable_value(v),
                            emakefile_current_path,
                            maybe_replacements,
                        )
                    })
                    .collect::<Vec<String>>();
                Some(stringify_variable_value(&keys))
            }
            _ => {
                log::panic!(
                    "Specified string is not resolved as json object {}",
                    map_str_or_map_var
                );
            }
        },
        Err(error) => {
            log::panic!(
                "Parsing error on keys_of with value {}: {}",
                map_str_or_map_var,
                error
            );
        }
    }
}

fn extract_function_args(element: &str, function: &str) -> Vec<String> {
    let mut args = String::from(element);
    args.pop();

    let mut fn_called = String::from(function);
    fn_called.push_str("(");
    args = args.replace(&fn_called, "");

    args.split(',')
        .map(|e| e.replace('"', "").replace("'", ""))
        .collect()
}

fn call_array_to_shell(pipe_in: &mut String) {
    match serde_json::from_str::<Vec::<String>>(pipe_in) {
        Ok(parsed) => {
            *pipe_in = parsed.join(" ");
        },
        Err(_) => {
            log::panic!("Helper array_to_shell can't parse pipe in as json [pipe_in={}]", pipe_in);
        }
    }
}

// fn call_function(
//     cwd: &str,
//     emakefile_current_path: &str,
//     element: &str,
//     maybe_replacements: Option<&HashMap<String, String>>,
// ) -> Option<String> {
//     let glob_re: Regex = Regex::new(r####"["|']{0,1}\s*glob(.[^)])\s*["|']{0,1}"####).unwrap();
//     if glob_re.is_match(&element) {
//         let args = extract_function_args(&element, "glob");
//         return call_glob(cwd, &args[0], maybe_replacements);
//     }

//     let get_secret_re: Regex =
//         Regex::new(r####"["|']{0,1}\s*get_secret(.[^)])\s*["|']{0,1}"####).unwrap();
//     if get_secret_re.is_match(&element) {
//         let args = extract_function_args(&element, "get_secret");
//         return call_get_secret(cwd, emakefile_current_path, &args[0]);
//     }

//     let values_of_re: Regex =
//         Regex::new(r####"["|']{0,1}\s*values_of(.[^)])\s*["|']{0,1}"####).unwrap();
//     if values_of_re.is_match(&element) {
//         let args = extract_function_args(&element, "values_of");
//         return call_values_of(cwd, emakefile_current_path, &args[0], maybe_replacements);
//     }

//     let keys_of_re: Regex =
//         Regex::new(r####"["|']{0,1}\s*keys_of(.[^)])\s*["|']{0,1}"####).unwrap();
//     if keys_of_re.is_match(&element) {
//         let args = extract_function_args(&element, "keys_of");
//         return call_keys_of(cwd, emakefile_current_path, &args[0], maybe_replacements);
//     }

//     return None;
// }

fn stringify_variable_value(value: &impl serde::Serialize) -> String {
    let json_value: serde_json::Value = serde_json::to_value(value).unwrap();

    match json_value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".into(),
        other => serde_json::to_string(&other).unwrap(),
    }
}

fn get_user_variable(
    user_variable: &String,
    cwd: &str,
    emakefile_current_path: &str,
) -> Result<String, String> {
    let result_target = emake::loader::get_target_on_path(
        cwd,
        user_variable,
        emakefile_current_path,
        Some(TargetType::Variables),
    );

    if let Ok(target) = result_target {
        match target {
            Target::VariableEntry(variable) => {
                return Ok(stringify_variable_value(&variable));
            }
            Target::TargetEntry(_) => {}
            Target::SecretEntry(_) => {}
        }
    }

    Err(format!("Variable {} not found", user_variable))
}

fn tokenizer(element: &String) -> Vec<(TOKEN_TAG, String)> {
    let mut ast = Vec::<(TOKEN_TAG, String)>::new();
    let mut current_token = TOKEN_STATE::PATH_OR_HELPER_NAME;
    let element_chars = element.as_bytes();
    let mut cumulate_token_chars = String::from("");

    for (element_index, element_char) in element_chars.iter().enumerate() {
        match current_token {
            TOKEN_STATE::PATH_OR_HELPER_NAME => {
                if *element_char == b'{' && element_chars[element_index - 1] == b'$' {
                    current_token = TOKEN_STATE::OPEN_VAR;
                    if cumulate_token_chars.trim().len() > 0 {
                        ast.push((TOKEN_TAG::PATH, cumulate_token_chars.clone()));
                    }
                    cumulate_token_chars.clear();
                } else if *element_char == b'"' || *element_char == b'\'' {
                    current_token = TOKEN_STATE::OPEN_STRING;
                    if cumulate_token_chars.trim().len() > 0 {
                        ast.push((TOKEN_TAG::PATH, cumulate_token_chars.clone()));
                    }
                    cumulate_token_chars.clear();
                } else if *element_char == b'(' {
                    current_token = TOKEN_STATE::ARG_PATH;
                    ast.push((TOKEN_TAG::HELPER, cumulate_token_chars.clone()));
                    cumulate_token_chars.clear();
                } else if *element_char == b'|' {
                    current_token = TOKEN_STATE::PIPE;
                    if cumulate_token_chars.trim().len() > 0 {
                        if FUNCTIONS.contains(&cumulate_token_chars.as_str()) {
                            ast.push((TOKEN_TAG::HELPER, cumulate_token_chars.clone()));
                        } else {
                            ast.push((TOKEN_TAG::PATH, cumulate_token_chars.clone()));
                        }
                    }
                    cumulate_token_chars.clear();
                } else {
                    cumulate_token_chars += &String::from_utf8(Vec::from([*element_char])).unwrap();
                }
            }
            TOKEN_STATE::PIPE => {
                current_token = TOKEN_STATE::PATH_OR_HELPER_NAME;
                ast.push((TOKEN_TAG::PIPE, String::from("|")));
            }
            TOKEN_STATE::OPEN_VAR => {
                if *element_char == b'}' {
                    current_token = TOKEN_STATE::CLOSE_VAR;
                    ast.push((TOKEN_TAG::VAR, cumulate_token_chars.clone()));
                    cumulate_token_chars.clear();
                } else {
                    cumulate_token_chars += &String::from_utf8(Vec::from([*element_char])).unwrap();
                }
            }
            TOKEN_STATE::OPEN_STRING => {
                if *element_char == b'"' || *element_char == b'\'' {
                    current_token = TOKEN_STATE::CLOSE_STRING;
                    ast.push((TOKEN_TAG::STRING, cumulate_token_chars.clone()));
                    cumulate_token_chars.clear();
                } else {
                    cumulate_token_chars += &String::from_utf8(Vec::from([*element_char])).unwrap();
                }
            }
            TOKEN_STATE::ARG_STRING_OPEN => {
                if *element_char == b'"' || *element_char == b'\'' {
                    current_token = TOKEN_STATE::ARG_STRING_CLOSE;
                    ast.push((TOKEN_TAG::ARG_STRING, cumulate_token_chars.clone()));
                    cumulate_token_chars.clear();
                } else {
                    cumulate_token_chars += &String::from_utf8(Vec::from([*element_char])).unwrap();
                }
            }
            TOKEN_STATE::ARG_STRING_CLOSE => {
                if *element_char == b')' {
                    current_token = TOKEN_STATE::PATH_OR_HELPER_NAME;
                } else if *element_char == b',' {
                    current_token = TOKEN_STATE::ARG_PATH;
                } else {
                    current_token = TOKEN_STATE::UNKNOWN;
                }
            }
            TOKEN_STATE::ARG_PATH => {
                if *element_char == b'"' || *element_char == b'\'' {
                    current_token = TOKEN_STATE::ARG_STRING_OPEN;
                } else if *element_char == b')' {
                    current_token = TOKEN_STATE::PATH_OR_HELPER_NAME;
                    ast.push((TOKEN_TAG::ARG_PATH, cumulate_token_chars.clone()));
                    cumulate_token_chars.clear();
                } else if *element_char == b',' {
                    ast.push((TOKEN_TAG::ARG_PATH, cumulate_token_chars.clone()));
                    cumulate_token_chars.clear();
                } else {
                    cumulate_token_chars += &String::from_utf8(Vec::from([*element_char])).unwrap();
                }
            }
            _ => {}
        }
    }

    if current_token == TOKEN_STATE::PATH_OR_HELPER_NAME && cumulate_token_chars.trim().len() > 0 {
        if FUNCTIONS.contains(&cumulate_token_chars.as_str()) {
            ast.push((TOKEN_TAG::HELPER, cumulate_token_chars.clone()));
        } else {
            ast.push((TOKEN_TAG::PATH, cumulate_token_chars.clone()));
        }
    }
    cumulate_token_chars.clear();

    ast
}

fn execute_helper(
    helper: &(TOKEN_TAG, String),
    args: &Vec::<(TOKEN_TAG, String)>,
    tokens: &Vec<(TOKEN_TAG, String)>,
    pipe_in: &mut String,
    cwd: &str,
    maybe_replacements: Option<&HashMap<String, String>>,
) {
    // Call the helper
    if helper.1 == String::from("glob") {
        if args.len() != 1 {
            log::panic!(
                "Glob helper expected one pattern as argument [args={:?}, tokens={:?}]", args,
                tokens
            );
        }
        *pipe_in += &call_glob(cwd, &args[0].1, maybe_replacements);
    } else if helper.1 == String::from("array_to_shell") {
        call_array_to_shell(pipe_in);
    } else {
        log::panic!("Unknown helper {} in template language", helper.1);
    }
}

fn template_executor(
    tokens: &Vec<(TOKEN_TAG, String)>,
    cwd: &str,
    maybe_replacements: Option<&HashMap<String, String>>,
) -> String {
    let mut context = (TOKEN_TAG::UNKNOWN, String::from(""));
    let mut args = Vec::<(TOKEN_TAG, String)>::new();
    let mut pipe_in = String::from("");

    for current_token in tokens {
        if context.0 == TOKEN_TAG::HELPER {
            if current_token.0 != TOKEN_TAG::ARG_PATH && current_token.0 != TOKEN_TAG::ARG_STRING {
                execute_helper(&context, &args, tokens, &mut pipe_in, &cwd, maybe_replacements);
                context = (TOKEN_TAG::UNKNOWN, String::from(""));
                args.clear();
            }
        }

        match current_token.0 {
            TOKEN_TAG::ARG_PATH => {
                if context.0 != TOKEN_TAG::HELPER {
                    log::panic!("Unexpected arg {} in template language", current_token.1);
                }
                args.push(current_token.clone());
            }
            TOKEN_TAG::ARG_STRING => {
                if context.0 != TOKEN_TAG::HELPER {
                    log::panic!("Unexpected arg {} in template language", current_token.1);
                }
                args.push(current_token.clone());
            }
            TOKEN_TAG::HELPER => {
                context = current_token.clone();
            }
            TOKEN_TAG::PIPE => {}
            TOKEN_TAG::VAR => {}
            TOKEN_TAG::PATH => {}
            TOKEN_TAG::STRING => {}
            TOKEN_TAG::UNKNOWN => {}
        }
    }

    if context.0 == TOKEN_TAG::HELPER {
        // Call the helper
        execute_helper(&context, &args, tokens, &mut pipe_in, cwd, maybe_replacements);
    }

    pipe_in
}

pub fn compile(
    cwd: &str,
    content: &str,
    emakefile_current_path: &str,
    maybe_replacements: Option<&HashMap<String, String>>,
) -> String {
    let re = Regex::new(r"\{\{(.*?)\}\}").unwrap();
    let result = re.replace_all(content, |caps: &regex::Captures| {
        let mut element = String::from(caps[1].trim());

        // Replace non user variables
        if let Some(replacements) = maybe_replacements {
            let var_re = Regex::new(r"\$\{([^}]+)\}").unwrap();
            element = var_re
                .replace_all(&element, |var_caps: &regex::Captures| {
                    return replacements
                        .get(&var_caps[1].trim().to_string())
                        .unwrap_or(&&var_caps[0].to_string())
                        .to_string();
                })
                .to_string();

            if replacements.contains_key(element.as_str()) {
                element = replacements.get(element.as_str()).unwrap().to_string();
            }
        }

        let tokens = tokenizer(&element);
        element = template_executor(&tokens, cwd, maybe_replacements);

        // log::panic!("{:?}", tokens);

        // log::panic!("Element {}", element);
        // tokenizer(element);

        // Replace user variables inside ${}
        // let var_re = Regex::new(r"\$\{([^}]+)\}").unwrap();
        // element = var_re.replace_all(&element, |var_caps: &regex::Captures| {
        //     let result_variable = get_user_variable(
        //         &var_caps[1].trim().to_string(),
        //         cwd,
        //         emakefile_current_path
        //     );

        //     match result_variable {
        //         Ok(variable) => {
        //             return variable;
        //         },
        //         Err(error) => {
        //             let mut throw_error = true;
        //             if let Some(replacements) = maybe_replacements {
        //                 if replacements.contains_key(var_caps[1].trim()) {
        //                     throw_error = false;
        //                 }
        //             }
        //             if throw_error {
        //                 log::panic!("{}", error);
        //             } else {
        //                 return var_caps[0].to_string();
        //             }
        //         }
        //     }
        // }).to_string();

        // let result_variable = get_user_variable(
        //     &element.trim().to_string(),
        //     cwd,
        //     emakefile_current_path
        // );

        // if let Ok(variable) = &result_variable {
        //     element = variable.to_owned();
        // }

        // // Replace non user variables
        // if let Some(replacements) = maybe_replacements {
        //     let var_re = Regex::new(r"\$\{([^}]+)\}").unwrap();
        //     element = var_re.replace_all(&element, |var_caps: &regex::Captures| {
        //         return replacements.get(&var_caps[1].trim().to_string()).unwrap_or(&&var_caps[0].to_string()).to_string();
        //     }).to_string();

        //     if replacements.contains_key(element.as_str()) {
        //         element = replacements.get(element.as_str()).unwrap().to_string();
        //     }
        // }

        // // Call functions
        // if let Some(result_function) = call_function(cwd, emakefile_current_path, &element, maybe_replacements) {
        //     element = result_function;
        // }

        // if element == String::from(caps[1].trim()) {
        //     if let Err(error) = &result_variable {
        //        log::panic!("{}", error);
        //     }
        // }

        element
    });

    log::debug!("Template compilation result from {} to {}", content, result);

    result.to_string()
}
