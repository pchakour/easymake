use config_macros::DocType;
use serde::{Deserialize, Deserializer, Serialize};
use serde_yml::Value;
use std::collections::HashMap;

use crate::actions::{archive, copy, extract, git_clone, mv, remove, shell};

pub mod compiler;
pub mod loader;

pub type SecretEntry = HashMap<String, Value>;
pub type VariableEntry = String;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Target {
    pub deps: Option<Vec<String>>,
    pub parallel: Option<bool>,
    pub steps: Option<Vec<Step>>,
}

#[derive(DocType, Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
#[doc_type(
    short_desc = "An input file definition",
    description = "\
Type: String | { file: String, credentials: String }
An input file can be a local file or a file from an url.
If you need to specify credentials to get an url file, you can use the field file and credentials.

**Note**

If you use the variable in_files inside the shell action to target an url file, the value will be automatically replaced by
the donwloaded path. 

**Examples**

```yaml
{% raw %}
secrets:
    my_deep_secret:
      type: plain
      username: My_username
      password: My_password

targets:
    getting_local_file:
        steps:
            - description: Getting a local file
              shell:
                in_files:
                  - \"{{ EMAKE_CWD_DIR }}/path_to_my_local_path\"
                cmd: ls {{ in_files[0] }} # or ls {{ in_files }}
    getting_from_url:
        steps:
            - description: Getting from url
              shell:
                in_files:
                  - https://github.com/pchakour/easymake/archive/refs/heads/main.zip
                cmd: ls {{ in_files }}
    getting_from_url_with_credentials:
        steps:
            - description: Getting from url with credentials
              shell:
                in_files:
                  - file: https://github.com/pchakour/easymake/archive/refs/heads/main.zip  
                    credentials: {{ secrets:my_deep_secret }}
                cmd: ls {{ in_files }}
{% endraw %}
```
"
)]
pub enum InFile {
    Simple(String),
    Detailed {
        file: String,
        credentials: Option<Credentials>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Credentials {
    pub username: String,
    pub password: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
// #[serde(deny_unknown_fields)]
pub struct Step {
    pub description: String,
    #[serde(flatten)]
    pub action: PluginAction, // The actual action like cmd/copy
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged, deny_unknown_fields)]
pub enum PluginAction {
    Shell {
        shell: shell::ShellAction,
    },
    Copy {
        copy: copy::CopyAction,
    },
    Extract {
        extract: extract::ExtractAction,
    },
    #[serde(rename = "move")]
    Move {
        mv: mv::MoveAction,
    },
    Remove {
        remove: remove::RemoveAction,
    },
    Archive {
        archive: archive::ArchiveAction,
    },
    GitClone {
        git_clone: git_clone::GitCloneAction,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Emakefile {
    pub path: Option<String>,
    pub secrets: Option<HashMap<String, SecretEntry>>,
    pub variables: Option<HashMap<String, VariableEntry>>,
    pub targets: HashMap<String, Target>,
}

impl<'de> Deserialize<'de> for Step {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        fn set_action<D: serde::de::Error>(
            action: &mut Option<PluginAction>,
            plugin_action: PluginAction,
        ) -> Result<(), D> {
            if action.is_some() {
                return Err(D::custom(
                    "You can't provide several actions in a step, create a new step",
                ));
            }
            *action = Some(plugin_action);
            Ok(())
        }

        let raw: serde_yml::Mapping = Deserialize::deserialize(deserializer)?;

        let mut description: Option<String> = None;
        let mut action: Option<PluginAction> = None;

        for (k, v) in &raw {
            let key = k
                .as_str()
                .ok_or_else(|| serde::de::Error::custom("Step key must be a string"))?;
            match key {
                "description" => {
                    description =
                        Some(String::deserialize(v.clone()).map_err(serde::de::Error::custom)?);
                }
                key if key == shell::ID => {
                    let deserialized_action: shell::ShellAction =
                        serde_yml::from_value(v.clone()).map_err(serde::de::Error::custom)?;

                    set_action(
                        &mut action,
                        PluginAction::Shell {
                            shell: deserialized_action,
                        },
                    )?;
                }
                key if key == git_clone::ID => {
                    let deserialized_action: git_clone::GitCloneAction =
                        serde_yml::from_value(v.clone()).map_err(serde::de::Error::custom)?;
                    set_action(
                        &mut action,
                        PluginAction::GitClone {
                            git_clone: deserialized_action,
                        },
                    )?;
                }
                key if key == archive::ID => {
                    let deserialized_action: archive::ArchiveAction =
                        serde_yml::from_value(v.clone()).map_err(serde::de::Error::custom)?;
                    set_action(
                        &mut action,
                        PluginAction::Archive {
                            archive: deserialized_action,
                        },
                    )?;
                }
                key if key == extract::ID => {
                    let deserialized_action: extract::ExtractAction =
                        serde_yml::from_value(v.clone()).map_err(serde::de::Error::custom)?;
                    set_action(
                        &mut action,
                        PluginAction::Extract {
                            extract: deserialized_action,
                        },
                    )?;
                }
                key if key == mv::ID => {
                    let deserialized_action: mv::MoveAction =
                        serde_yml::from_value(v.clone()).map_err(serde::de::Error::custom)?;
                    set_action(
                        &mut action,
                        PluginAction::Move {
                            mv: deserialized_action,
                        },
                    )?;
                }
                key if key == remove::ID => {
                    let deserialized_action: remove::RemoveAction =
                        serde_yml::from_value(v.clone()).map_err(serde::de::Error::custom)?;
                    set_action(
                        &mut action,
                        PluginAction::Remove {
                            remove: deserialized_action,
                        },
                    )?;
                }
                key if key == copy::ID => {
                    let deserialized_action: copy::CopyAction =
                        serde_yml::from_value(v.clone()).map_err(serde::de::Error::custom)?;
                    set_action(
                        &mut action,
                        PluginAction::Copy {
                            copy: deserialized_action,
                        },
                    )?;
                }
                // Add other actions: copy, extract, move, remove...
                _ => {
                    return Err(serde::de::Error::custom(format!(
                        "Unknown key `{}` (expected field description or an action)",
                        key
                    )));
                }
            }
        }

        let description = description.unwrap_or_default();
        let action = action.ok_or_else(|| {
            serde::de::Error::custom(format!(
                "No action found in step. Expected one of: {:?}",
                [
                    shell::ID,
                    git_clone::ID,
                    archive::ID,
                    copy::ID,
                    extract::ID,
                    mv::ID,
                    remove::ID
                ]
            ))
        })?;

        Ok(Step {
            description,
            action,
        })
    }
}
