use std::fmt;
use std::fs::*;
use std::io::Write;
use std::str::FromStr;
use std::{collections::HashMap, unimplemented};

use regex::Regex;

/// Helper macro to assert values in snapshot
#[macro_export]
macro_rules! jassert {
    ($tc:ident, $k:literal, $v: literal) => {
        assert_eq!(
            $tc.json.get($k).unwrap(),
            $v,
            "{}",
            format!("assert error for tag={}", &$tc.tag)
        );
    };
    ($rc:ident, $k:literal) => {
        assert!($rc.1.contains($k));
    };
}

// used to specify debug or release configuration
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum Target {
    debug,
    release,
}

impl Target {
    pub fn path(&self) -> &str {
        match self {
            Target::debug => "./target/debug/clf",
            Target::release => "./target/release/clf",
        }
    }
}

impl FromStr for Target {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debug" => Ok(Target::debug),
            "release" => Ok(Target::release),
            &_ => unimplemented!("unknown Target mode"),
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", Target::debug)
    }
}

// manage YAML configurations
const DEFAULT_CONFIG_FILE: &'static str = "./tests/integration/config/default.yml";
pub struct Config {
    config_file: String,
    yaml: String,
}

impl Default for Config {
    fn default() -> Self {
        // load file data
        let yaml = read_to_string(DEFAULT_CONFIG_FILE).expect(&format!(
            "unable to open default config file: {}",
            &DEFAULT_CONFIG_FILE
        ));
        Config {
            config_file: DEFAULT_CONFIG_FILE.to_string(),
            yaml,
        }
    }
}

impl Config {
    // load file data into a string
    pub fn from_file(path: &str) -> Self {
        // load file data
        let yaml = read_to_string(path)
            .expect(&format!("unable to open alternate config file: {}", &path));
        Config {
            config_file: path.to_string(),
            yaml,
        }
    }

    // modify a field in the YAML data: change its value
    pub fn set_tag(&mut self, tag: &str, value: &str) -> &mut Config {
        // build new re
        let formatted = format!(r"(?m)\b{}:.*$", tag);
        let new_tag = format!("{}: {}", tag, value);
        let re = Regex::new(&formatted).unwrap();

        // save replaced text
        self.yaml = re.replace(&self.yaml, new_tag.as_str()).to_string();

        self
    }

    // change both tag and value
    pub fn replace_tag(&mut self, tag: &str, newtag: &str, value: &str) -> &mut Config {
        // build new re
        let formatted = format!(r"(?m)\b{}:.*$", tag);
        let new_tag = format!("{}: {}", newtag, value);
        let re = Regex::new(&formatted).unwrap();

        // save replaced text
        self.yaml = re.replace(&self.yaml, new_tag.as_str()).to_string();

        self
    }

    // save modified data to another file
    pub fn save_as(&self, name: &str) {
        let mut file = File::create(name).expect(&format!("unable to save config file: {}", name));
        writeln!(&mut file, "{}", self.yaml).unwrap();
    }
}

// a struct for holding test case details
pub struct TestCase {
    pub tag: String,
    pub snap_file: String,
    pub config_file: String,
    pub json: HashMap<String, String>,
}

impl TestCase {
    // a new testcase with the default config
    pub fn new(tag: &str) -> Self {
        println!("running test case: {}", tag);

        TestCase {
            tag: tag.to_string(),
            snap_file: format!("./tests/integration/tmp/{}.json", tag),
            config_file: format!("./tests/integration/tmp/{}.yml", tag),
            json: HashMap::new(),
        }
    }

    // call CLF executable with optional arguments
    pub fn run(&mut self, target: &Target, optargs: &[&str]) -> (i32, String) {
        let clf = target.path();

        let output = std::process::Command::new(clf)
            .args(&[
                "-c",
                &self.config_file,
                "-p",
                &self.snap_file,
                "-g",
                "Trace",
            ])
            .args(optargs)
            .output()
            .expect("unable to start clf");

        let s = String::from_utf8_lossy(&output.stdout);

        // load json as hashmap
        self.json = self.json(&self.snap_file);
        //println!("{:?}", &self.json);

        (output.status.code().unwrap(), s.to_string())
    }

    // call CLF executable with arguments
    pub fn exec(&self, target: &Target, args: &[&str]) -> i32 {
        let clf = target.path();
        let mut output = std::process::Command::new(clf)
            .args(args)
            .output()
            .expect("unable to start clf");
        output.status.code().unwrap()
    }

    // extract JSON data
    fn json(&self, name: &str) -> HashMap<String, String> {
        let data =
            read_to_string(&name).expect(&format!("unable to open snapshot file: {}", &name));
        let re = Regex::new(r#""([\w_]+)":([^{]+?)(,?)$"#).unwrap();
        let mut hmap = HashMap::new();

        for s in data.lines() {
            if re.is_match(s) {
                let caps = re.captures(s).unwrap();
                hmap.insert(
                    caps.get(1).unwrap().as_str().trim().to_string(),
                    caps.get(2).unwrap().as_str().trim().replace("\"", ""),
                );
            }
        }

        hmap
    }
}
