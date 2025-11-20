use std::{collections::HashMap, path::Path};

use glob::glob;
use regex::Regex;

use crate::{
    console::log,
    emake::{
        self,
        loader::{get_target_on_path, Target, TargetType},
    },
    CREDENTIALS_STORE,
};

#[derive(PartialEq)]
enum TOKEN_STATE {
    UNKNOWN,
    PIPE,
    ARG_STRING_OPEN,
    ARG_STRING_CLOSE,
    ARG_PATH,
    OPEN_VAR,
    CLOSE_VAR,
    OPEN_STRING,
    CLOSE_STRING,
    PATH,
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

const ESCAPE_CHAR: u8 = b'\\';

const FUNCTIONS: [&str; 9] = [
    "glob",
    "get_secret",
    "values_of",
    "keys_of",
    "array_to_shell",
    "prepend_text",
    "append_text",
    "append_in_array",
    "prepend_in_array",
];

fn parse_array_or_string(input: &str) -> Result<Vec<String>, String> {
    // Try JSON first
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(input) {
        match value {
            serde_json::Value::Array(arr) => {
                let mut out = Vec::new();
                for el in arr {
                    match el {
                        serde_json::Value::String(s) => out.push(s),
                        _ => return Err("Array contains non-string elements".into()),
                    }
                }
                return Ok(out);
            }
            serde_json::Value::String(s) => {
                return Ok(vec![s]);
            }
            _ => return Err("Expected a string or array of strings".into()),
        }
    }

    // If input is NOT JSON, treat it as a raw string
    Ok(vec![input.to_string()])
}

fn call_glob(
    pipe_in: &mut String,
    cwd: &str,
    _maybe_replacements: Option<&HashMap<String, String>>,
) {
    match parse_array_or_string(pipe_in) {
        Ok(patterns) => {
            let mut paths = Vec::<String>::new();

            for pattern in patterns {
                let absolute_pattern = Path::new(cwd).join(&pattern);

                for entry in glob(&absolute_pattern.to_string_lossy())
                    .unwrap_or_else(|_| panic!("Failed to read glob pattern {}", pattern))
                {
                    match entry {
                        Ok(path) => {
                            paths.push(path.to_string_lossy().into_owned());
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
            }

            *pipe_in = stringify_variable_value(&paths);
        }
        Err(error) => {
            log::panic!(
                "Helper glob can't parse pipe in as json [pipe_in={}, error={}]",
                pipe_in,
                error
            );
        }
    }
}

fn call_concat_array(pipe_in: &mut String, text_or_array: &str, prepend: bool) {
    match serde_json::from_str::<Vec<String>>(pipe_in) {
        Ok(parsed) => match parse_array_or_string(text_or_array) {
            Ok(values) => {
                let mut result;
                if prepend {
                    result = values.clone();
                    result.extend(parsed);
                } else {
                    result = parsed.clone();
                    result.extend(values);
                }

                *pipe_in = serde_json::to_string(&result).unwrap_or_else(|e| {
                    log::panic!(
                        "Error when concat pipe_in [pipe_in={:?}, args={:?}, serde_err={}]",
                        pipe_in,
                        text_or_array,
                        e
                    );
                });
            }
            Err(error) => {
                log::panic!(
                "Helper prepend_in_array and append_in_array can't parse your argument [args={}, pipe_in={}, error={}]",
                text_or_array,
                pipe_in,
                error
            );
            }
        },
        Err(error) => {
            log::panic!(
                "Helper prepend_in_array and append_in_array can't parse pipe in as json, expected an array as pipe_in [pipe_in={}, error={}]",
                pipe_in,
                error
            );
        }
    }
}

fn call_concat_text(pipe_in: &mut String, text: &str, prepend: bool) {
    match parse_array_or_string(pipe_in.as_str()) {
        Ok(mut parsed) => {
            // append text to each element in-place
            for s in parsed.iter_mut() {
                if prepend {
                    *s = String::from(text) + s;
                } else {
                    s.push_str(text);
                }
            }

            *pipe_in = serde_json::to_string(&parsed).unwrap_or_else(|e| {
                log::panic!(
                    "Error when concat pipe_in [pipe_in={:?}, args={:?}, serde_err={}]",
                    pipe_in,
                    text,
                    e
                );
            });
        }
        Err(error) => {
            log::panic!(
                "Helper concat can't parse pipe in as json [pipe_in={}, error={}]",
                pipe_in,
                error
            );
        }
    }
}

fn resolve_secret(
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
    pipe_in: &mut String,
    cwd: &str,
    emakefile_current_path: &str,
    maybe_replacements: Option<&HashMap<String, String>>,
) {
    let resolved =
        get_user_variable(pipe_in, cwd, emakefile_current_path).unwrap_or_else(|_| pipe_in.clone());

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
                *pipe_in = stringify_variable_value(&values);
            }
            _ => {
                log::panic!(
                    "Specified string is not resolved as json object {}",
                    pipe_in
                );
            }
        },
        Err(error) => {
            log::panic!("{}", error);
        }
    }
}

fn resolve_variable(
    value: &String,
    cwd: &str,
    emakefile_current_path: &str,
    maybe_replacements: Option<&HashMap<String, String>>,
) -> String {
    if let Some(replacements) = maybe_replacements {
        if replacements.contains_key(value) {
            return replacements.get(value).unwrap().to_owned();
        }
    }
    
    _resolve_variable(value, value, cwd, emakefile_current_path, 0)
}

fn _resolve_variable(
    value: &String,
    original: &String,
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
            return _resolve_variable(&v, original, cwd, emakefile_current_path, counter);
        }
        Err(err) => {
            if original == value {
                log::panic!("{}", err);
            }
            value.clone()
        }
    }
}

