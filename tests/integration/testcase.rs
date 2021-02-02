#![deny(clippy::all)]
use std::fmt;
use std::fs::*;
use std::io::{BufWriter, Write};
use std::process::Command;
use std::str::FromStr;
use std::{collections::HashMap, unimplemented};

use log::{error, info, trace};
use rand::{thread_rng, Rng};
use regex::Regex;
use simplelog::*;

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

// list of cli options
#[derive(Debug)]
pub struct Options {
    pub mode: Target,
    pub verbose: bool,
    pub clf: String,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            mode: Target::debug,
            verbose: false,
            clf: String::new(),
        }
    }
}

// manage YAML configurations
const DEFAULT_CONFIG_FILE: &str = "./tests/integration/config/generated.yml";
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
    pub logfile: String,
    pub logfile_gzip: String,
    pub tmpfile: String,
}

impl TestCase {
    // a new testcase with the default config
    pub fn new(tag: &str, i: &mut u16) -> Self {
        println!("running test case {:02?}: {}", i, tag);
        *i += 1;
        info!(
            "==============================================> running test case: {}",
            tag
        );

        let tc = TestCase {
            tag: tag.to_string(),
            snap_file: format!("./tests/integration/tmp/{}.json", tag),
            config_file: format!("./tests/integration/tmp/{}.yml", tag),
            json: HashMap::new(),
            logfile: format!("./tests/integration/tmp/{}.log", tag),
            logfile_gzip: format!("./tests/integration/tmp/{}.log.gz", tag),
            tmpfile: format!("./tests/integration/tmp/{}.txt", tag),
        };

        // safe to delete logfile if any
        let _ = std::fs::remove_file(&tc.logfile);
        let _ = std::fs::remove_file(&tc.logfile_gzip);
        let _ = std::fs::remove_file(&tc.snap_file);
        let _ = std::fs::remove_file(&tc.tmpfile);

        // create dummy log file from the tc name
        tc.create_log(None, false);

        tc
    }

    // create a logfile which is specific to the tc
    pub fn create_log(&self, path: Option<&str>, append: bool) {
        match path {
            None => FakeLogfile::create(&self.logfile, append),
            Some(path) => FakeLogfile::create(path, append),
        }
    }

    // create an utf8 logfile which is specific to the tc
    pub fn create_log_utf8(&self) {
        FakeLogfile::create_utf8(&self.logfile);
    }

    // simulate file growth
    pub fn grow(&self) {
        self.create_log(None, true);
    }

    // create multiple logfiles
    pub fn multiple_logs(&self) {
        trace!("creating fake logfiles");
        for i in 1..=10 {
            let logfile = format!("{}.{}", &self.logfile, i);
            self.create_log(Some(&logfile), false);
        }
    }

    // gzip internal logfile
    pub fn gzip(&self) {
        let output = Command::new("gzip")
            .args(&[&self.logfile])
            .output()
            .expect(&format!("unable to gzip dummy logfile {}", self.logfile));
        trace!("gzip, output={:?}", output);
    }

    // simulate file rotation
    pub fn rotate(&self) {
        // if .gz is already existing, delete it first
        let _ = std::fs::remove_file(&self.logfile_gzip);
        trace!("rotating file {}", &self.logfile);

        let output = Command::new("gzip")
            .args(&[&self.logfile])
            .output()
            .expect("unable to gzip dummy logfile");

        trace!("gzip file={}, {:?}", &self.logfile, output);
        if output.status.code().unwrap() != 0 {
            error!("error compressing logfile {}", &self.logfile);
            println!("error compressing logfile {}", &self.logfile);
        }

        // regenerate a new logfile
        self.create_log(None, false);
        trace!("created new file {}", &self.logfile);

        // wait a little before calling
        let timeout = std::time::Duration::from_millis(100);
        std::thread::sleep(timeout);
    }

    // call CLF executable with optional arguments
    pub fn run(&mut self, opts: &Options, optargs: &[&str]) -> (i32, String) {
        let clf = &opts.clf;

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

        // wait a little before calling
        let timeout = std::time::Duration::from_millis(1000);
        std::thread::sleep(timeout);

        trace!("{:?}", output);
        let s = String::from_utf8_lossy(&output.stdout);

        // load json as hashmap
        self.json = self.json(&self.snap_file).unwrap_or(HashMap::new());

        (output.status.code().unwrap(), s.to_string())
    }

    // call CLF executable with arguments
    pub fn exec(&self, opts: &Options, args: &[&str]) -> i32 {
        let clf = &opts.clf;

        let output = Command::new(clf)
            .args(args)
            .output()
            .expect("unable to start clf");

        trace!("{:?}", output);
        output.status.code().unwrap()
    }

    // extract JSON data
    fn json(&self, name: &str) -> Option<HashMap<String, String>> {
        // if JSON is not existing, give up
        if !std::path::Path::new(name).exists() {
            return None;
        }

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

        Some(hmap)
    }
}

// manage creation or growth of a fake logfile
//const FAKE_LOGFILE_UTF8: &'static str = "./tests/integration/logfiles/generated_utf8.log";

pub struct FakeLogfile;

impl FakeLogfile {
    pub fn create(logfile: &str, append: bool) {
        trace!("creating logfile {}", logfile);

        // open file in write or append mode
        let log = if append {
            OpenOptions::new()
                .append(true)
                .open(logfile)
                .expect(&format!("unable to create fake logfile {}", logfile))
        } else {
            File::create(logfile).expect(&format!("unable to create fake logfile {}", logfile))
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
    pub fn create_utf8(logfile: &str) {
        // open file in write or append mode
        let log = File::create(&logfile).expect("unable to create fake logfile");
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
}

// This is made for testing using a UDS
use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct JSONStream {
    pub args: Vec<String>,
    pub global: Option<HashMap<String, String>>,
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

/// Prepare test execution
pub struct TestScenario;

impl TestScenario {
    pub fn prepare() {
        // create tmp directory if not present
        if !std::path::Path::new("./tests/integration/tmp").exists() {
            std::fs::create_dir("./tests/integration/tmp").expect("unable to create tmp dir");
        }
        // create logfiles directory if not present
        if !std::path::Path::new("./tests/integration/logfiles").exists() {
            std::fs::create_dir("./tests/integration/logfiles")
                .expect("unable to create logfiles dir");
        }

        // create logger
        TestScenario::init_log();
    }

    /// Create new logger and optionally delete logfile is bigger than cli value
    fn init_log() {
        // initialize logger
        WriteLogger::init(
            LevelFilter::Trace,
            simplelog::ConfigBuilder::new()
                .set_time_format("%Y-%b-%d %H:%M:%S.%f".to_string())
                .build(),
            OpenOptions::new()
                .write(true)
                .create(true)
                .open("integration_test.log")
                .unwrap(),
        )
        .expect("unable to create integration_test.log");

        // useful traces
        trace!("created log");
    }
}
