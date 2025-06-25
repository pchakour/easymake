    use std::{collections::HashMap, future::Future, pin::Pin};

    use serde_yml::Value;

    mod cmd;
    mod target;

    pub trait Action: Send + Sync {
        fn action<'a>(
            &'a self,
            cwd: &'a str,
            emakefile_cwd: &'a str,
            silent: bool,
            args: &'a Value,
            in_files: &'a Vec<String>,
            out_file: &'a Vec<String>,
            working_dir: &'a String,
            default_replacments: Option<&'a HashMap<String, String>>) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
        fn clone_box(&self) -> Box<dyn Action + Send + Sync>;
    }

    impl Clone for Box<dyn Action + Send + Sync> {
        fn clone(&self) -> Box<dyn Action + Send + Sync> {
            self.clone_box()
        }
    }

    pub struct ActionsStore {
        actions: HashMap<String, Box<dyn Action + Send + Sync>>
    }

    impl ActionsStore {
        pub fn add(mut self, key: &String, action: Box<dyn Action + Send + Sync>) -> ActionsStore {
            self.actions.insert(key.clone(), action);
            self
        }

        pub fn get(&self, action_id: &String) -> Option<&Box<dyn Action + Send + Sync>> {
            self.actions.get(action_id)
        }
    }

    pub fn instanciate() -> ActionsStore {
        ActionsStore {
            actions: HashMap::new(),
        }
        .add(&String::from(cmd::ID), Box::new(cmd::Cmd))
        .add(&String::from(target::ID), Box::new(target::Target))
    }
