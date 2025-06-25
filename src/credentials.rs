    use std::{collections::HashMap, future::Future, pin::Pin};

    use serde_yml::Value;

    mod plain;

    pub trait Credentials {
        fn extract<'a>(
            &'a self,
            key: &'a str,
            cwd: &'a str,
            emakefile_cwd: &'a str);
    }

    pub struct CredentialsStore {
        credentials: HashMap<String, Credentials>
    }

    impl CredentialsStore {
        pub fn add(mut self, key: &String, credentials: Credentials) -> CredentialsStore {
            self.credentials.insert(key.clone(), credentials);
            self
        }

        pub fn get(&self, credentials_id: &String) -> Option<&Credentials> {
            self.credentials.get(action_id)
        }
    }

    pub fn instanciate() -> CredentialsStore {
        CredentialsStore {
            credentials: HashMap::new(),
        }
        .add(&String::from(plain::ID), plain::Plain)
    }