fn call_keys_of(
    pipe_in: &mut String,
    cwd: &str,
    emakefile_current_path: &str,
    maybe_replacements: Option<&HashMap<String, String>>,
) {
    let resolved = get_user_variable(&pipe_in, cwd, emakefile_current_path)
        .unwrap_or_else(|_| pipe_in.clone());

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
                *pipe_in = stringify_variable_value(&keys);
            }
            _ => {
                log::panic!(
                    "Specified string is not resolved as json object {}",
                    pipe_in
                );
            }
        },
        Err(error) => {
            log::panic!("Parsing error on keys_of with value {}: {}", pipe_in, error);
        }
    }
}

fn call_array_to_shell(pipe_in: &mut String) {
    match serde_json::from_str::<Vec<String>>(pipe_in) {
        Ok(parsed) => {
            *pipe_in = parsed.join(" ");
        }
        Err(error) => {
            log::panic!(
                "Helper array_to_shell can't parse pipe in as json [pipe_in={}, error={}]",
                pipe_in,
                error
            );
        }
    }
}

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

fn is_escape(element_chars: &[u8], element_index: usize) -> bool {
    element_index > 0 && element_chars[element_index - 1] == ESCAPE_CHAR
}

fn tokenizer(element: &String) -> Vec<(TOKEN_TAG, String)> {
    let mut ast = Vec::<(TOKEN_TAG, String)>::new();
    let mut current_token = TOKEN_STATE::UNKNOWN;
    let element_chars = element.as_bytes();
    let mut cumulate_token_chars = String::from("");

    for (element_index, element_char) in element_chars.iter().enumerate() {
        match current_token {
            TOKEN_STATE::UNKNOWN => {
                if *element_char == b'/' {
                    current_token = TOKEN_STATE::PATH;
                    cumulate_token_chars += &String::from_utf8(Vec::from([*element_char])).unwrap();
                } else if *element_char == b'{'
                    && element_chars[element_index - 1] == b'$'
                    && !is_escape(element_chars, element_index - 1)
                {
                    current_token = TOKEN_STATE::OPEN_VAR;
                    if cumulate_token_chars.trim().len() > 0 {
                        ast.push((TOKEN_TAG::PATH, cumulate_token_chars.clone()));
                    }
                    cumulate_token_chars.clear();
                } else if (*element_char == b'"' || *element_char == b'\'')
                    && !is_escape(element_chars, element_index)
                {
                    current_token = TOKEN_STATE::OPEN_STRING;
                    if cumulate_token_chars.trim().len() > 0 {
                        ast.push((TOKEN_TAG::PATH, cumulate_token_chars.clone()));
                    }
                    cumulate_token_chars.clear();
                } else if *element_char == b'(' && !is_escape(element_chars, element_index) {
                    current_token = TOKEN_STATE::ARG_PATH;
                    ast.push((TOKEN_TAG::HELPER, cumulate_token_chars.clone()));
                    cumulate_token_chars.clear();
                } else if *element_char == b'|' && !is_escape(element_chars, element_index) {
                    current_token = TOKEN_STATE::PIPE;
                    if cumulate_token_chars.trim().len() > 0 {
                        if FUNCTIONS.contains(&cumulate_token_chars.trim()) {
                            ast.push((TOKEN_TAG::HELPER, cumulate_token_chars.trim().to_string()));
                        } else {
                            ast.push((TOKEN_TAG::VAR, cumulate_token_chars.trim().to_string()));
                        }
                    }
                    cumulate_token_chars.clear();
                } else {
                    cumulate_token_chars += &String::from_utf8(Vec::from([*element_char])).unwrap();
                }
            }
            TOKEN_STATE::PATH => {
                if *element_char == b'|' {
                    current_token = TOKEN_STATE::PIPE;
                    ast.push((TOKEN_TAG::PATH, cumulate_token_chars.trim().to_string()));
                    cumulate_token_chars.clear();
                } else if *element_char == b'('
                    || *element_char == b','
                    || *element_char == b')'
                    || *element_char == b'\''
                    || *element_char == b'"'
                {
                    log::panic!(
                        "Unexpected token [element={},index={}]",
                        element,
                        element_index
                    );
                } else {
                    cumulate_token_chars += &String::from_utf8(Vec::from([*element_char])).unwrap();
                }
            }
            TOKEN_STATE::PIPE => {
                current_token = TOKEN_STATE::UNKNOWN;
                ast.push((TOKEN_TAG::PIPE, String::from("|")));
            }
            TOKEN_STATE::OPEN_VAR => {
                if *element_char == b'}' && !is_escape(element_chars, element_index) {
                    current_token = TOKEN_STATE::CLOSE_VAR;
                    ast.push((TOKEN_TAG::VAR, cumulate_token_chars.clone()));
                    cumulate_token_chars.clear();
                } else {
                    cumulate_token_chars += &String::from_utf8(Vec::from([*element_char])).unwrap();
                }
            }
            TOKEN_STATE::CLOSE_VAR => {
                current_token = TOKEN_STATE::UNKNOWN;
            }
            TOKEN_STATE::OPEN_STRING => {
                if (*element_char == b'"' || *element_char == b'\'')
                    && !is_escape(element_chars, element_index)
                {
                    current_token = TOKEN_STATE::CLOSE_STRING;
                    ast.push((TOKEN_TAG::STRING, cumulate_token_chars.clone()));
                    cumulate_token_chars.clear();
                } else if *element_char == ESCAPE_CHAR && !is_escape(element_chars, element_index) {
                } else {
                    cumulate_token_chars += &String::from_utf8(Vec::from([*element_char])).unwrap();
                }
            }
            TOKEN_STATE::CLOSE_STRING => {
                current_token = TOKEN_STATE::UNKNOWN;
            }
            TOKEN_STATE::ARG_STRING_OPEN => {
                if (*element_char == b'"' || *element_char == b'\'')
                    && !is_escape(element_chars, element_index)
                {
                    current_token = TOKEN_STATE::ARG_STRING_CLOSE;
                    ast.push((TOKEN_TAG::ARG_STRING, cumulate_token_chars.clone()));
                    cumulate_token_chars.clear();
                } else {
                    cumulate_token_chars += &String::from_utf8(Vec::from([*element_char])).unwrap();
                }
            }
            TOKEN_STATE::ARG_STRING_CLOSE => {
                if *element_char == b')' && !is_escape(element_chars, element_index) {
                    current_token = TOKEN_STATE::UNKNOWN;
                } else if *element_char == b',' {
                    current_token = TOKEN_STATE::ARG_PATH;
                } else {
                    current_token = TOKEN_STATE::UNKNOWN;
                }
            }
            TOKEN_STATE::ARG_PATH => {
                if (*element_char == b'"' || *element_char == b'\'')
                    && !is_escape(element_chars, element_index)
                {
                    current_token = TOKEN_STATE::ARG_STRING_OPEN;
                } else if *element_char == b')' && !is_escape(element_chars, element_index) {
                    current_token = TOKEN_STATE::UNKNOWN;
                    ast.push((TOKEN_TAG::ARG_PATH, cumulate_token_chars.clone()));
                    cumulate_token_chars.clear();
                } else if *element_char == b',' && !is_escape(element_chars, element_index) {
                    ast.push((TOKEN_TAG::ARG_PATH, cumulate_token_chars.clone()));
                    cumulate_token_chars.clear();
                } else {
                    cumulate_token_chars += &String::from_utf8(Vec::from([*element_char])).unwrap();
                }
            }
        }
    }

    if cumulate_token_chars.trim().len() > 0 {
        if current_token == TOKEN_STATE::PATH {
            ast.push((TOKEN_TAG::PATH, cumulate_token_chars.trim().to_string()));
        } else if FUNCTIONS.contains(&cumulate_token_chars.as_str()) {
            ast.push((TOKEN_TAG::HELPER, cumulate_token_chars.clone()));
        } else {
            ast.push((TOKEN_TAG::VAR, cumulate_token_chars.clone()));
        }
    }
    cumulate_token_chars.clear();

    ast
}

