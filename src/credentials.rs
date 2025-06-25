use std::{collections::HashMap};

mod plain;

pub struct PlainCredentials {
    pub username: String,
    pub password: Option<String>,
}

pub trait Credentials: Send + Sync {
    fn extract<'a>(
        &'a self,
    ) -> PlainCredentials;
    fn clone_box(&self) -> Box<dyn Credentials + Send + Sync>;
}

impl Clone for Box<dyn Credentials + Send + Sync> {
    fn clone(&self) -> Box<dyn Credentials + Send + Sync> {
        self.clone_box()
    }
}

pub struct CredentialsStore {
    credentials: HashMap<String, Box<dyn Credentials + Send + Sync>>
}

impl CredentialsStore {
    pub fn add(mut self, key: &String, action: Box<dyn Credentials + Send + Sync>) -> CredentialsStore {
        self.credentials.insert(key.clone(), action);
        self
    }

    pub fn get(&self, credentials_key: &String) -> Option<&Box<dyn Credentials + Send + Sync>> {
        self.credentials.get(credentials_key)
    }
}

pub fn instanciate() -> CredentialsStore {
    CredentialsStore {
        credentials: HashMap::new(),
    }
    .add(&String::from(plain::ID), Box::new(plain::Plain))
}
