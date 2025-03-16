use std::{collections::HashMap, path::Path};

use glob::glob;
use regex::Regex;

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

fn extract_function_args(element: &str, function: &str) -> Vec<String> {
    let mut args = String::from(element);
    args.pop();

    let mut fn_called = String::from(function);
    fn_called.push_str("(");
    args = args.replace(&fn_called, "");

    args.split(',').map(|e| { e.replace('"', "").replace("'", "")}).collect()
}

fn call_function(cwd: &Path, element: &str, maybe_replacements: Option<&HashMap<&str, &str>>) -> Option<String> {
    let mut replaced_element = String::from(element);
    if let Some(replacements) = maybe_replacements {
        let re = Regex::new(r"\$\{([^}]+?)\}").unwrap();
        replaced_element = re.replace_all(element, |caps: &regex::Captures| {
            return replacements.get(&caps[1].trim()).unwrap_or(&&caps[0]).to_string();
        }).to_string();
    }

    let glob_re: Regex = Regex::new(r####"["|']{0,1}\s*glob(.[^)])\s*["|']{0,1}"####).unwrap();
    if glob_re.is_match(&replaced_element) {
        let args = extract_function_args(&replaced_element, "glob");
        return call_glob(cwd, &args[0]);
    }

    return None;
}

pub fn compile(cwd: &str, content: &str, maybe_replacements: Option<&HashMap<&str, &str>>) -> String {
    let current_dir = std::path::Path::new(cwd);
    let re = Regex::new(r"\{\{(.*?)\}\}").unwrap();
    let result = re.replace_all(content, |caps: &regex::Captures| {
        if let Some(result_function) =  call_function(current_dir, &caps[1].trim(), maybe_replacements) {
            return result_function;
        }

        if let Some(replacements) = maybe_replacements {
            return replacements.get(&caps[1].trim()).unwrap_or(&&caps[0]).to_string();
        }

        caps[0].to_string()
    });

    // println!("RESULT {:?}", result);

    result.to_string()
}