fn execute_helper(
    helper: &(TOKEN_TAG, String),
    args: &Vec<(TOKEN_TAG, String)>,
    tokens: &Vec<(TOKEN_TAG, String)>,
    pipe_in: &mut String,
    cwd: &str,
    emakefile_current_path: &str,
    maybe_replacements: Option<&HashMap<String, String>>,
) {
    // Call the helper
    if helper.1 == String::from("glob") {
        if args.len() > 0 {
            log::panic!(
                "Helper glob doesn't expected arguments [args={:?}, tokens={:?}]",
                args,
                tokens
            );
        }
        call_glob(pipe_in, cwd, maybe_replacements);
    } else if helper.1 == String::from("array_to_shell") {
        if args.len() > 0 {
            log::panic!(
                "Helper array_to_shell doesn't expected arguments [args={:?}, tokens={:?}]",
                args,
                tokens
            );
        }
        call_array_to_shell(pipe_in);
    } else if helper.1 == String::from("values_of") {
        if args.len() > 0 {
            log::panic!(
                "Helper values_of doesn't expected arguments [args={:?}, tokens={:?}]",
                args,
                tokens
            );
        }

        call_values_of(pipe_in, cwd, emakefile_current_path, maybe_replacements);
    } else if helper.1 == String::from("keys_of") {
        if args.len() > 0 {
            log::panic!(
                "Helper keys_of doesn't expected arguments [args={:?}, tokens={:?}]",
                args,
                tokens
            );
        }

        call_keys_of(pipe_in, cwd, emakefile_current_path, maybe_replacements);
    } else if helper.1 == String::from("prepend_text") {
        if args.len() != 1 {
            log::panic!(
                "Helper prepend_text expect one argument [args={:?}, tokens={:?}]",
                args,
                tokens
            );
        }

        call_concat_text(pipe_in, &args[0].1, true);
    } else if helper.1 == String::from("append_text") {
        if args.len() != 1 {
            log::panic!(
                "Helper append_text expect one argument [args={:?}, tokens={:?}]",
                args,
                tokens
            );
        }

        call_concat_text(pipe_in, &args[0].1, false);
    } else if helper.1 == String::from("prepend_in_array") {
        if args.len() != 1 {
            log::panic!(
                "Helper prepend_in_array expect one argument [args={:?}, tokens={:?}]",
                args,
                tokens
            );
        }

        call_concat_array(pipe_in, &args[0].1, true);
    } else if helper.1 == String::from("append_in_array") {
        if args.len() != 1 {
            log::panic!(
                "Helper append_in_array expect one argument [args={:?}, tokens={:?}]",
                args,
                tokens
            );
        }

        call_concat_array(pipe_in, &args[0].1, false);
    } else {
        log::panic!("Unknown helper {} in template language", helper.1);
    }
}

