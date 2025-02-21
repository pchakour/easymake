use serde_yml::Value;
use std::{collections::HashMap, process::Command};

use crate::emake;

use super::Plugin;
pub static ID: &str = "cmd";

pub struct Cmd;

impl Plugin for Cmd {
    fn action(&self, cwd: &String, args: &Value, in_files: &Vec<String>, out_files: &Vec<String>, working_dir: &String) -> () {
        println!("Run command {:?}", args);
        let mut command = args.as_str().unwrap_or("").to_string();
        let in_files_string = in_files.join(" ");
        let out_files_string = out_files.join("");

        let replacements = HashMap::from([
            ("in_files", in_files_string.as_str()),
            ("out_files", out_files_string.as_str()),
            ("EMAKE_WORKING_DIR", working_dir)
        ]);

        println!("Replacements {:?}", replacements);
        command = emake::compiler::compile(cwd, &command, Some(&replacements));
        let mut shell = "sh";
        let mut arg = "-c";

        if cfg!(target_os = "windows") {
            shell = "cmd";
            arg = "/C";
        }

        let output = Command::new(shell)
            .current_dir(cwd)
            .arg(arg) // Pass the command string to the shell
            .arg(command)
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stederr = String::from_utf8_lossy(&output.stderr);
        println!("Command output {}\n{}", stdout, stederr);
    }

    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync> {
        Box::new(Self)
    }
}
