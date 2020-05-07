//! Holds the main configuration data, loaded from a YAML file.
use std::convert::From;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use log::{debug, info};
use regex::Captures;
use serde::{Deserialize, Serialize};
use wait_timeout::ChildExt;

use crate::error::*;
use crate::pattern::{Pattern, PatternSet};

#[cfg(target_os = "linux")]
const SEPARATOR: char = ':';

#[cfg(target_os = "windows")]
const SEPARATOR: char = ';';

/// A list of paths, where the script which is potentially called, are scanned the locate
/// this script.
#[derive(Debug, Deserialize)]
#[serde(from = "String")]
pub struct PathList(Vec<PathBuf>);

/// Just converts a list of paths separated by either ':' or ';' depending on the platform
/// to a vector of `PathBuf`.
impl From<String> for PathList {
    fn from(list: String) -> Self {
        PathList(list.split(SEPARATOR).map(|p| PathBuf::from(p)).collect())
    }
}

/// A helper structure to represent a script or command to be run on each match.
#[derive(Debug, Deserialize)]
pub struct Script {
    // name of the script to spawn without path
    pub path: PathBuf,

    // list of its optional paths
    //pub pathlist: Option<String>,

    // list of its optional arguments
    pub args: Option<Vec<String>>,

    // timeout in seconds
    #[serde(default)]
    pub timeout: u64,
}

impl Script {
    /// Returns the canonical, absolute form of the path with all intermediate
    /// components normalized and symbolic links resolved.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use clf::config::Script;
    ///
    /// let script = Script {
    ///     path: PathBuf::from("gzip"),
    ///     args: None,
    ///     timeout: 0
    /// };
    /// let path_list = "/usr:/dev:/usr/lib:/usr/bin:/bin";
    /// let pathbuf_list: Vec<_> = path_list
    ///     .split(":")
    ///     .map(|p| PathBuf::from(p))
    ///     .collect();
    /// assert_eq!(script.canonicalize(&pathbuf_list).unwrap(), PathBuf::from("/bin/gzip"));
    /// ```
    pub fn canonicalize(&self, pathlist: &[PathBuf]) -> Result<PathBuf, Error> {
        // if script is relative, find the path where is it located
        if self.path.is_relative() {
            // at least, if script is relative, we need to find it in at least one
            // path from pathlist. So, in this case, pathlist must exist
            // if pathlist.is_none() {
            //     return Err(Error::new(ErrorKind::NotFound, "script not found"));
            // }

            // // path separator is OS dependant
            // let path_sep = if cfg!(unix) {
            //     ":"
            // } else if cfg!(windows) {
            //     ";"
            // } else {
            //     unimplemented!("OS is not supported")
            // };

            // split the string to get individual paths
            //let path_vec: Vec<_> = pathlist.as_ref().unwrap().split(path_sep).collect();

            // find the first one where script is located and build the whole path + script name
            for path in pathlist {
                let mut full_path = PathBuf::new();
                full_path.push(path);
                full_path.push(&self.path);

                if full_path.is_file() {
                    return full_path.canonicalize();
                }
            }
        }

        // just check if script exists
        if self.path.is_file() {
            self.path.canonicalize()
        } else {
            Err(Error::new(ErrorKind::NotFound, "script not found"))
        }
    }

