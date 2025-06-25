use crate::{credentials::PlainCredentials};

use super::Credentials;
pub static ID: &str = "plain";

pub struct Plain;

impl Credentials for Plain {
    fn extract<'a>(
        &'a self,
    ) -> PlainCredentials {
        PlainCredentials { username: String::from("username"), password: Some(String::from("password")) }
    }
    
    fn clone_box(&self) -> Box<dyn Credentials + Send + Sync> {
        Box::new(Self)
    }
}
