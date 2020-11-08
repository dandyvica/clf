use std::io::Result;
use std::process::{Command, ExitStatus};

// path to our clf
const CLF_DEBUG: &str = "target/debug/clf";

// a macro for defining a new config
#[macro_export]
macro_rules! config_file {
    ($file:literal, $yaml:literal) => {{
        let yaml = $yaml;

        let _ = std::fs::create_dir("tests/tmp/");
        let config_file = concat!("tests/tmp/", $file, ".yml");
        //config_file.push("config_test.yml");

        //let _ = std::fs::remove_file(&config_file);

        std::fs::write(&config_file, yaml).expect("Unable to write file");
        config_file
    }};
}

// A unit struct to bundle utility functions
pub struct Util;

impl Util {
    // check exit code
    pub fn exit_status(cli_args: &[&str]) -> Result<i32> {
        Command::new(CLF_DEBUG)
            .args(cli_args)
            .output()
            .map(|x| x.status.code().unwrap())
    }

    // output as a vector of strings
    pub fn output(cli_args: &[&str]) -> Result<(i32, Vec<u8>)> {
        Command::new(CLF_DEBUG)
            .args(cli_args)
            .output()
            .map(|x| (x.status.code().unwrap(), x.stdout))
    }

    // get output as a Vec<String>
    pub fn to_vec(buf: &Vec<u8>) -> Vec<String> {
        let s = String::from_utf8_lossy(buf);
        s.lines().map(|x| x.to_string()).collect()
    }
}