    /// Replace, for each argument, the capture groups values.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use regex::{Captures, Regex};
    /// use clf::config::Script;
    ///
    /// let script = Script {
    ///     path: PathBuf::from("gzip"),
    ///     args: Some(vec!["address=$hw".to_string(), "id=$id".to_string(), "ok".to_string()]),
    ///     timeout: 0
    /// };
    /// let line = ">>> wlan0: authenticate with FF:FA:FB:FC:FD:FE";
    /// let re = Regex::new(r"(?P<id>\w+): authenticate with (?P<hw>[A-Z:]+)").unwrap();
    /// let caps = re.captures(line).unwrap();
    /// let replaced = script.replace_args(caps);
    /// assert!(replaced.is_some());
    /// assert_eq!(replaced.unwrap(), &["address=FF:FA:FB:FC:FD:FE", "id=wlan0", "ok"]);
    /// ```
    pub fn replace_args<'t>(&self, caps: Captures<'t>) -> Option<Vec<String>> {
        // if we got captures, for each argument, replace by capture groups
        if caps.len() > 1 && self.args.is_some() {
            // this vector will receive new arguments
            let mut new_args = Vec::new();
            let mut buffer = String::with_capacity(256);

            // replace capture groups for each arg
            for arg in self.args.as_ref().unwrap() {
                // replace strings like $name by capture groups values
                caps.expand(arg, &mut buffer);

                // add replaced string
                new_args.push(buffer.clone());

                // reset buffer
                buffer.clear();
            }
            return Some(new_args);
        }
        None
    }

    /// Spawns the script, and wait at most `timeout` seconds for the job to finish. Spawns
    /// a new thread, and if timeout, the thread will end gracefully.
    pub fn spawn(&self, duration: u64) -> thread::JoinHandle<()> {
        // let cmd = self.name.clone();
        //let args: Vec<&str> = self.args.as_ref().unwrap().iter().map(|s| &**s).collect();

        let mut cmd = Command::new(&self.path);
        let mut child = cmd
            .args(&self.args.as_ref().unwrap()[..])
            .spawn()
            .expect("failed to execute");

        let handle = thread::spawn(move || {
            // let mut cmd = Command::new(cmd);
            // let mut child = cmd.args(args).spawn().expect("failed to execute");
            let one_sec = std::time::Duration::from_secs(duration);
            let status_code = match child.wait_timeout(one_sec).unwrap() {
                Some(status) => status.code(),
                None => {
                    // child hasn't exited yet
                    child.kill().unwrap();
                    child.wait().unwrap().code()
                }
            };
        });
        handle
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
/// A list of options which are specific to a search.
pub struct SearchOptions {
    /// if `true`, the defined script will be run a first match
    pub runscript: bool,

    /// if `true`, the matching line will be saved in an output file
    pub keep_output: bool,

    /// if `true`, the logfile will be search from the beginning, regardless of any saved offset
    pub rewind: bool,
}

/// A `enum` matching either a logfile name if only a single logfile is defined, or a list
/// of logfile names is case of command is given. This command is expected to return to the
/// the standard output the list of files to check. One of the enum variant is loaded from
/// the YAML configuration file.
#[derive(Debug, Deserialize)]
pub enum LogSource {
    #[serde(rename = "logfile")]
    LogFile(String),

    #[serde(rename = "loglist")]
    LogList(String),
}

impl LogSource {
    pub fn get_files(&self) -> Result<Vec<String>, AppError> {
        match self {
            LogSource::LogFile(s) => Ok(vec![s.clone()]),
            LogSource::LogList(cmd) => {
                let filelist = Command::new(cmd).output()?;
                let output = String::from_utf8_lossy(&filelist.stdout);
                let v: Vec<_> = output.split('\n').map(|x| x.to_string()).collect();
                Ok(v)
            }
        }
    }
}

/// This is the core structure which handles data used to search into the logfile. These are
/// gathered and refered to a tag name.
#[derive(Debug, Deserialize)]
pub struct Tag {
    /// a name to identify the name
    pub name: String,

    /// a list of options specific to this search. As such options are optional, add a default serde
    /// directive
    #[serde(default)]
    pub options: SearchOptions,

    /// a script details like path, name, parameters, delay etc to be possibly run for a match
    pub script: Option<Script>,

    /// patterns to be checked against
    pub patterns: PatternSet,
}

impl Tag {
    /// Returns the capture groups corresponding to the leftmost-first match in text.
    pub fn captures<'t>(&self, line: &'t str) -> Option<Captures<'t>> {
        // if we find a match, replace
        // if let Some(caps) = self.patterns.captures(&line) {
        //     debug!("line match: {:?}", caps);
        //     break;
        // }

        // None
        self.patterns.captures(&line)
    }

    //pub fn try_match(&self, line: &str) {}
}

/// This is the structure mapping exactly data coming from the configuration YAML file.
#[derive(Debug, Deserialize)]
pub struct Search {
    /// the logfile name to check
    pub logfile: PathBuf,

    /// a unique identifier for this search
    pub tags: Vec<Tag>,
}

#[derive(Debug, Deserialize)]
/// A list of global options, which apply globally for all searches.
pub struct Global {
    /// A list of paths, separated by either ':' for unix, or ';' for windows. This is
    /// where the script, if any, will be searched for.
    pathlist: Option<PathList>,

    /// A directory where matches lines will be stored.
    #[serde(default = "std::env::temp_dir")]
    outputdir: PathBuf,

    /// A directory where the snapshot file is kept.
    #[serde(default = "std::env::temp_dir")]
    snapshotdir: PathBuf,
}

/// The main search configuration used to search patterns in a logfile. This is loaded from
/// the YAML file found in the command line argument. This configuration can include a list
/// of logfiles to lookup and for each logfile, a list of regexes to match.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub global: Global,
    pub searches: Vec<Search>,
}

impl Config {
    /// Loads a YAML configuration string as a `Config` struct.
    pub fn from_str(s: &str) -> Result<Config, AppError> {
        // load YAML data
        let yaml = serde_yaml::from_str(s)?;
        Ok(yaml)
    }

    /// Loads a YAML configuration file as a `Config` struct.
    pub fn from_file<P: AsRef<Path>>(file_name: P) -> Result<Config, AppError> {
        // open YAML file
        let file = File::open(file_name)?;

        // load YAML data
        let yaml = serde_yaml::from_reader(file)?;
        Ok(yaml)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, Script};
    //use std::path::PathBuf;

    fn get_toml() -> String {
        include_str!("config.yml").to_string()
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_load() {
        let toml = get_toml();
        let config = Config::from_str(&toml).unwrap();

        // test global struct
        assert_eq!(config.global.pathlist.unwrap().0.len(), 6);
        //assert_eq!(config.global.snapshotdir, PathBuf::from("/tmp"));

        assert_eq!(config.searches.len(), 2);
    }

    #[test]
    fn test_canonicalize() {
        use std::io::ErrorKind;
        use std::path::PathBuf;

        let script = Script {
            path: PathBuf::from("foo"),
            args: None,
            timeout: 0,
        };
        let path_list = "/usr:/dev:/usr/lib:/usr/bin:/bin";
        let pathbuf_list: Vec<_> = path_list.split(":").map(|p| PathBuf::from(p)).collect();
        assert_eq!(
            script.canonicalize(&pathbuf_list).unwrap_err().kind(),
            ErrorKind::NotFound
        );
    }
}
