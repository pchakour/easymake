use serde_yml::Value;
use std::{
    collections::HashMap, future::Future, io::{BufRead, BufReader}, pin::Pin, process::{Command, Stdio}, sync::{Arc, Mutex}
};

use crate::{console::log, emake};

use super::Credentials;
pub static ID: &str = "plain";

pub struct Plain;

impl Credentials for Plain {
    fn credentials<'a>(
        &'a self,
        cwd: &'a str,
        _emakefile_cwd: &'a str,        
    ) -> String {
        String::from("TRY it");
    }
}
