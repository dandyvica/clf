use std::fmt;
use std::fs::*;
use std::io::{BufWriter, Write};
use std::process::Command;
use std::str::FromStr;
use std::{collections::HashMap, unimplemented};

use rand::{thread_rng, Rng};
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
const DEFAULT_CONFIG_FILE: &'static str = "./tests/integration/config/generated.yml";
#[allow(dead_code)]
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
    #[allow(dead_code)]
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
        //println!("{:?}", output);

        let s = String::from_utf8_lossy(&output.stdout);

        // load json as hashmap
        self.json = self.json(&self.snap_file);

        (output.status.code().unwrap(), s.to_string())
    }

    // call CLF executable with arguments
    pub fn exec(&self, target: &Target, args: &[&str]) -> i32 {
        let clf = target.path();
        let output = Command::new(clf)
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

// manage creation or growth of a fake logfile
const FAKE_LOGFILE: &'static str = "./tests/integration/logfiles/generated.log";
const FAKE_LOGFILE_UTF8: &'static str = "./tests/integration/logfiles/generated_utf8.log";
const FAKE_LOGFILE_GZIP: &'static str = "./tests/integration/logfiles/generated.log.gz";

pub struct FakeLogfile;

impl FakeLogfile {
    fn _create(append: bool) {
        // open file in write or append mode
        let log = if append {
            OpenOptions::new()
                .append(true)
                .open(FAKE_LOGFILE)
                .expect("unable to create fake logfile")
        } else {
            File::create(FAKE_LOGFILE).expect("unable to create fake logfile")
        };
        let mut writer = BufWriter::new(&log);

        // initialize random seed
        let mut rng = thread_rng();

        // now write into our fake logfile
        let mut line_number = 0;

        for _ in 1..102 {
            line_number += 1;

            // insert ok pattern once
            if line_number == 51 {
                writeln!(
                    &mut writer,
                    "1970-01-01 00:00:00: ############# this is a fake ok pattern generated for tests, line number = {:03}",
                    line_number
                )
                .unwrap();
                continue;
            }

            // otherwise add error and warning lines
            let error_id: u32 = rng.gen_range(10000..=99999);
            writeln!(
                &mut writer,
                "1970-01-01 00:00:00: ---- this is an error generated for tests, line number = {:03}, error id = {}",
                line_number, error_id
            ).unwrap();

            let warning_id: u32 = rng.gen_range(10000..=99999);
            line_number += 1;

            writeln!(
                &mut writer,
                "1970-01-01 00:00:00: * this is a warning generated for tests, line number = {:03}, warning id = {}",
                line_number, warning_id
            )
            .unwrap();
        }
    }

    // create a log with japanese utf8 chars
    pub fn create_utf8() {
        // open file in write or append mode
        let log = File::create(FAKE_LOGFILE_UTF8).expect("unable to create fake logfile");
        let mut writer = BufWriter::new(&log);

        // initialize random seed
        let mut rng = thread_rng();

        // now write into our fake logfile
        let mut line_number = 0;

        for _ in 1..102 {
            line_number += 1;

            // insert ok pattern once
            if line_number == 51 {
                writeln!(
                    &mut writer,
                    "1970-01-01 00:00:00: ############# これはテスト用に生成された偽の OK パターンで、行番号 = {:03} です。",
                    line_number
                )
                .unwrap();
                continue;
            }

            // otherwise add error and warning lines
            let error_id: u32 = rng.gen_range(10000..=99999);
            writeln!(
                &mut writer,
                "1970-01-01 00:00:00: ---- これはテストに対して生成されたエラーで、行番号 = {:03}, エラー ID = {} です。",
                line_number, error_id
            ).unwrap();

            let warning_id: u32 = rng.gen_range(10000..=99999);
            line_number += 1;

            writeln!(
                &mut writer,
                "1970-01-01 00:00:00: * これはテストに対して生成された警告で、行番号 = {:03}、警告 ID = {} です。",
                line_number, warning_id
            )
            .unwrap();
        }
    }

    // simulate file growth
    pub fn init() {
        FakeLogfile::_create(false);
    }

    // simulate file growth
    pub fn grow() {
        FakeLogfile::_create(true);
    }

    // simulate file rotation
    pub fn rotate() {
        let output = Command::new("gzip")
            .args(&[FAKE_LOGFILE])
            .output()
            .expect("unable to gzip dummy logfile");

        if output.status.code().unwrap() != 0 {
            println!("error compressing dummy logfile");
        }

        // regenerate a new logfile
        FakeLogfile::init();

        // wait a little before calling
        let ten_millis = std::time::Duration::from_millis(500);
        std::thread::sleep(ten_millis);        
    }

    // simulate gzip
    pub fn gzip(keep: bool) {
        let args = if keep {
            vec!["-k", FAKE_LOGFILE]
        } else {
            vec![FAKE_LOGFILE]
        };

        let _ = Command::new("gzip")
            .args(&args)
            .output()
            .expect("unable to gzip dummy logfile");
    }

    // delete gzipped logfile
    pub fn gzip_delete() {
        std::fs::remove_file(FAKE_LOGFILE_GZIP).expect("unable to delete gzip fake logfile");
    }
}

// This is made for testing using a UDS
use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct JSONStream {
    pub args: Vec<String>,
    pub global: HashMap<String, String>,
    pub vars: HashMap<String, String>,
}

// utility fn to receive JSON from a stream
impl JSONStream {
    pub fn get_json_from_stream<T: std::io::Read>(socket: &mut T) -> std::io::Result<JSONStream> {
        // try to read size first
        let mut size_buffer = [0; std::mem::size_of::<u16>()];

        let bytes_read = socket.read(&mut size_buffer)?;
        if bytes_read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "socket closed",
            ));
        }

        let json_size = u16::from_be_bytes(size_buffer);

        // read JSON raw data
        let mut json_buffer = vec![0; json_size as usize];
        socket.read_exact(&mut json_buffer).unwrap();

        // get JSON
        let s = std::str::from_utf8(&json_buffer).unwrap();
        let json: JSONStream = serde_json::from_str(&s).unwrap();
        Ok(json)
    }
}
