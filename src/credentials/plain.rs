use std::collections::HashMap;

use crate::{console::log, credentials::PlainCredentials};

use super::Credentials;
pub static ID: &str = "plain";

pub struct Plain;

const USERNAME_KEY: &str = "username";
const PASSWORD_KEY: &str = "password";


impl Credentials for Plain {
    fn extract<'a>(
        &'a self,
        _cwd: &'a str,
        credential: &'a HashMap<String, serde_yml::Value>,
    ) -> PlainCredentials {
        if !credential.contains_key(USERNAME_KEY) {
            log::error!("Plain credential must contains a key named {}", USERNAME_KEY);
        }

        let mut password: Option<String> = None;
        if credential.contains_key(PASSWORD_KEY) {
            password = Some(String::from(credential.get(PASSWORD_KEY).unwrap().as_str().unwrap()));
        }

        PlainCredentials {
            username: String::from(credential.get(USERNAME_KEY).unwrap().as_str().unwrap()),
            password,
        }
    }
    
    fn clone_box(&self) -> Box<dyn Credentials + Send + Sync> {
        Box::new(Self)
    }
}