fn template_executor(
    tokens: &Vec<(TOKEN_TAG, String)>,
    cwd: &str,
    emakefile_current_path: &str,
    maybe_replacements: Option<&HashMap<String, String>>,
) -> String {
    let mut context = (TOKEN_TAG::UNKNOWN, String::from(""));
    let mut args = Vec::<(TOKEN_TAG, String)>::new();
    let mut pipe_in = String::from("");

    for current_token in tokens {
        if context.0 == TOKEN_TAG::HELPER {
            if current_token.0 != TOKEN_TAG::ARG_PATH && current_token.0 != TOKEN_TAG::ARG_STRING {
                execute_helper(
                    &context,
                    &args,
                    tokens,
                    &mut pipe_in,
                    &cwd,
                    emakefile_current_path,
                    maybe_replacements,
                );
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

                let mut current_str = current_token.1.clone();
                // Replace non user variables
                if let Some(replacements) = maybe_replacements {
                    let var_re = Regex::new(r"\$\{([^}]+)\}").unwrap();
                    current_str = var_re
                        .replace_all(&current_str, |var_caps: &regex::Captures| {
                            let mut result = replacements
                                .get(&var_caps[1].trim().to_string())
                                .unwrap_or(&&var_caps[0].to_string())
                                .to_string();

                            result = resolve_variable(
                                &var_caps[1].trim().to_string(),
                                cwd,
                                emakefile_current_path,
                                maybe_replacements
                            );

                            result
                        })
                        .to_string();

                    if replacements.contains_key(&current_str) {
                        current_str = replacements.get(&current_str).unwrap().to_string();
                    }
                }

                args.push((TOKEN_TAG::ARG_STRING, current_str.clone()));
            }
            TOKEN_TAG::HELPER => {
                context = current_token.clone();
            }
            TOKEN_TAG::PATH => {
                let current_str = &current_token.1;

                let target_path =
                    get_target_on_path(cwd, current_str, &emakefile_current_path, None);
                match target_path.unwrap_or_else(|error| {
                    log::panic!("Can't resolve path {}: {}", current_str, error);
                }) {
                    Target::TargetEntry(_) => {
                        log::panic!("You can not use target inside template language as path. Only variables and secrets are accepted");
                    }
                    Target::SecretEntry(_) => {
                        pipe_in = resolve_secret(cwd, emakefile_current_path, current_str)
                            .unwrap_or(current_str.clone());
                    }
                    Target::VariableEntry(_) => {
                        pipe_in = resolve_variable(
                            &current_str.trim().to_string(),
                            cwd,
                            emakefile_current_path,
                            maybe_replacements
                        );
                    }
                }
            }
            TOKEN_TAG::STRING => {
                let mut current_str = current_token.1.clone();
                // Replace non user variables
                if let Some(replacements) = maybe_replacements {
                    let var_re = Regex::new(r"\$\{([^}]+)\}").unwrap();
                    current_str = var_re
                        .replace_all(&current_str, |var_caps: &regex::Captures| {
                            let mut result = replacements
                                .get(&var_caps[1].trim().to_string())
                                .unwrap_or(&&var_caps[0].to_string())
                                .to_string();

                            result = resolve_variable(
                                &var_caps[1].trim().to_string(),
                                cwd,
                                emakefile_current_path,
                                maybe_replacements
                            );

                            result
                        })
                        .to_string();

                    if replacements.contains_key(&current_str) {
                        current_str = replacements.get(&current_str).unwrap().to_string();
                    }
                }

                pipe_in += &current_str;
            }
            TOKEN_TAG::VAR => {
                // Replace non user variables
                if let Some(replacements) = maybe_replacements {
                    if replacements.contains_key(&current_token.1) {
                        pipe_in = replacements.get(&current_token.1).unwrap().to_owned();
                    }
                }
            }
            TOKEN_TAG::UNKNOWN => {
                log::panic!("Unknown token detected {:?}", current_token);
            }
            _ => {}
        }
    }

    if context.0 == TOKEN_TAG::HELPER {
        // Call the helper
        execute_helper(
            &context,
            &args,
            tokens,
            &mut pipe_in,
            cwd,
            emakefile_current_path,
            maybe_replacements,
        );
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
        let element = String::from(caps[1].trim());
        let tokens = tokenizer(&element);
        log::debug!("TOKENS {:?}", tokens);
        template_executor(&tokens, cwd, emakefile_current_path, maybe_replacements)
    });

    log::debug!("Template compilation result from {} to {}", content, result);

    result.to_string()
}
