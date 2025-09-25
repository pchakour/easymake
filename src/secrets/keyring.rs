use std::collections::HashMap;
use config_macros::SecretDoc;
use crate::{console::log, secrets::PlainSecret};
use keyring::{Entry};
use super::Secrets;

pub static ID: &str = "keyring";

#[derive(SecretDoc)]
#[secret_doc(
    id = "keyring",
    short_desc = "Get secrets from the local keyring",
    description = "Local keyring storage, see command emake keyring to store or clear password",
    example = "
secrets:
  my_deep_secret:
    type: keyring
    service: service_name
    name: secret_name
"
)]
pub struct Keyring;

const SERVICE_KEY: &str = "service";
const NAME_KEY: &str = "name";


impl Secrets for Keyring {
    fn extract<'a>(
        &'a self,
        _cwd: &'a str,
        unextracted_secrets: &'a HashMap<String, serde_yml::Value>,
    ) -> PlainSecret {
        if !unextracted_secrets.contains_key(NAME_KEY) {
            log::panic!("Keyring secret must contains a key named {}", NAME_KEY);
        }
        if !unextracted_secrets.contains_key(SERVICE_KEY) {
            log::panic!("Keyring secret must contains a key named {}", SERVICE_KEY);
        }
        let service = unextracted_secrets.get(SERVICE_KEY).unwrap().as_str().unwrap();
        let name = unextracted_secrets.get(NAME_KEY).unwrap().as_str().unwrap();

        let keyring_entry = Entry::new(service, name).unwrap();
        keyring_entry.get_password().expect(&format!("Secret entry not found at service {service} with name {name}"))
    }
    
    fn clone_box(&self) -> Box<dyn Secrets + Send + Sync> {
        Box::new(Self)
    }
}
