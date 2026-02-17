use config_macros::ActionDoc;
use serde::{Deserialize, Serialize};
use serde_yml::Value;
use std::{
    collections::HashMap,
    fs,
    future::Future,
    pin::Pin,
};

use crate::{
    console::log,
    emake::{self, InFile, PluginAction},
};

use super::Action;
pub static ID: &str = "yaml";

#[derive(ActionDoc, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[action_doc(
    id = "yaml",
    short_desc = "Create or edit yaml files",
    example = "
targets:
    yaml:
        steps:
            - description: Testing yaml action
              yaml:
                to: \"{{ EMAKE_OUT_DIR }}/yaml_action.yml\"
                set:
                    version: 0.1
                    name: yaml
                    type: action 
"
)]
pub struct YamlAction {
    #[action_prop(
        description = "The path to the yaml file to edit. Not mandatory if the parameter `to` is specified",
        required = false
    )]
    pub from: Option<String>,
    #[action_prop(
        description = "Specify a path to save the yaml file. Not mandatory if the parameter `from` is specified",
        required = false
    )]
    pub to: Option<String>,
    #[action_prop(
        description = "Value to set inside the file. Use null if you want delete the key",
        required = true
    )]
    pub set: Value,
}

pub struct Yaml;

fn merge_yaml(
    base: &mut Value,
    update: &Value,
    emakefile_current_path: &str,
    maybe_replacements: Option<&HashMap<String, String>>,
) {
    match (base, update) {
        // Both sides are mappings → deep merge
        (Value::Mapping(base_map), Value::Mapping(update_map)) => {
            for (k, v) in update_map {
                // DELETE LOGIC: if update has "key: null", delete from base
                if v.is_null() {
                    base_map.remove(k);
                    continue;
                }

                // Otherwise merge recursively or replace
                merge_yaml(
                    base_map.entry(k.clone()).or_insert(Value::Null),
                    v,
                    emakefile_current_path,
                    maybe_replacements,
                );
            }
        }

        // Everything else: replace
        (base_value, update_value) => match update_value {
            serde_yml::Value::String(update_value_string) => {
                *base_value = serde_yml::from_str(&emake::compiler::compile(
                    &update_value_string,
                    emakefile_current_path,
                    maybe_replacements,
                    None,
                ))
                .unwrap();
            }
            _ => {
                *base_value = update_value.clone();
            }
        },
    }
}

impl Action for Yaml {
    fn insert_in_files<'a>(
        &'a self,
        action: &'a PluginAction,
        in_files: &'a mut Vec<InFile>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::Yaml { yaml } => {
                    if let Some(from) = &yaml.from {
                        in_files.push(InFile::Simple(from.clone()));
                    }
                }
                _ => {}
            }
        })
    }

    fn insert_out_files<'a>(
        &'a self,
        action: &'a PluginAction,
        out_files: &'a mut Vec<String>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::Yaml { yaml } => {
                    if let Some(to) = &yaml.to {
                        out_files.push(to.clone());
                    }
                }
                _ => {}
            }
        })
    }

    fn run<'a>(
        &'a self,
        _target_id: &'a str,
        step_id: &'a str,
        emakefile_cwd: &'a str,
        _silent: bool,
        action: &'a PluginAction,
        in_files: &'a Vec<String>,
        out_files: &'a Vec<String>,
        _working_dir: &'a String,
        maybe_replacements: Option<&'a HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>> {
        Box::pin(async move {
            match action {
                PluginAction::Yaml { yaml } => {
                    if yaml.from.is_none() && yaml.to.is_none() {
                        log::panic!(
                            "When using yaml action, you must define at least from or to property"
                        );
                    }

                    if yaml.from.is_some() {
                        let from = &in_files[0];
                        let file_content = fs::read(from).unwrap();
                        let file_content_string = String::from_utf8(file_content).unwrap();
                        let mut from_yaml =
                            serde_yml::from_str::<serde_yml::Value>(&file_content_string).unwrap();
                        merge_yaml(
                            &mut from_yaml,
                            &yaml.set,
                            emakefile_cwd,
                            maybe_replacements,
                        );

                        let mut to = from;
                        if yaml.to.is_some() {
                            to = &out_files[0];
                        }

                        fs::write(to, serde_yml::to_string(&from_yaml).unwrap()).unwrap();
                    } else if yaml.to.is_some() {
                        let to = &out_files[0];
                        let mut to_yaml =
                            serde_yml::from_str::<serde_yml::Value>("").unwrap();
                        merge_yaml(&mut to_yaml, &yaml.set, emakefile_cwd, maybe_replacements);
                        fs::write(to, serde_yml::to_string(&to_yaml).unwrap()).unwrap();
                    }
                }
                _ => {}
            }

            Ok(())
        })
    }
    fn clone_box(&self) -> Box<dyn Action + Send + Sync> {
        Box::new(Self)
    }
    fn get_checksum(&self) -> Option<String> {
        None
    }
}
