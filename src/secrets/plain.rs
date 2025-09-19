use std::collections::HashMap;
use config_macros::SecretDoc;
use crate::{console::log, secrets::PlainSecret};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use super::Secrets;

pub static ID: &str = "plain";

#[derive(SecretDoc)]
#[secret_doc(
    id = "plain",
    short_desc = "Plain secret, use only for dev purpose",
    example = "
secrets:
  my_deep_secret:
    type: plain
    secret: my_secret_in_base64
"
)]
pub struct Plain;

const SECRET_KEY: &str = "secret";


impl Secrets for Plain {
    fn extract<'a>(
        &'a self,
        _cwd: &'a str,
        unextracted_secrets: &'a HashMap<String, serde_yml::Value>,
    ) -> PlainSecret {
        if !unextracted_secrets.contains_key(SECRET_KEY) {
            log::error!("Plain secret must contains a key named {}", SECRET_KEY);
        }

        String::from_utf8(STANDARD.decode(unextracted_secrets.get(SECRET_KEY).unwrap().as_str().unwrap()).unwrap()).unwrap()
    }
    
    fn clone_box(&self) -> Box<dyn Secrets + Send + Sync> {
        Box::new(Self)
    }
}
