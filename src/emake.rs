use config_macros::DocType;
use serde::{Deserialize, Deserializer, Serialize};
use serde_yml::{Value};
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
    Shell { shell: shell::ShellAction },
    Copy { copy: copy::CopyAction },
    Extract { extract: extract::ExtractAction },
    #[serde(rename = "move")]
    Move { mv: mv::MoveAction },
    Remove { remove: remove::RemoveAction },
    Archive { archive: archive::ArchiveSpec },
    GitClone { git_clone: git_clone::GitCloneAction },
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
        let raw: serde_yml::Mapping = Deserialize::deserialize(deserializer)?;
        let mut description = None;
        let mut plugin_map = serde_yml::Mapping::new();

        for (k, v) in &raw {
            if let Some(key) = k.as_str() {
                match key {
                    "description" => description = Some(String::deserialize(v.clone()).map_err(serde::de::Error::custom)?),
                    _ => {
                        plugin_map.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        let action: PluginAction = serde_yml::from_value(Value::Mapping(plugin_map))
            .map_err(|e| serde::de::Error::custom(format!(
                "Step deserialization failed (expected fields: description and action fields): {}",
                e
            )))?;

        Ok(Step {
            description: description.unwrap_or_default(),
            action,
        })
    }
}
