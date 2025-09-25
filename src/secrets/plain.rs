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
            log::panic!("Plain secret must contains a key named {}", SECRET_KEY);
        }

        let secret = unextracted_secrets.get(SECRET_KEY).unwrap().as_str().unwrap();
        let decoded_secret_result = STANDARD.decode(secret);

        match decoded_secret_result {
            Ok(decoded_secret) => {
                let plain_secret_result = String::from_utf8(decoded_secret);
                if plain_secret_result.is_err() {
                    log::panic!("Error when decoding your base64 secret {}", secret);
                }

                plain_secret_result.unwrap()
            },
            _ => {
                log::panic!("Error when decoding your base64 secret {}", secret);
            }
        }
    }
    
    fn clone_box(&self) -> Box<dyn Secrets + Send + Sync> {
        Box::new(Self)
    }
}
