use std::{collections::HashMap, path::Path};

use glob::glob;
use regex::Regex;
use serde_yml::modules::path;

fn call_glob(cwd: &Path, pattern: &String) -> Option<String> {
    let absolute_pattern = cwd.join(pattern);
    println!("Path {:?}", absolute_pattern);
    let mut paths: Vec<String> = Vec::new();
    for entry in glob(&absolute_pattern.to_string_lossy()).expect(&format!("Failed to read glob pattern {}", pattern)) {
        match entry {
            Ok(path) => {
                let current_dir = cwd.to_str().unwrap().replace("./", "") + "/";
                let test = String::from(path.to_string_lossy()).replace(&current_dir, "");
                paths.push(test);
            },
            Err(e) => println!("{:?}", e),
        }
    }

    let result = format!("[{}]", paths.join(", "));
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

fn call_function(cwd: &Path, element: &str) -> Option<String> {
    let glob_re: Regex = Regex::new(r####"["|']{0,1}\s*glob(.[^)])\s*["|']{0,1}"####).unwrap();
    if glob_re.is_match(element) {
        let args = extract_function_args(element, "glob");
        return call_glob(cwd, &args[0]);
    }

    return None;
}

pub fn compile(cwd: &str, content: &str, maybe_replacements: Option<&HashMap<&str, &str>>) -> String {
    let current_dir = std::path::Path::new(cwd);
    let re = Regex::new(r"\{\{(.*?)\}\}").unwrap();
    let result = re.replace_all(content, |caps: &regex::Captures| {
        if let Some(result_function) =  call_function(current_dir, &caps[1].trim()) {
            return result_function;
        }

        if let Some(replacements) = maybe_replacements {
            return replacements.get(&caps[1].trim()).unwrap_or(&&caps[0]).to_string();
        }

        caps[0].to_string()
    });

    println!("Maybe_replacements {:?}", maybe_replacements);
    println!("Compile content {}", content);
    println!("Compile result {}", result);
    result.to_string()
}

