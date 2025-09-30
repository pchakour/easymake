use std::{collections::HashMap};
mod plain;
mod keyring;

pub type PlainSecret = String;

pub trait Secrets: Send + Sync {
    fn extract<'a>(
        &'a self,
        cwd: &'a str,
        unextracted_secrets: &'a HashMap<String, serde_yml::Value>,
    ) -> PlainSecret;
    fn clone_box(&self) -> Box<dyn Secrets + Send + Sync>;
}

impl Clone for Box<dyn Secrets + Send + Sync> {
    fn clone(&self) -> Box<dyn Secrets + Send + Sync> {
        self.clone_box()
    }
}

pub struct SecretsStore {
    secrets: HashMap<String, Box<dyn Secrets + Send + Sync>>
}

impl SecretsStore {
    pub fn add(mut self, key: &String, action: Box<dyn Secrets + Send + Sync>) -> SecretsStore {
        self.secrets.insert(key.clone(), action);
        self
    }

    pub fn get(&self, secrets_key: &String) -> Option<&Box<dyn Secrets + Send + Sync>> {
        self.secrets.get(secrets_key)
    }
}

pub fn instanciate() -> SecretsStore {
    SecretsStore {
        secrets: HashMap::new(),
    }
    .add(&String::from(plain::ID), Box::new(plain::Plain))
    .add(&String::from(keyring::ID), Box::new(keyring::Keyring))
}
